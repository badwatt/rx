#![expect(
    clippy::unreadable_literal,
    reason = "colour literals read best as #RRGGBB"
)]

//! The rx desktop surface.
//!
//! Operate mode: the visitor completes a task.  The composition is a calm
//! instrument panel — pipeline on the left, state and live output in the
//! middle — with exactly one saturated element, the aurora accent that marks
//! progress.  Every colour, weight and gap comes from [`crate::theme`].

use std::collections::VecDeque;
use std::path::PathBuf;
use std::sync::mpsc::{Receiver, TryRecvError};
use std::time::{Duration, Instant};

use gpui::prelude::*;
use gpui::{Context, Div, FocusHandle, IntoElement, KeyDownEvent, Render, Window, div, px, rgb};

use crate::pipeline::{self, LineKind, Msg, RunConfig, Step};
use crate::theme as t;

/// The CLI version, single-sourced from the repository-root `VERSION` file.
const VERSION: &str = include_str!("../../VERSION");

/// Lines newer than this are dropped; the console only ever shows the tail.
const LOG_CAP: usize = 4000;
/// Visible rows in the console well (height / line height).
const CONSOLE_ROWS: usize = 14;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Phase {
    Idle,
    Confirm,
    Running,
    Done,
    Failed,
    Auth,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Status {
    Pending,
    Running,
    Ok,
    Failed,
}

struct Row {
    step: Step,
    status: Status,
    took: Option<Duration>,
}

struct Line {
    kind: LineKind,
    text: String,
}

pub(crate) struct RxApp {
    cfg: RunConfig,
    phase: Phase,
    rows: Vec<Row>,
    log: VecDeque<Line>,
    started: Option<Instant>,
    elapsed: Option<Duration>,
    rx_binary: Option<PathBuf>,
    root_ready: bool,
    /// `(PRETTY_NAME, generation)` of the running system, when on NixOS.
    system: (Option<String>, Option<String>),
    /// Newest system generations, current first.
    generations: Vec<Generation>,
    /// Password being typed for `sudo -v`; cleared as soon as it is used.
    auth: String,
    auth_error: bool,
    auth_busy: bool,
    /// Stashed password for `sudo`'s askpass helper, removed after the run.
    askpass: Option<std::path::PathBuf>,
    focus: FocusHandle,
    /// Why the last run could not start, or why a step failed.
    error: Option<String>,
    feed: Option<Receiver<Msg>>,
}

impl RxApp {
    pub(crate) fn new(cx: &mut Context<Self>) -> Self {
        let cfg = RunConfig::default();
        let rows = cfg
            .plan()
            .into_iter()
            .map(|step| Row {
                step,
                status: Status::Pending,
                took: None,
            })
            .collect();
        Self {
            cfg,
            phase: Phase::Idle,
            rows,
            log: VecDeque::new(),
            started: None,
            elapsed: None,
            rx_binary: pipeline::find_rx(),
            root_ready: pipeline::root_is_ready(),
            system: system_identity(),
            generations: recent_generations(4),
            auth: String::new(),
            auth_error: false,
            auth_busy: false,
            askpass: None,
            focus: cx.focus_handle(),
            error: None,
            feed: None,
        }
    }

    /// Drain supervisor messages.  Called from the foreground poller.
    pub(crate) fn pump(&mut self, cx: &mut Context<Self>) {
        let mut drained = Vec::new();
        let mut disconnected = false;
        if let Some(feed) = &self.feed {
            loop {
                match feed.try_recv() {
                    Ok(msg) => drained.push(msg),
                    Err(TryRecvError::Empty) => break,
                    Err(TryRecvError::Disconnected) => {
                        disconnected = true;
                        break;
                    }
                }
            }
        }
        let changed = !drained.is_empty() || disconnected;
        for msg in drained {
            self.apply(msg);
        }
        if disconnected {
            self.feed = None;
        }
        // A running pipeline owns the status line, so keep the clock ticking.
        if changed || self.phase == Phase::Running {
            cx.notify();
        }
    }

    fn apply(&mut self, msg: Msg) {
        match msg {
            Msg::Started(step) => {
                if let Some(row) = self.row_mut(step) {
                    row.status = Status::Running;
                }
                self.push(LineKind::Section, section_title(step));
            }
            Msg::Line(kind, text) => self.push(kind, text),
            Msg::Finished(step, ok, took) => {
                if let Some(row) = self.row_mut(step) {
                    row.status = if ok { Status::Ok } else { Status::Failed };
                    row.took = Some(took);
                }
                if !ok {
                    self.phase = Phase::Failed;
                }
            }
            Msg::Done(ok, took) => {
                self.elapsed = Some(took);
                self.phase = if ok { Phase::Done } else { Phase::Failed };
                for row in &mut self.rows {
                    if row.status == Status::Running {
                        row.status = Status::Failed;
                    }
                }
            }
        }
    }

    fn row_mut(&mut self, step: Step) -> Option<&mut Row> {
        self.rows.iter_mut().find(|row| row.step == step)
    }

    fn push(&mut self, kind: LineKind, text: String) {
        if self.log.len() == LOG_CAP {
            self.log.pop_front();
        }
        self.log.push_back(Line { kind, text });
    }

    /// The single, deliberate confirmation before anything touches the system.
    fn request_run(&mut self, cx: &mut Context<Self>) {
        self.error = None;
        self.phase = Phase::Confirm;
        cx.notify();
    }

    fn begin(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(binary) = self.rx_binary.clone() else {
            self.phase = Phase::Failed;
            self.error = Some(String::from(
                "rx was not found on PATH; install it or run the CLI directly",
            ));
            cx.notify();
            return;
        };
        if self.cfg.needs_root() && !self.cfg.dry_run && !pipeline::root_is_ready() {
            self.phase = Phase::Auth;
            window.focus(&self.focus);
            cx.notify();
            return;
        }
        self.rows = self
            .cfg
            .plan()
            .into_iter()
            .map(|step| Row {
                step,
                status: Status::Pending,
                took: None,
            })
            .collect();
        self.log.clear();
        self.error = None;
        self.elapsed = None;
        self.started = Some(Instant::now());
        self.phase = Phase::Running;
        let (tx, rx) = std::sync::mpsc::channel();
        self.feed = Some(rx);
        pipeline::run_all(self.cfg.clone(), binary, self.askpass.take(), tx);
        cx.notify();
    }

    /// Validate the typed password through `sudo -v`, then start.
    fn submit_password(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.auth.is_empty() || self.auth_busy {
            return;
        }
        self.auth_busy = true;
        let ok = pipeline::authenticate(&self.auth);
        // Hand the secret to sudo's askpass helper before dropping it: a
        // rebuild can outlive the timestamp and there is no terminal to
        // answer a second prompt.
        let stashed = ok.then(|| pipeline::stash_secret(&self.auth));
        // Never keep the secret around once sudo has consumed it.
        self.auth.clear();
        self.auth_busy = false;
        if ok {
            self.auth_error = false;
            self.root_ready = true;
            self.askpass = stashed.flatten();
            self.begin(window, cx);
        } else {
            self.auth_error = true;
            window.focus(&self.focus);
            cx.notify();
        }
    }

    fn on_key(&mut self, event: &KeyDownEvent, window: &mut Window, cx: &mut Context<Self>) {
        match event.keystroke.key.as_ref() {
            "enter" => self.submit_password(window, cx),
            "escape" => {
                self.auth.clear();
                self.phase = Phase::Idle;
                cx.notify();
            }
            "backspace" => {
                self.auth.pop();
                self.auth_error = false;
                cx.notify();
            }
            _ => {
                if let Some(text) = event.keystroke.key_char.as_ref()
                    && !text.chars().any(char::is_control)
                    && self.auth.len() + text.len() <= 64
                {
                    self.auth.push_str(text);
                    self.auth_error = false;
                    cx.notify();
                }
            }
        }
    }

    /// Masked password field plus the unlock action.
    fn auth_row(&self, cx: &mut Context<Self>) -> Div {
        let border = if self.auth_error {
            t::RED
        } else if self.auth_busy {
            t::AMBER
        } else {
            t::LINE
        };
        div()
            .mt(px(12.))
            .flex()
            .items_center()
            .gap(px(10.))
            .child(
                div()
                    .id("password")
                    .track_focus(&self.focus)
                    .w(px(240.))
                    .h(px(38.))
                    .px(px(12.))
                    .rounded(px(9.))
                    .border_1()
                    .border_color(rgb(border))
                    .bg(rgb(t::INK_SUNKEN))
                    .flex()
                    .items_center()
                    .font_family(t::MONO)
                    .text_size(px(13.))
                    .text_color(rgb(t::TEXT))
                    .child(if self.auth.is_empty() {
                        div().text_color(rgb(t::FAINT)).child("sudo password")
                    } else {
                        div().child("•".repeat(self.auth.chars().count()))
                    })
                    .on_key_down(cx.listener(|this, event: &KeyDownEvent, window, cx| {
                        this.on_key(event, window, cx);
                    })),
            )
            .child(
                div()
                    .id("unlock")
                    .h(px(38.))
                    .px(px(18.))
                    .rounded(px(9.))
                    .bg(t::aurora())
                    .flex()
                    .items_center()
                    .text_size(px(12.5))
                    .font_weight(t::SEMIBOLD)
                    .text_color(rgb(0x07101C))
                    .cursor_pointer()
                    .hover(|style| style.opacity(0.92))
                    .on_click(cx.listener(|this, _, window, cx| this.submit_password(window, cx)))
                    .child(if self.auth_busy {
                        "Checking…"
                    } else {
                        "Unlock"
                    }),
            )
            .child(
                div()
                    .font_family(t::MONO)
                    .text_size(px(10.))
                    .text_color(rgb(if self.auth_error { t::RED } else { t::FAINT }))
                    .child(if self.auth_error {
                        "wrong password"
                    } else {
                        "used once, never stored"
                    }),
            )
    }

    fn reset(&mut self, cx: &mut Context<Self>) {
        if let Some(path) = self.askpass.take() {
            let _ = std::fs::remove_file(&path);
        }
        self.phase = Phase::Idle;
        self.started = None;
        self.elapsed = None;
        self.error = None;
        self.rows = self
            .cfg
            .plan()
            .into_iter()
            .map(|step| Row {
                step,
                status: Status::Pending,
                took: None,
            })
            .collect();
        cx.notify();
    }

    fn toggle_update(&mut self, cx: &mut Context<Self>) {
        self.cfg.update = !self.cfg.update;
        self.reset(cx);
    }

    fn toggle_diff(&mut self, cx: &mut Context<Self>) {
        self.cfg.diff = !self.cfg.diff;
        self.reset(cx);
    }

    fn toggle_dry(&mut self, cx: &mut Context<Self>) {
        self.cfg.dry_run = !self.cfg.dry_run;
        cx.notify();
    }

    fn done_count(&self) -> usize {
        self.rows
            .iter()
            .filter(|row| row.status == Status::Ok)
            .count()
    }

    fn current(&self) -> Option<&Row> {
        self.rows.iter().find(|row| row.status == Status::Running)
    }

    fn live_elapsed(&self) -> Option<Duration> {
        self.elapsed.or_else(|| self.started.map(|at| at.elapsed()))
    }

    // ---------------------------------------------------------------- chrome

    fn top_bar(&self) -> Div {
        let mut chips = vec![chip(self.cfg.hostname.clone(), t::MUTED)];
        chips.push(chip(format!("keep {}", self.cfg.keep), t::FAINT));
        if self.cfg.dry_run {
            chips.push(chip(String::from("DRY RUN"), t::AMBER));
        }

        div()
            .flex_shrink_0()
            .bg(rgb(t::INK))
            .child(
                div()
                    .h(px(58.))
                    .flex()
                    .items_center()
                    .justify_between()
                    .px(px(24.))
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap(px(11.))
                            .child(div().size(px(14.)).rounded(px(4.)).bg(t::aurora()))
                            .child(
                                div()
                                    .text_size(px(13.))
                                    .font_weight(t::SEMIBOLD)
                                    .text_color(rgb(t::TEXT))
                                    .child("RX"),
                            )
                            .child(div().w(px(1.)).h(px(14.)).bg(rgb(t::LINE)))
                            .child(
                                div()
                                    .text_size(px(11.))
                                    .text_color(rgb(t::FAINT))
                                    .child("system update"),
                            ),
                    )
                    .child(div().flex().items_center().gap(px(7.)).children(chips)),
            )
            .child(div().h(px(1.)).w_full().bg(rgb(t::LINE_SOFT)))
    }

    fn rail(&self) -> Div {
        div()
            .flex_shrink_0()
            .flex()
            .child(
                div()
                    .w(px(296.))
                    .flex()
                    .flex_col()
                    .px(px(18.))
                    .py(px(24.))
                    .gap(px(3.))
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .justify_between()
                            .mb(px(14.))
                            .child(eyebrow("pipeline", t::FAINT))
                            .child(
                                div()
                                    .font_family(t::MONO)
                                    .text_size(px(9.))
                                    .text_color(rgb(t::FAINT))
                                    .child(format!("{:02}", self.rows.len())),
                            ),
                    )
                    .children(
                        self.rows
                            .iter()
                            .enumerate()
                            .map(|(index, row)| self.step_row(index, row)),
                    )
                    .child(div().flex_1())
                    .children(self.generations_block())
                    .children(self.identity_block()),
            )
            .child(div().w(px(1.)).h_full().bg(rgb(t::LINE_SOFT)))
    }

    /// The last few system generations: proof the machine has a history.
    fn generations_block(&self) -> Vec<Div> {
        if self.generations.is_empty() {
            return Vec::new();
        }
        let mut rows = vec![
            div()
                .mt(px(14.))
                .mb(px(6.))
                .child(eyebrow("generations", t::FAINT)),
        ];
        for generation in &self.generations {
            rows.push(
                div()
                    .h(px(20.))
                    .flex()
                    .items_center()
                    .gap(px(8.))
                    .child(div().size(px(4.)).rounded_full().bg(t::dim(
                        if generation.current { t::BLUE } else { t::LINE },
                        1.,
                    )))
                    .child(
                        div()
                            .font_family(t::MONO)
                            .text_size(px(10.))
                            .text_color(rgb(if generation.current {
                                t::TEXT
                            } else {
                                t::MUTED
                            }))
                            .child(format!("{}", generation.number)),
                    )
                    .child(
                        div()
                            .font_family(t::MONO)
                            .text_size(px(9.5))
                            .text_color(rgb(t::FAINT))
                            .child(generation.age.clone()),
                    ),
            );
        }
        rows
    }

    /// What machine are we looking at?  Quiet, factual, at the foot of the rail.
    fn identity_block(&self) -> Vec<Div> {
        let (name, _) = &self.system;
        let Some(name) = name else {
            return Vec::new();
        };
        let mut rows = vec![
            div()
                .mt(px(10.))
                .mb(px(10.))
                .h(px(1.))
                .w_full()
                .bg(rgb(t::LINE_SOFT)),
        ];
        rows.push(
            div()
                .font_family(t::MONO)
                .text_size(px(9.5))
                .text_color(rgb(t::FAINT))
                .whitespace_nowrap()
                .text_ellipsis()
                .child(name.clone()),
        );
        rows
    }

    fn step_row(&self, index: usize, row: &Row) -> Div {
        let active = row.status == Status::Running;
        let (dot, status_text, status_color) = match row.status {
            Status::Pending => (t::LINE, row.step.short_command(), t::FAINT),
            Status::Running => (t::VIOLET, String::from("running"), t::VIOLET),
            Status::Ok => (
                t::GREEN,
                row.took.map(pipeline::clock).unwrap_or_default(),
                t::FAINT,
            ),
            Status::Failed => (t::RED, String::from("failed"), t::RED),
        };
        let name_color = match row.status {
            Status::Pending => t::MUTED,
            _ => t::TEXT,
        };
        let upper = if index == 0 {
            t::none()
        } else if self.rows[index - 1].status == Status::Ok {
            t::dim(t::BLUE, 1.)
        } else {
            t::dim(t::LINE, 1.)
        };
        let lower = if row.status == Status::Ok {
            t::dim(t::BLUE, 1.)
        } else if row.status == Status::Running {
            t::dim(t::VIOLET, 1.)
        } else {
            t::dim(t::LINE, 1.)
        };

        div()
            .h(px(52.))
            .px(px(10.))
            .rounded(px(9.))
            .flex()
            .items_center()
            .gap(px(12.))
            .bg(if active {
                t::dim(t::BLUE, 0.07)
            } else {
                t::none()
            })
            .border_1()
            .border_color(if active {
                t::dim(t::BLUE, 0.22)
            } else {
                t::none()
            })
            .child(
                div()
                    .h_full()
                    .w(px(12.))
                    .flex_shrink_0()
                    .flex()
                    .flex_col()
                    .items_center()
                    .child(div().w(px(2.)).h(px(16.)).bg(upper))
                    .child(
                        div()
                            .size(px(7.))
                            .rounded_full()
                            .bg(rgb(dot))
                            .border_1()
                            .border_color(t::dim(t::INK, 0.9)),
                    )
                    .child(div().w(px(2.)).flex_1().bg(lower)),
            )
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap(px(2.))
                    .flex_1()
                    .child(
                        div()
                            .text_size(px(12.5))
                            .font_weight(if active || row.status == Status::Ok {
                                t::MEDIUM
                            } else {
                                t::REGULAR
                            })
                            .text_color(rgb(name_color))
                            .child(row.step.label()),
                    )
                    .child(
                        div()
                            .font_family(t::MONO)
                            .text_size(px(9.5))
                            .text_color(rgb(status_color))
                            .whitespace_nowrap()
                            .text_ellipsis()
                            .child(status_text),
                    ),
            )
            .child(
                div()
                    .font_family(t::MONO)
                    .text_size(px(9.5))
                    .text_color(rgb(t::FAINT))
                    .child(format!("{:02}", index + 1)),
            )
    }

    fn workspace(&self, cx: &mut Context<Self>) -> Div {
        if self.phase == Phase::Confirm {
            return self.confirm_panel(cx);
        }
        div()
            .flex_1()
            .flex()
            .flex_col()
            .gap(px(22.))
            .p(px(28.))
            .child(self.hero(cx))
            .child(self.console())
    }

    fn hero(&self, cx: &mut Context<Self>) -> Div {
        let eyebrow_text = match self.phase {
            Phase::Running => match self.current() {
                Some(row) => format!(
                    "step {:02} of {:02}",
                    self.rows
                        .iter()
                        .position(|r| r.step == row.step)
                        .map_or(0, |index| index + 1),
                    self.rows.len()
                ),
                None => String::from("finishing"),
            },
            Phase::Done => String::from("complete"),
            Phase::Failed => String::from("halted"),
            Phase::Auth => String::from("root required"),
            _ => String::from("ready"),
        };

        let headline = match self.phase {
            Phase::Running => match self.current() {
                Some(row) => row.step.label(),
                None => "Finishing up",
            },
            Phase::Done => "System updated",
            Phase::Failed => match self.rows.iter().find(|row| row.status == Status::Failed) {
                Some(row) => row.step.failure_headline(),
                None => "Run stopped",
            },
            Phase::Auth => "Root access needed",
            _ => "Update your system",
        };

        let subline = match self.phase {
            Phase::Running => match self.current() {
                Some(row) => self.cfg.command(row.step),
                None => String::from("no step is running"),
            },
            Phase::Done => format!(
                "{:02} steps finished in {}",
                self.done_count(),
                self.live_elapsed().map(pipeline::clock).unwrap_or_default()
            ),
            Phase::Failed => self.error.clone().unwrap_or_else(|| {
                String::from(
                    "the pipeline stopped before cleanup; your running generation is untouched",
                )
            }),
            Phase::Auth => {
                String::from("the rebuild runs as root — enter your password, or press Esc")
            }
            _ => format!(
                "{} · {} · stages, rebuilds, then prunes old generations",
                self.cfg.repo, self.cfg.hostname
            ),
        };

        div()
            .flex()
            .flex_col()
            .gap(px(10.))
            .child(eyebrow(&eyebrow_text, self.phase_color()))
            .child(
                div()
                    .text_size(px(30.))
                    .font_weight(t::SEMIBOLD)
                    .text_color(rgb(t::TEXT))
                    .child(headline),
            )
            .child(
                div()
                    .text_size(px(12.5))
                    .text_color(rgb(t::MUTED))
                    .child(subline),
            )
            .child(
                div()
                    .mt(px(12.))
                    .flex()
                    .items_end()
                    .justify_between()
                    .child(if self.phase == Phase::Auth {
                        self.auth_row(cx).into_any_element()
                    } else {
                        self.primary_action(cx).into_any_element()
                    })
                    .child(if self.phase == Phase::Auth {
                        div().into_any_element()
                    } else {
                        self.ghost_toggles(cx).into_any_element()
                    }),
            )
    }

    fn phase_color(&self) -> u32 {
        match self.phase {
            Phase::Running => t::VIOLET,
            Phase::Done => t::GREEN,
            Phase::Failed => t::RED,
            Phase::Auth => t::AMBER,
            _ => t::BLUE,
        }
    }

    fn primary_action(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let (label, enabled) = match self.phase {
            Phase::Running => (String::from("Working"), false),
            Phase::Done => (String::from("Run again"), true),
            Phase::Failed => (String::from("Try again"), true),
            Phase::Auth => (String::from("Unlock"), true),
            _ => (String::from("Run update"), true),
        };
        let button = div()
            .id("primary")
            .h(px(38.))
            .px(px(18.))
            .rounded(px(9.))
            .flex()
            .items_center()
            .gap(px(8.))
            .bg(if enabled {
                t::aurora()
            } else {
                t::dim(t::VIOLET, 0.16).into()
            })
            .text_size(px(12.5))
            .font_weight(t::SEMIBOLD)
            .text_color(rgb(if enabled { 0x07101C } else { t::MUTED }))
            .child(label);

        if !enabled {
            return button;
        }
        button
            .cursor_pointer()
            .hover(|style| style.opacity(0.92))
            .active(|style| style.opacity(0.82))
            .on_click(cx.listener(|this, _, window, cx| match this.phase {
                Phase::Auth => this.begin(window, cx),
                Phase::Done | Phase::Failed => this.reset(cx),
                _ => this.request_run(cx),
            }))
    }

    fn ghost_toggles(&self, cx: &mut Context<Self>) -> Div {
        let idle = matches!(self.phase, Phase::Idle);
        div()
            .flex()
            .flex_col()
            .items_end()
            .gap(px(6.))
            .child(eyebrow("options", t::FAINT))
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(8.))
                    .child(
                        toggle("update", self.cfg.update, idle, false)
                            .id("toggle-update")
                            .cursor_pointer()
                            .when(idle, |element| {
                                element
                                    .on_click(cx.listener(|this, _, _, cx| this.toggle_update(cx)))
                            }),
                    )
                    .child(
                        toggle("diff", self.cfg.diff, idle, false)
                            .id("toggle-diff")
                            .cursor_pointer()
                            .when(idle, |element| {
                                element.on_click(cx.listener(|this, _, _, cx| this.toggle_diff(cx)))
                            }),
                    )
                    .child(
                        toggle("dry run", self.cfg.dry_run, idle, true)
                            .id("toggle-dry")
                            .cursor_pointer()
                            .when(idle, |element| {
                                element.on_click(cx.listener(|this, _, _, cx| this.toggle_dry(cx)))
                            }),
                    ),
            )
    }

    fn console(&self) -> Div {
        let live = self.phase == Phase::Running;
        let mut body = div()
            .flex_1()
            .flex()
            .flex_col()
            .overflow_hidden()
            .px(px(16.))
            .py(px(14.))
            .gap(px(2.));

        if self.log.is_empty() {
            body = body.justify_start().children(self.empty_state());
        } else {
            let skip = self.log.len().saturating_sub(CONSOLE_ROWS);
            body = body
                .justify_end()
                .children(self.log.iter().skip(skip).map(line_row));
        }

        div()
            .flex_1()
            .flex()
            .flex_col()
            .rounded(px(13.))
            .border_1()
            .border_color(rgb(t::LINE))
            .bg(rgb(t::INK_SUNKEN))
            .overflow_hidden()
            .child(
                div()
                    .h(px(36.))
                    .flex_shrink_0()
                    .flex()
                    .items_center()
                    .justify_between()
                    .px(px(14.))
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap(px(8.))
                            .child(eyebrow("output", t::FAINT))
                            .child(div().size(px(5.)).rounded_full().bg(if live {
                                rgb(t::VIOLET)
                            } else {
                                rgb(t::FAINT)
                            })),
                    )
                    .child(
                        div()
                            .font_family(t::MONO)
                            .text_size(px(9.))
                            .text_color(rgb(t::FAINT))
                            .child(if live {
                                String::from("live")
                            } else if self.log.is_empty() {
                                String::from("waiting")
                            } else {
                                format!("{} lines", self.log.len())
                            }),
                    ),
            )
            .child(div().h(px(1.)).w_full().bg(rgb(t::LINE_SOFT)))
            .child(body)
    }

    /// What the console shows before the first line arrives: the exact plan.
    fn empty_state(&self) -> Vec<Div> {
        let mut rows: Vec<Div> = self
            .cfg
            .plan()
            .into_iter()
            .map(|step| plan_row(&self.cfg.command(step)))
            .collect();
        rows.push(
            div()
                .mt(px(18.))
                .mb(px(14.))
                .h(px(1.))
                .w_full()
                .bg(rgb(t::LINE_SOFT)),
        );
        let mut notes = vec![String::from("nothing runs until you confirm")];
        notes.push(if self.cfg.dry_run {
            String::from("dry run: rx prints each command and touches nothing")
        } else {
            String::from("sudo is requested once, right before the rebuild")
        });
        notes.push(String::from(
            "staging is additive: git add ., never a commit",
        ));
        notes.push(format!(
            "cleanup keeps the last {} generations",
            self.cfg.keep
        ));
        for note in notes {
            rows.push(note_row(note));
        }
        rows
    }

    /// One row per planned step, separator between them.
    fn confirm_rows(&self) -> Vec<Div> {
        let plan = self.cfg.plan();
        let mut rows = Vec::new();
        for (index, step) in plan.iter().enumerate() {
            rows.push(
                div()
                    .py(px(11.))
                    .flex()
                    .items_start()
                    .gap(px(14.))
                    .child(
                        div()
                            .w(px(18.))
                            .font_family(t::MONO)
                            .text_size(px(9.5))
                            .text_color(rgb(t::FAINT))
                            .child(format!("{:02}", index + 1)),
                    )
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .gap(px(3.))
                            .flex_1()
                            .child(
                                div()
                                    .text_size(px(12.5))
                                    .font_weight(t::MEDIUM)
                                    .text_color(rgb(t::TEXT))
                                    .child(step.label()),
                            )
                            .child(
                                div()
                                    .text_size(px(10.5))
                                    .text_color(rgb(t::FAINT))
                                    .child(step.detail()),
                            )
                            .child(
                                div()
                                    .mt(px(2.))
                                    .font_family(t::MONO)
                                    .text_size(px(10.5))
                                    .text_color(rgb(t::BLUE))
                                    .child(format!("$ {}", self.cfg.command(*step))),
                            ),
                    ),
            );
            if index + 1 < plan.len() {
                rows.push(div().h(px(1.)).w_full().bg(rgb(t::LINE_SOFT)));
            }
        }
        rows
    }

    fn confirm_panel(&self, cx: &mut Context<Self>) -> Div {
        let plan = self.cfg.plan();
        let rows = self.confirm_rows();

        div()
            .flex_1()
            .flex()
            .flex_col()
            .p(px(28.))
            .child(
                div()
                    .flex_1()
                    .flex()
                    .flex_col()
                    .justify_center()
                    .gap(px(6.))
                    .max_w(px(620.))
                    .child(eyebrow("confirm", t::BLUE))
                    .child(
                        div()
                            .mt(px(4.))
                            .text_size(px(26.))
                            .font_weight(t::SEMIBOLD)
                            .child("Run the update?"),
                    )
                    .child(
                        div()
                            .text_size(px(12.5))
                            .text_color(rgb(t::MUTED))
                            .child(format!(
                                "{} steps against {} with {}.{}",
                                plan.len(),
                                self.cfg.repo,
                                if self.cfg.dry_run {
                                    "dry run on — nothing is changed"
                                } else {
                                    "real changes"
                                },
                                if self.cfg.dry_run || !self.cfg.needs_root() {
                                    ""
                                } else {
                                    " sudo is asked once, before the rebuild."
                                }
                            )),
                    )
                    .child(
                        div()
                            .mt(px(14.))
                            .rounded(px(13.))
                            .border_1()
                            .border_color(rgb(t::LINE))
                            .bg(rgb(t::INK_RAISED))
                            .px(px(18.))
                            .children(rows),
                    ),
            )
            .child(Self::confirm_footer(cx))
    }

    /// Version note on the left, the two decisions on the right.
    fn confirm_footer(cx: &mut Context<Self>) -> Div {
        div()
            .mt(px(18.))
            .flex()
            .items_center()
            .justify_between()
            .max_w(px(620.))
            .child(
                div()
                    .font_family(t::MONO)
                    .text_size(px(10.))
                    .text_color(rgb(t::FAINT))
                    .child(format!("rx {}", VERSION.trim())),
            )
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(10.))
                    .child(
                        div()
                            .id("cancel")
                            .h(px(38.))
                            .px(px(16.))
                            .rounded(px(9.))
                            .border_1()
                            .border_color(rgb(t::LINE))
                            .flex()
                            .items_center()
                            .text_size(px(12.5))
                            .text_color(rgb(t::MUTED))
                            .cursor_pointer()
                            .hover(|style| style.bg(t::dim(t::TEXT, 0.04)))
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.phase = Phase::Idle;
                                cx.notify();
                            }))
                            .child("Cancel"),
                    )
                    .child(
                        div()
                            .id("confirm")
                            .h(px(38.))
                            .px(px(18.))
                            .rounded(px(9.))
                            .bg(t::aurora())
                            .flex()
                            .items_center()
                            .text_size(px(12.5))
                            .font_weight(t::SEMIBOLD)
                            .text_color(rgb(0x07101C))
                            .cursor_pointer()
                            .hover(|style| style.opacity(0.92))
                            .active(|style| style.opacity(0.82))
                            .on_click(cx.listener(|this, _, window, cx| this.begin(window, cx)))
                            .child("Start"),
                    ),
            )
    }

    fn status_bar(&self) -> Div {
        let state = match self.phase {
            Phase::Idle => "idle",
            Phase::Confirm => "confirm",
            Phase::Running => "running",
            Phase::Done => "done",
            Phase::Failed => "failed",
            Phase::Auth => "paused",
        };
        let elapsed = self.live_elapsed().map(pipeline::clock);
        let mut left = div().flex().items_center().gap(px(10.)).child(
            div()
                .font_family(t::MONO)
                .text_size(px(9.5))
                .text_color(rgb(self.phase_color()))
                .child(state.to_uppercase()),
        );
        if let Some(elapsed) = elapsed {
            left = left
                .child(div().size(px(3.)).rounded_full().bg(rgb(t::FAINT)))
                .child(
                    div()
                        .font_family(t::MONO)
                        .text_size(px(9.5))
                        .text_color(rgb(t::FAINT))
                        .child(elapsed),
                );
        }

        div()
            .flex_shrink_0()
            .bg(rgb(t::INK))
            .child(div().h(px(1.)).w_full().bg(rgb(t::LINE_SOFT)))
            .child(
                div()
                    .h(px(38.))
                    .px(px(24.))
                    .flex()
                    .items_center()
                    .justify_between()
                    .child(left)
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap(px(10.))
                            .child(
                                div()
                                    .font_family(t::MONO)
                                    .text_size(px(9.5))
                                    .text_color(rgb(t::FAINT))
                                    .child(format!("rx {}", VERSION.trim())),
                            )
                            .child(div().size(px(3.)).rounded_full().bg(rgb(t::LINE)))
                            .child(div().text_size(px(9.5)).text_color(rgb(t::FAINT)).child(
                                match &self.rx_binary {
                                    Some(_) => String::from("cli ready"),
                                    None => String::from("cli not found"),
                                },
                            )),
                    ),
            )
    }
}

