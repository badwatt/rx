#![expect(
    clippy::unreadable_literal,
    reason = "colour literals read best as #RRGGBB"
)]

//! Visual tokens for the rx desktop surface.
//!
//! One world: deep GitHub-ink canvas from the README banner, one aurora accent
//! (blue -> violet -> pink) reserved for progress and focus, warm white type,
//! hairlines everywhere.  Nothing else earns a colour.

use gpui::{Background, FontWeight, Hsla, linear_color_stop, linear_gradient, rgb};

/// Canvas, back to front.
pub(crate) const INK: u32 = 0x0A0E15;
pub(crate) const INK_RAISED: u32 = 0x0E141E;
pub(crate) const INK_SUNKEN: u32 = 0x070A10;

/// Hairlines.
pub(crate) const LINE: u32 = 0x1C2735;
pub(crate) const LINE_SOFT: u32 = 0x141C27;

/// Type.
pub(crate) const TEXT: u32 = 0xE9F0F7;
pub(crate) const MUTED: u32 = 0x93A2B5;
pub(crate) const FAINT: u32 = 0x5F6E81;

/// Accents.  Aurora only for progress; semantic colours only for state.
pub(crate) const BLUE: u32 = 0x58A6FF;
pub(crate) const VIOLET: u32 = 0xBC8CFF;
pub(crate) const PINK: u32 = 0xF778BA;
pub(crate) const GREEN: u32 = 0x3FB950;
pub(crate) const RED: u32 = 0xF85149;
pub(crate) const AMBER: u32 = 0xE3B341;

/// Bundled typefaces (see `assets/fonts`).
pub(crate) const SANS: &str = "Inter";
pub(crate) const MONO: &str = "JetBrains Mono";

pub(crate) const REGULAR: FontWeight = FontWeight::NORMAL;
pub(crate) const MEDIUM: FontWeight = FontWeight::MEDIUM;
pub(crate) const SEMIBOLD: FontWeight = FontWeight::SEMIBOLD;

/// Nothing at all, for the "off" branch of conditional styles.
pub(crate) fn none() -> Hsla {
    Hsla::from(rgb(0x000000)).opacity(0.)
}

/// A colour at reduced presence, for hover washes and quiet chips.
pub(crate) fn dim(color: u32, alpha: f32) -> Hsla {
    Hsla::from(rgb(color)).opacity(alpha)
}

/// The signature: blue into pink, used on the beam and the primary action.
pub(crate) fn aurora() -> Background {
    linear_gradient(
        180.,
        linear_color_stop(rgb(BLUE), 0.),
        linear_color_stop(rgb(PINK), 1.),
    )
}
