use super::prelude::*;
use super::widgets::{chip, eyebrow};
use super::{Phase, Row, RxApp, Status, VERSION};

impl RxApp {
    pub(super) fn top_bar(&self) -> Div {
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

    pub(super) fn rail(&self) -> Div {
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
    pub(super) fn generations_block(&self) -> Vec<Div> {
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
    pub(super) fn identity_block(&self) -> Vec<Div> {
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

    pub(super) fn step_row(&self, index: usize, row: &Row) -> Div {
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

    pub(super) fn status_bar(&self) -> Div {
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