impl Render for RxApp {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .size_full()
            .bg(rgb(t::INK))
            .font_family(t::SANS)
            .text_color(rgb(t::TEXT))
            .flex()
            .flex_col()
            .child(self.top_bar())
            .child(
                div()
                    .flex()
                    .flex_1()
                    .child(self.rail())
                    .child(self.workspace(cx)),
            )
            .child(self.status_bar())
    }
}

// ------------------------------------------------------------------ helpers

fn eyebrow(text: &str, color: u32) -> Div {
    div()
        .font_family(t::MONO)
        .text_size(px(9.))
        .text_color(rgb(color))
        .child(text.to_uppercase())
}

fn chip(text: String, color: u32) -> Div {
    div()
        .h(px(22.))
        .px(px(9.))
        .rounded_full()
        .border_1()
        .border_color(rgb(t::LINE))
        .bg(t::dim(t::TEXT, 0.025))
        .flex()
        .items_center()
        .font_family(t::MONO)
        .text_size(px(9.5))
        .text_color(rgb(color))
        .child(text)
}

fn toggle(label: &'static str, on: bool, enabled: bool, warn: bool) -> Div {
    div()
        .h(px(30.))
        .px(px(11.))
        .rounded(px(8.))
        .border_1()
        .border_color(if on {
            t::dim(if warn { t::AMBER } else { t::BLUE }, 0.35)
        } else {
            t::dim(t::LINE, 1.)
        })
        .bg(if on {
            t::dim(if warn { t::AMBER } else { t::BLUE }, 0.08)
        } else {
            t::none()
        })
        .when(enabled && !on, |element| {
            element.hover(|style| style.bg(t::dim(t::TEXT, 0.04)))
        })
        .flex()
        .items_center()
        .gap(px(7.))
        .font_family(t::MONO)
        .text_size(px(10.))
        .text_color(rgb(if on {
            if warn { t::AMBER } else { t::BLUE }
        } else if enabled {
            t::MUTED
        } else {
            t::FAINT
        }))
        .child(label)
}

