
use gpui::KeyDownEvent;

use super::prelude::*;
use super::widgets::eyebrow;
use super::{Phase, RxApp, VERSION};
impl RxApp {
    /// One row per planned step, separator between them.
    pub(super) fn confirm_rows(&self) -> Vec<Div> {
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

    pub(super) fn confirm_panel(&self, cx: &mut Context<Self>) -> Div {
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
    pub(super) fn confirm_footer(cx: &mut Context<Self>) -> Div {
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
                            .on_click(cx.listener(|this, _, _window, cx| this.begin(cx)))
                            .child("Start"),
                    ),
            )
    }

    /// Masked password field plus the unlock action.
    pub(super) fn auth_row(&self, cx: &mut Context<Self>) -> Div {
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
                    .on_key_down(cx.listener(|this, event: &KeyDownEvent, _window, cx| {
                        this.on_key(event, cx);
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
                    .on_click(cx.listener(|this, _, _, cx| this.submit_password(cx)))
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
}
