use std::io::{IsTerminal, Write};

// ANSI escape codes used in rx output styling.
pub(crate) const RED: &str = "\x1b[31m";
pub(crate) const GREEN: &str = "\x1b[32m";
pub(crate) const YELLOW: &str = "\x1b[33m";
pub(crate) const BLUE: &str = "\x1b[34m";
pub(crate) const CYAN: &str = "\x1b[36m";
pub(crate) const RESET: &str = "\x1b[0m";

/// The colors used for a given run. [`Palette::PLAIN`] renders without ANSI.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Palette {
    pub(crate) red: &'static str,
    pub(crate) green: &'static str,
    pub(crate) yellow: &'static str,
    pub(crate) blue: &'static str,
    pub(crate) cyan: &'static str,
    pub(crate) reset: &'static str,
}

impl Palette {
    pub(crate) const COLOR: Self = Self {
        red: RED,
        green: GREEN,
        yellow: YELLOW,
        blue: BLUE,
        cyan: CYAN,
        reset: RESET,
    };

    pub(crate) const PLAIN: Self = Self {
        red: "",
        green: "",
        yellow: "",
        blue: "",
        cyan: "",
        reset: "",
    };

    /// Palette for stdout.
    pub(crate) fn for_stdout() -> Self {
        Self::detect(std::io::stdout().is_terminal())
    }

    /// Palette for stderr.
    pub(crate) fn for_stderr() -> Self {
        Self::detect(std::io::stderr().is_terminal())
    }

    fn detect(is_tty: bool) -> Self {
        Self::from(color_env(std::env::var("COLOR").ok().as_deref()), is_tty)
    }

    /// Pure decision: `Some(true)` forces colour, `Some(false)` plain, and
    /// `None` follows the stream's TTY.
    pub(crate) fn from(color: Option<bool>, is_tty: bool) -> Self {
        match color {
            Some(true) => Self::COLOR,
            None if is_tty => Self::COLOR,
            Some(false) | None => Self::PLAIN,
        }
    }
}

/// `COLOR=true` forces colour and `COLOR=false` forces plain output; anything
/// else (or unset) leaves the decision to the stream.
fn color_env(value: Option<&str>) -> Option<bool> {
    match value {
        Some("true") => Some(true),
        Some("false") => Some(false),
        _ => None,
    }
}

/// Write `msg` to `writer` and flush immediately.
pub(crate) fn write_flush(writer: &mut dyn Write, msg: &str) -> Result<(), std::io::Error> {
    writer.write_all(msg.as_bytes())?;
    writer.flush()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn style_constants_are_non_empty() {
        assert_ne!(RED, "");
        assert_ne!(GREEN, "");
        assert_ne!(YELLOW, "");
        assert_ne!(BLUE, "");
        assert_ne!(CYAN, "");
        assert_ne!(RESET, "");
    }

    #[test]
    fn failing_writer_flush_is_err() {
        let mut w = crate::test_support::FailingFlushWriter;
        assert!(std::io::Write::flush(&mut w).is_err());
    }

    #[test]
    fn failing_writer_write_is_err() {
        let mut w = crate::test_support::FailingFlushWriter;
        assert!(w.write(b"x").is_err());
    }

    #[test]
    fn write_flush_writes_exact_bytes() {
        let mut buf = Vec::new();
        assert!(write_flush(&mut buf, "hello").is_ok());
        assert_eq!(buf, b"hello");
    }

    #[test]
    fn write_flush_propagates_write_error() {
        let mut writer = crate::test_support::FailingWriter;
        assert!(write_flush(&mut writer, "x").is_err());
    }

    #[test]
    fn write_flush_propagates_flush_error_on_empty_write() {
        // Write succeeds but flush fails
        let mut writer = crate::test_support::FlushFailingWriter;
        assert!(write_flush(&mut writer, "x").is_err());
    }

    #[test]
    fn palette_from_false_forces_plain_even_on_a_tty() {
        assert_eq!(Palette::from(Some(false), true), Palette::PLAIN);
    }

    #[test]
    fn palette_from_true_forces_color_even_off_a_tty() {
        assert_eq!(Palette::from(Some(true), false), Palette::COLOR);
    }

    #[test]
    fn palette_from_none_follows_the_tty() {
        assert_eq!(Palette::from(None, false), Palette::PLAIN);
        assert_eq!(Palette::from(None, true), Palette::COLOR);
    }

    #[test]
    fn palette_for_stdout_and_stderr_run() {
        let _ = Palette::for_stdout();
        let _ = Palette::for_stderr();
    }

    #[test]
    fn color_env_accepts_only_true_and_false() {
        assert_eq!(color_env(Some("true")), Some(true));
        assert_eq!(color_env(Some("false")), Some(false));
        assert_eq!(color_env(Some("1")), None);
        assert_eq!(color_env(None), None);
    }
}