fn line_row(line: &Line) -> Div {
    let color = match line.kind {
        LineKind::Section => t::BLUE,
        LineKind::Good => t::GREEN,
        LineKind::Warn => t::AMBER,
        LineKind::Bad => t::RED,
        LineKind::Plain => t::MUTED,
    };
    div()
        .font_family(t::MONO)
        .text_size(px(10.5))
        .line_height(px(16.5))
        .text_color(rgb(color))
        .whitespace_nowrap()
        .child(line.text.clone())
}

fn note_row(text: String) -> Div {
    div()
        .font_family(t::MONO)
        .text_size(px(10.))
        .line_height(px(18.))
        .text_color(rgb(t::FAINT))
        .child(text)
}

fn plan_row(command: &str) -> Div {
    div().flex().items_center().gap(px(10.)).child(
        div()
            .font_family(t::MONO)
            .text_size(px(10.5))
            .text_color(rgb(t::MUTED))
            .child(command.to_owned()),
    )
}

struct Generation {
    number: u32,
    current: bool,
    age: String,
}

/// The newest `limit` system generations, newest first.
fn recent_generations(limit: usize) -> Vec<Generation> {
    let current = std::fs::read_link("/nix/var/nix/profiles/system")
        .ok()
        .and_then(|path| path.file_name()?.to_str().map(str::to_owned))
        .and_then(|name| {
            name.strip_prefix("system-")?
                .strip_suffix("-link")
                .map(str::to_owned)
        });
    let Ok(entries) = std::fs::read_dir("/nix/var/nix/profiles") else {
        return Vec::new();
    };
    let mut found: Vec<Generation> = entries
        .flatten()
        .filter_map(|entry| {
            let name = entry.file_name().to_str().map(str::to_owned)?;
            let number: u32 = name
                .strip_prefix("system-")?
                .strip_suffix("-link")?
                .parse()
                .ok()?;
            let age = entry
                .metadata()
                .ok()
                .and_then(|meta| meta.modified().ok())
                .map(ago)
                .unwrap_or_default();
            Some(Generation {
                number,
                current: current.as_deref() == Some(number.to_string().as_str()),
                age,
            })
        })
        .collect();
    found.sort_by_key(|generation| std::cmp::Reverse(generation.number));
    found.truncate(limit);
    found
}

