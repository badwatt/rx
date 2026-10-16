use std::process::{Command, Stdio};

use crate::error::Error;
use crate::ports::command::CommandRunner;

/// Production [`CommandRunner`] backed by `std::process`.
pub struct ProcessRunner;

impl CommandRunner for ProcessRunner {
    fn run(&self, cmd: &[String], output: bool) -> Result<i32, Error> {
        run_cmd(cmd, output)
    }

    fn run_pipeline(&self, cmd: &str, output: bool) -> Result<i32, Error> {
        run_pipeline(cmd, output)
    }
}

/// Run a command via argv list and return its exit code.
///
/// When `output` is `false`, stdout and stderr are redirected to `/dev/null`.
/// A process killed by a signal returns `1`.
pub fn run_cmd(cmd: &[String], output: bool) -> Result<i32, Error> {
    let status = Command::new(&cmd[0])
        .args(&cmd[1..])
        .stdin(Stdio::inherit())
        .stdout(stdio_for(output))
        .stderr(stdio_for(output))
        .status()
        .map_err(Error::Io)?;
    Ok(exit_code(status))
}

/// Run a shell pipeline string via `sh -c`.
///
/// Used only for commands requiring pipes (e.g. `nix_diff`).
/// When `output` is `false`, stdout and stderr are redirected to `/dev/null`.
/// A process killed by a signal returns `1`.
pub fn run_pipeline(cmd: &str, output: bool) -> Result<i32, Error> {
    run_cmd(&["sh".into(), "-c".into(), cmd.into()], output)
}

/// Select `Stdio` based on whether output should be visible.
fn stdio_for(output: bool) -> Stdio {
    if output {
        Stdio::inherit()
    } else {
        Stdio::null()
    }
}

/// Extract exit code from process status; signal-killed processes return 1.
fn exit_code(status: std::process::ExitStatus) -> i32 {
    status.code().unwrap_or(1)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn run_cmd_true_returns_zero() {
        assert_eq!(run_cmd(&["true".into()], true).expect("run true"), 0);
    }

    #[test]
    fn run_cmd_false_returns_one() {
        assert_eq!(run_cmd(&["false".into()], true).expect("run false"), 1);
    }

    #[test]
    fn run_cmd_echo_no_output_returns_zero() {
        assert_eq!(
            run_cmd(&["echo".into(), "hello".into()], false).expect("run echo"),
            0
        );
    }

    #[test]
    fn run_cmd_not_found_returns_io_error() {
        let result = run_cmd(&["nonexistent_cmd_zxy123".into()], false);
        assert!(result.is_err());
        let err = result.expect_err("expected io err");
        assert_eq!(
            std::mem::discriminant(&err),
            std::mem::discriminant(&Error::Io(std::io::Error::other("")))
        );
    }

    #[test]
    fn run_cmd_signal_returns_one() {
        let result = run_cmd(&["sh".into(), "-c".into(), "kill -9 $$".into()], false);
        assert!(result.is_ok());
        assert_eq!(result.expect("run cmd signal"), 1);
    }

    #[test]
    fn run_pipeline_true_returns_zero() {
        assert_eq!(run_pipeline("true", true).expect("run true"), 0);
    }

    #[test]
    fn run_pipeline_echo_no_output_returns_zero() {
        assert_eq!(run_pipeline("echo hello", false).expect("run echo"), 0);
    }

    #[test]
    fn run_pipeline_not_found_returns_nonzero() {
        let result = run_pipeline("nonexistent_cmd_zxy123", false);
        assert!(result.is_ok());
        assert_ne!(result.expect("run nonexistent"), 0);
    }

    #[test]
    fn run_pipeline_signal_returns_one() {
        let result = run_pipeline("kill -9 $$", false);
        assert!(result.is_ok());
        assert_eq!(result.expect("run pipeline signal"), 1);
    }

    #[test]
    fn exit_code_returns_one_for_signal() {
        let status = Command::new("sh")
            .arg("-c")
            .arg("kill -9 $$")
            .status()
            .expect("spawn kill");
        assert_eq!(exit_code(status), 1);
    }

    #[test]
    fn exit_code_returns_code_for_normal_exit() {
        let status = Command::new("false").status().expect("spawn false");
        assert_eq!(exit_code(status), 1);
    }

    #[test]
    fn process_runner_run_true_returns_zero() {
        let runner = ProcessRunner;
        assert_eq!(runner.run(&["true".into()], false).expect("run true"), 0);
    }

    #[test]
    fn process_runner_run_pipeline_true_returns_zero() {
        let runner = ProcessRunner;
        assert_eq!(runner.run_pipeline("true", false).expect("run true"), 0);
    }
}
