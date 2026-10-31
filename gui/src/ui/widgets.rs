//! Pure building blocks: chips, labels, log lines and machine facts.

use crate::pipeline::LineKind;
use super::prelude::*;
use super::{Line, Step};
pub(super) fn eyebrow(text: &str, color: u32) -> Div {
    div()
        .font_family(t::MONO)
        .text_size(px(9.))
        .text_color(rgb(color))
        .child(text.to_uppercase())
}

pub(super) fn chip(text: String, color: u32) -> Div {
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

pub(super) fn toggle(label: &'static str, on: bool, enabled: bool, warn: bool) -> Div {
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

pub(super) fn line_row(line: &Line) -> Div {
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

pub(super) fn note_row(text: String) -> Div {
    div()
        .font_family(t::MONO)
        .text_size(px(10.))
        .line_height(px(18.))
        .text_color(rgb(t::FAINT))
        .child(text)
}

pub(super) fn plan_row(command: &str) -> Div {
    div().flex().items_center().gap(px(10.)).child(
        div()
            .font_family(t::MONO)
            .text_size(px(10.5))
            .text_color(rgb(t::MUTED))
            .child(command.to_owned()),
    )
}

pub(crate) struct Generation {
    pub(super) number: u32,
    pub(super) current: bool,
    pub(super) age: String,
}

/// The newest `limit` system generations, newest first.
pub(super) fn recent_generations(limit: usize) -> Vec<Generation> {
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
pub(super) fn ago(modified: std::time::SystemTime) -> String {
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
pub(super) fn system_identity() -> (Option<String>, Option<String>) {
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

pub(super) fn section_title(step: Step) -> String {
    format!("── {} ──", step.label())
}