/// A short, human age for a generation.
fn ago(modified: std::time::SystemTime) -> String {
    let Ok(elapsed) = std::time::SystemTime::now().duration_since(modified) else {
        return String::from("just now");
    };
    let secs = elapsed.as_secs();
    if secs < 60 {
        String::from("just now")
    } else if secs < 3600 {
        format!("{}m ago", secs / 60)
    } else if secs < 86_400 {
        format!("{}h ago", secs / 3600)
    } else if secs < 86_400 * 14 {
        format!("{}d ago", secs / 86_400)
    } else {
        format!("{}w ago", secs / (86_400 * 7))
    }
}

/// `(PRETTY_NAME, generation)`, read once at startup.
fn system_identity() -> (Option<String>, Option<String>) {
    let pretty = std::fs::read_to_string("/etc/os-release")
        .ok()
        .and_then(|text| {
            text.lines()
                .find_map(|line| line.strip_prefix("PRETTY_NAME="))
                .map(|value| value.trim_matches('"').to_owned())
        });
    let generation = std::fs::read_link("/nix/var/nix/profiles/system")
        .ok()
        .and_then(|path| path.file_name()?.to_str().map(str::to_owned))
        .and_then(|name| {
            name.strip_prefix("system-")?
                .strip_suffix("-link")
                .map(str::to_owned)
        });
    (pretty, generation)
}

fn section_title(step: Step) -> String {
    format!("── {} ──", step.label())
}
