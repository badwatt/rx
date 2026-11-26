//! Command result helpers: exit codes and the success check.

use crate::domain::error::Error;

/// `Ok(())` when `status` is a success, else `error`.
///
/// Every adapter maps a non-zero exit to its own error variant; the check
/// lives here instead of being repeated at each call site.
pub(crate) fn ensure_success(status: i32, error: Error) -> Result<(), Error> {
    if status == 0 { Ok(()) } else { Err(error) }
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::process::Command;

    #[test]
    fn ensure_success_accepts_zero() {
        assert!(ensure_success(0, Error::GitPullFailed).is_ok());
    }

    #[test]
    fn ensure_success_maps_a_nonzero_status_to_the_given_error() {
        let error = ensure_success(1, Error::GitPullFailed).expect_err("nonzero status");
        assert_eq!(error.to_string(), "Failed to pull changes");
    }

    #[test]
    fn exit_code_maps_signal_to_128_plus_n() {
        let status = Command::new("sh")
            .arg("-c")
            .arg("kill -9 $$")
            .status()
            .expect("spawn kill");
        assert_eq!(exit_code(status), 137);
    }

    #[cfg(unix)]
    #[test]
    fn exit_code_unknown_termination_is_one() {
        use std::os::unix::process::ExitStatusExt;
        // A stopped child has neither an exit code nor a terminating signal.
        let stopped = std::process::ExitStatus::from_raw(0x7f | (20 << 8));
        assert_eq!(exit_code(stopped), 1);
    }

    #[test]
    fn exit_code_returns_code_for_normal_exit() {
        let status = Command::new("false").status().expect("spawn false");
        assert_eq!(exit_code(status), 1);
    }
}
