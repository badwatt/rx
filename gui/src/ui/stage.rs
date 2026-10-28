use super::prelude::*;
use super::widgets::{eyebrow, line_row, note_row, plan_row, toggle};
use super::{CONSOLE_ROWS, Phase, RxApp, Status};

impl RxApp {
    pub(super) fn workspace(&self, cx: &mut Context<Self>) -> Div {
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

    pub(super) fn hero(&self, cx: &mut Context<Self>) -> Div {
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

    pub(super) fn primary_action(&self, cx: &mut Context<Self>) -> impl IntoElement {
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
            .on_click(cx.listener(|this, _, _window, cx| match this.phase {
                Phase::Auth => this.submit_password(cx),
                Phase::Done | Phase::Failed => this.reset(cx),
                _ => this.request_run(cx),
            }))
    }

    pub(super) fn ghost_toggles(&self, cx: &mut Context<Self>) -> Div {
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

    pub(super) fn console(&self) -> Div {
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
    pub(super) fn empty_state(&self) -> Vec<Div> {
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
}
