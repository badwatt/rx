//! Process policy for the composition root: exit codes, quiet failures and
//! the one place an error is voiced.

use std::io::Write;

use crate::domain::error::Error;
use crate::shared::ansi::Palette;

pub(crate) fn exit_code_for(error: &Error) -> i32 {
    match error {
        Error::Io(e) if e.kind() == std::io::ErrorKind::BrokenPipe => 141,
        Error::InvalidArgs | Error::InvalidConfig(_) => 2,
        _ => 1,
    }
}

/// A closed stdout (e.g. `rx | head`) is a normal way to stop, so it must not
/// be reported as an error.
pub(crate) fn is_quiet(error: &Error) -> bool {
    matches!(error, Error::Io(e) if e.kind() == std::io::ErrorKind::BrokenPipe)
}

/// Print a terminal error unless it is quiet. Returns whether anything was
/// printed.
pub(crate) fn report_error(writer: &mut dyn Write, palette: Palette, error: &Error) -> bool {
    // Quiet on a closed stdout; the parser already printed the detail for
    // invalid arguments.
    if is_quiet(error) || matches!(error, Error::InvalidArgs) {
        return false;
    }
    let _ = writeln!(writer, "{}Error: {}{}", palette.red, error, palette.reset);
    true
}

/// Palettes for this process. `--no-color` forces plain output for both
/// streams; otherwise each stream decides from `NO_COLOR` and its own TTY.
pub(crate) fn palettes(no_color: bool) -> (Palette, Palette) {
    if no_color {
        (Palette::PLAIN, Palette::PLAIN)
    } else {
        (Palette::for_stdout(), Palette::for_stderr())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn palettes_honor_no_color_flag() {
        assert_eq!(palettes(true), (Palette::PLAIN, Palette::PLAIN));
    }
    #[test]
    fn palettes_use_detected_streams_by_default() {
        let (out, err) = palettes(false);
        assert_eq!(out, Palette::for_stdout());
        assert_eq!(err, Palette::for_stderr());
    }
    #[test]
    fn exit_code_invalid_args_is_2() {
        assert_eq!(exit_code_for(&Error::InvalidArgs), 2);
    }
    #[test]
    fn exit_code_invalid_config_is_2() {
        assert_eq!(exit_code_for(&Error::InvalidConfig("bad")), 2);
    }
    #[test]
    fn exit_code_git_pull_failed_is_1() {
        assert_eq!(exit_code_for(&Error::GitPullFailed), 1);
    }
    #[test]
    fn exit_code_io_error_is_1() {
        assert_eq!(exit_code_for(&Error::Io(std::io::Error::other("x"))), 1);
    }
    #[test]
    fn exit_code_broken_pipe_is_141() {
        let err = Error::Io(std::io::Error::from(std::io::ErrorKind::BrokenPipe));
        assert_eq!(exit_code_for(&err), 141);
    }
    #[test]
    fn is_quiet_only_for_broken_pipe() {
        let pipe = Error::Io(std::io::Error::from(std::io::ErrorKind::BrokenPipe));
        assert!(is_quiet(&pipe));
        assert!(!is_quiet(&Error::GitPullFailed));
    }
    #[test]
    fn report_error_skips_broken_pipe() {
        let pipe = Error::Io(std::io::Error::from(std::io::ErrorKind::BrokenPipe));
        let mut buf = Vec::new();
        assert!(!report_error(&mut buf, Palette::PLAIN, &pipe));
        assert_eq!(String::from_utf8_lossy(&buf), "");
    }
    #[test]
    fn report_error_skips_invalid_args() {
        let mut buf = Vec::new();
        assert!(!report_error(&mut buf, Palette::PLAIN, &Error::InvalidArgs));
        assert_eq!(String::from_utf8_lossy(&buf), "");
    }
    #[test]
    fn report_error_writes_other_errors() {
        let mut buf = Vec::new();
        assert!(report_error(
            &mut buf,
            Palette::PLAIN,
            &Error::GitPullFailed
        ));
        assert!(String::from_utf8_lossy(&buf).contains("Failed to pull changes"));
    }
}
