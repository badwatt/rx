//! Pure helpers for building a command line and reading its exit status.

use crate::domain::error::Error;

/// Return the program name, or an error when `cmd` is empty.
pub(crate) fn program(cmd: &[String]) -> Result<&str, Error> {
    cmd.first().map(String::as_str).ok_or_else(|| {
        Error::Io(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "empty command",
        ))
    })
}

/// Extract the exit code from a process status.
///
/// Signal- and stop-terminated children report the shell convention `128 + N`;
/// when no signal is available, report `1`.
pub(crate) fn exit_code(status: std::process::ExitStatus) -> i32 {
    use std::os::unix::process::ExitStatusExt;
    match status.code() {
        Some(code) => code,
        None => status.signal().map_or(1, |signal| 128 + signal),
    }
}
