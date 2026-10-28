#![expect(
    clippy::unreadable_literal,
    reason = "colour literals read best as #RRGGBB"
)]

//! The rx desktop surface, one module per region of the window.
//!
//! Operate mode: the visitor completes a task.  Pipeline on the left, state
//! and live output in the middle, one saturated element — the aurora accent
//! that marks progress.  Colours, weights and gaps come from [`crate::theme`].

mod prelude;
mod rail;
mod sheets;
mod stage;
mod widgets;

use std::collections::VecDeque;
use std::path::PathBuf;
use std::sync::mpsc::{Receiver, TryRecvError};
use std::time::{Duration, Instant};

use gpui::prelude::*;
use gpui::{Context, FocusHandle, KeyDownEvent, Render, Window, div, rgb};

use crate::pipeline::{self, LineKind, Msg, RunConfig, Step};
use crate::prefs::Prefs;
use crate::theme as t;

use self::widgets::{Generation, recent_generations, section_title, system_identity};

/// The CLI version, single-sourced from the repository-root `VERSION` file.
const VERSION: &str = include_str!("../../../VERSION");

/// Lines newer than this are dropped; the console only ever shows the tail.
const LOG_CAP: usize = 4000;
/// Visible rows in the console well (height / line height).
const CONSOLE_ROWS: usize = 14;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum Phase {
    Idle,
    Confirm,
    Running,
    Done,
    Failed,
    Auth,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum Status {
    Pending,
    Running,
    Ok,
    Failed,
}

pub(crate) struct Row {
    pub(crate) step: Step,
    pub(crate) status: Status,
    pub(crate) took: Option<Duration>,
}

pub(crate) struct Line {
    pub(crate) kind: LineKind,
    pub(crate) text: String,
}

pub(crate) struct RxApp {
    pub(crate) cfg: RunConfig,
    pub(crate) phase: Phase,
    pub(crate) rows: Vec<Row>,
    pub(crate) log: VecDeque<Line>,
    pub(crate) started: Option<Instant>,
    pub(crate) elapsed: Option<Duration>,
    pub(crate) rx_binary: Option<PathBuf>,
    pub(crate) root_ready: bool,
    pub(crate) system: (Option<String>, Option<String>),
    pub(crate) generations: Vec<Generation>,
    pub(crate) auth: String,
    pub(crate) auth_error: bool,
    pub(crate) auth_busy: bool,
    pub(crate) auth_feed: Option<Receiver<(bool, Option<PathBuf>)>>,
    pub(crate) askpass: Option<PathBuf>,
    pub(crate) focus: FocusHandle,
    pub(crate) error: Option<String>,
    pub(crate) feed: Option<Receiver<Msg>>,
}

impl RxApp {
    pub(crate) fn new(cx: &mut Context<Self>) -> Self {
        // The toggles the visitor picked last time come back on launch; the
        // CLI itself keeps its documented defaults.
        let prefs = Prefs::load();
        let cfg = RunConfig {
            update: prefs.update,
            diff: prefs.diff,
            dry_run: prefs.dry_run,
            ..RunConfig::default()
        };
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
            auth_feed: None,
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
        self.drain_auth(cx);
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

    /// Drain the auth result; called from the foreground poller.
    fn drain_auth(&mut self, cx: &mut Context<Self>) {
        let result = self
            .auth_feed
            .as_ref()
            .and_then(|feed| feed.try_recv().ok());
        let Some((ok, stash)) = result else {
            return;
        };
        self.auth_feed = None;
        self.auth_busy = false;
        if ok {
            if let Some(path) = stash {
                self.askpass = Some(path);
                self.root_ready = true;
                self.begin(cx);
            } else {
                // Without the stash a long rebuild would prompt again with no
                // terminal; stay here and say so instead of failing later.
                self.auth_error = true;
                self.error = Some(String::from(
                    "could not store the password for sudo; try again",
                ));
                cx.notify();
            }
        } else {
            self.auth_error = true;
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

    /// Validate the typed password on a worker thread.
    ///
    /// `sudo` can block on PAM, LDAP or a fingerprint reader, so it must not
    /// run on the frame thread; the result arrives through [`Self::pump`].
    fn submit_password(&mut self, cx: &mut Context<Self>) {
        if self.auth.is_empty() || self.auth_busy {
            return;
        }
        self.auth_busy = true;
        self.auth_error = false;
        let mut password = std::mem::take(&mut self.auth);
        let (tx, rx) = std::sync::mpsc::channel();
        self.auth_feed = Some(rx);
        std::thread::spawn(move || {
            let ok = pipeline::authenticate(&password);
            let stash = ok.then(|| pipeline::stash_secret(&password));
            pipeline::wipe(&mut password);
            let _ = tx.send((ok, stash.flatten()));
        });
        cx.notify();
    }

    fn on_key(&mut self, event: &KeyDownEvent, cx: &mut Context<Self>) {
        match event.keystroke.key.as_ref() {
            "enter" => self.submit_password(cx),
            "escape" => {
                pipeline::wipe(&mut self.auth);
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

    fn begin(&mut self, cx: &mut Context<Self>) {
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
        self.persist();
        self.reset(cx);
    }

    fn toggle_diff(&mut self, cx: &mut Context<Self>) {
        self.cfg.diff = !self.cfg.diff;
        self.persist();
        self.reset(cx);
    }

    fn toggle_dry(&mut self, cx: &mut Context<Self>) {
        self.cfg.dry_run = !self.cfg.dry_run;
        self.persist();
        cx.notify();
    }

    /// Remember the toggles for the next launch.
    fn persist(&self) {
        Prefs {
            update: self.cfg.update,
            diff: self.cfg.diff,
            dry_run: self.cfg.dry_run,
        }
        .save();
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

    /// Is any worker still reporting to the UI?
    pub(crate) fn is_busy(&self) -> bool {
        self.feed.is_some() || self.auth_feed.is_some()
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
