use std::process::{Command, Stdio};

use crate::error::Error;
use crate::ports::command::CommandRunner;

/// Production [`CommandRunner`] backed by `std::process`.
pub(crate) struct ProcessRunner;

impl CommandRunner for ProcessRunner {
    fn run(&self, cmd: &[String]) -> Result<i32, Error> {
        run_cmd(cmd)
    }

    fn capture(&self, cmd: &[String]) -> Result<(i32, String), Error> {
        capture_cmd(cmd)
    }
}

/// Run a command via argv list (stdio inherited) and return its exit code.
///
/// A process killed by a signal returns `1`.
pub(crate) fn run_cmd(cmd: &[String]) -> Result<i32, Error> {
    let status = Command::new(&cmd[0])
        .args(&cmd[1..])
        .stdin(Stdio::inherit())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .status()
        .map_err(Error::Io)?;
    Ok(exit_code(status))
}

/// Run a command via argv list and return its exit code and lossy-UTF-8 stdout.
///
/// stderr is inherited so diagnostics stay visible.
pub(crate) fn capture_cmd(cmd: &[String]) -> Result<(i32, String), Error> {
    let output = Command::new(&cmd[0])
        .args(&cmd[1..])
        .stdin(Stdio::null())
        .stderr(Stdio::inherit())
        .output()
        .map_err(Error::Io)?;
    Ok((
        exit_code(output.status),
        String::from_utf8_lossy(&output.stdout).into_owned(),
    ))
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
        assert_eq!(run_cmd(&["true".into()]).expect("run true"), 0);
    }

    #[test]
    fn run_cmd_false_returns_one() {
        assert_eq!(run_cmd(&["false".into()]).expect("run false"), 1);
    }

    #[test]
    fn run_cmd_not_found_returns_io_error() {
        let result = run_cmd(&["nonexistent_cmd_zxy123".into()]);
        assert!(result.is_err());
        let err = result.expect_err("expected io err");
        assert_eq!(
            std::mem::discriminant(&err),
            std::mem::discriminant(&Error::Io(std::io::Error::other("")))
        );
    }

    #[test]
    fn run_cmd_signal_returns_one() {
        let result = run_cmd(&["sh".into(), "-c".into(), "kill -9 $$".into()]);
        assert!(result.is_ok());
        assert_eq!(result.expect("run cmd signal"), 1);
    }

    #[test]
    fn capture_cmd_echo_returns_status_and_stdout() {
        let (status, out) = capture_cmd(&["echo".into(), "hello".into()]).expect("capture echo");
        assert_eq!(status, 0);
        assert_eq!(out, "hello\n");
    }

    #[test]
    fn capture_cmd_nonzero_returns_status_and_stdout() {
        let (status, out) =
            capture_cmd(&["sh".into(), "-c".into(), "echo x; exit 3".into()]).expect("capture");
        assert_eq!(status, 3);
        assert_eq!(out, "x\n");
    }

    #[test]
    fn capture_cmd_not_found_returns_io_error() {
        assert!(capture_cmd(&["nonexistent_cmd_zxy123".into()]).is_err());
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
        assert_eq!(runner.run(&["true".into()]).expect("run true"), 0);
    }

    #[test]
    fn process_runner_capture_echo_returns_status_and_stdout() {
        let runner = ProcessRunner;
        let (status, out) = runner
            .capture(&["echo".into(), "hi".into()])
            .expect("capture");
        assert_eq!(status, 0);
        assert_eq!(out, "hi\n");
    }
}
