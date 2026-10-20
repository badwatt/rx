use std::io::Write;

// ANSI escape codes used in rx output styling.
pub(crate) const RED: &str = "\x1b[31m";
pub(crate) const GREEN: &str = "\x1b[32m";
pub(crate) const YELLOW: &str = "\x1b[33m";
pub(crate) const BLUE: &str = "\x1b[34m";
// Used only in tests; kept for future styling needs.
#[allow(dead_code)]
pub(crate) const MAGENTA: &str = "\x1b[35m";
pub(crate) const CYAN: &str = "\x1b[36m";
// Used only in tests; kept for future styling needs.
#[allow(dead_code)]
pub(crate) const GRAY: &str = "\x1b[37m";
// Used only in tests; kept for future styling needs.
#[allow(dead_code)]
pub(crate) const BLACK: &str = "\x1b[30m";
pub(crate) const RESET: &str = "\x1b[0m";
// Used only in tests; kept for future styling needs.
#[allow(dead_code)]
pub(crate) const BOLD: &str = "\x1b[1m";
// Used only in tests; kept for future styling needs.
#[allow(dead_code)]
pub(crate) const UNDERLINE: &str = "\x1b[4m";

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
        assert_ne!(MAGENTA, "");
        assert_ne!(CYAN, "");
        assert_ne!(GRAY, "");
        assert_ne!(BLACK, "");
        assert_ne!(RESET, "");
        assert_ne!(BOLD, "");
        assert_ne!(UNDERLINE, "");
    }

    #[test]
    fn failing_writer_flush_is_err() {
        let mut w = crate::shared::test_helpers::FailingFlushWriter;
        assert!(std::io::Write::flush(&mut w).is_err());
    }

    #[test]
    fn failing_writer_write_is_err() {
        let mut w = crate::shared::test_helpers::FailingFlushWriter;
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
        let mut writer = crate::shared::test_helpers::FailingWriter;
        assert!(write_flush(&mut writer, "x").is_err());
    }

    #[test]
    fn write_flush_propagates_flush_error_on_empty_write() {
        // Write succeeds but flush fails
        let mut writer = crate::shared::test_helpers::FlushFailingWriter;
        assert!(write_flush(&mut writer, "x").is_err());
    }
}
