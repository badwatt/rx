use std::process::{Command, Stdio};

use crate::domain::error::Error;
use crate::infra::command::CommandRunner;
use crate::infra::exit::exit_code;

/// Return the program name, or an error when `cmd` is empty.
fn program(cmd: &[String]) -> Result<&str, Error> {
    cmd.first().map(String::as_str).ok_or_else(|| {
        Error::Io(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "empty command",
        ))
    })
}

/// Production [`CommandRunner`] backed by `std::process`.
pub(crate) struct ProcessRunner;

impl CommandRunner for ProcessRunner {
    fn run(&self, cmd: &[String]) -> Result<i32, Error> {
        run_cmd(cmd)
    }

    fn run_in(&self, dir: &str, cmd: &[String]) -> Result<i32, Error> {
        run_cmd_in(dir, cmd)
    }

    fn run_with_env(&self, cmd: &[String], var: &str, value: &str) -> Result<i32, Error> {
        run_cmd_env(cmd, var, value)
    }

    fn capture(&self, cmd: &[String]) -> Result<(i32, String), Error> {
        capture_cmd(cmd)
    }

    fn probe(&self, cmd: &[String]) -> Result<i32, Error> {
        probe_cmd(cmd)
    }
}

/// Run a command via argv list (stdio inherited) and return its exit code.
///
/// A process killed by a signal returns `1`.
pub(crate) fn run_cmd(cmd: &[String]) -> Result<i32, Error> {
    run_impl(cmd, None, None)
}

/// Run a command via argv list with `dir` as its working directory.
pub(crate) fn run_cmd_in(dir: &str, cmd: &[String]) -> Result<i32, Error> {
    run_impl(cmd, Some(dir), None)
}

/// Run a command via argv list with `var=value` in its environment.
pub(crate) fn run_cmd_env(cmd: &[String], var: &str, value: &str) -> Result<i32, Error> {
    run_impl(cmd, None, Some((var, value)))
}

/// Shared body of [`run_cmd`], [`run_cmd_in`] and [`run_cmd_env`].
fn run_impl(cmd: &[String], dir: Option<&str>, env: Option<(&str, &str)>) -> Result<i32, Error> {
    let mut command = Command::new(program(cmd)?);
    command
        .args(&cmd[1..])
        .stdin(Stdio::inherit())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit());
    if let Some(dir) = dir {
        command.current_dir(dir);
    }
    if let Some((var, value)) = env {
        command.env(var, value);
    }
    let status = command.status().map_err(Error::Io)?;
    Ok(exit_code(status))
}

/// Run a command via argv list and return its exit code and lossy-UTF-8 stdout.
///
/// stderr is inherited so diagnostics stay visible.
pub(crate) fn capture_cmd(cmd: &[String]) -> Result<(i32, String), Error> {
    let output = Command::new(program(cmd)?)
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

/// Run a command via argv list with stdout/stderr discarded, returning its
/// exit code. Used for capability probes that must stay silent.
pub(crate) fn probe_cmd(cmd: &[String]) -> Result<i32, Error> {
    let status = Command::new(program(cmd)?)
        .args(&cmd[1..])
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map_err(Error::Io)?;
    Ok(exit_code(status))
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
    fn run_cmd_in_uses_the_working_directory() {
        let cmd = vec!["sh".into(), "-c".into(), "test \"$(pwd)\" = /".into()];
        assert_eq!(run_cmd_in("/", &cmd).expect("run in root"), 0);
    }

    #[test]
    fn run_cmd_in_missing_directory_returns_io_error() {
        let cmd = vec!["true".into()];
        assert!(run_cmd_in("/nonexistent_rx_dir_xyz", &cmd).is_err());
    }

    #[test]
    fn process_runner_run_in_returns_zero() {
        assert_eq!(
            ProcessRunner.run_in("/", &["true".into()]).expect("run in"),
            0
        );
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
    fn run_cmd_signal_returns_signal_code() {
        let result = run_cmd(&["sh".into(), "-c".into(), "kill -9 $$".into()]);
        assert!(result.is_ok());
        assert_eq!(result.expect("run cmd signal"), 137);
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
    fn run_cmd_empty_returns_error() {
        assert!(run_cmd(&[]).is_err());
    }

    #[test]
    fn capture_cmd_empty_returns_error() {
        assert!(capture_cmd(&[]).is_err());
    }

    #[test]
    fn probe_cmd_true_returns_zero() {
        assert_eq!(probe_cmd(&["true".into()]).expect("probe true"), 0);
    }

    #[test]
    fn probe_cmd_false_returns_one() {
        assert_eq!(probe_cmd(&["false".into()]).expect("probe false"), 1);
    }

    #[test]
    fn probe_cmd_not_found_returns_io_error() {
        assert!(probe_cmd(&["nonexistent_cmd_zxy123".into()]).is_err());
    }

    #[test]
    fn probe_cmd_empty_returns_error() {
        assert!(probe_cmd(&[]).is_err());
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

    #[test]
    fn run_cmd_env_exposes_the_variable() {
        let cmd = vec![
            "sh".into(),
            "-c".into(),
            "test \"$GIT_TERMINAL_PROMPT\" = 0".into(),
        ];
        assert_eq!(
            run_cmd_env(&cmd, "GIT_TERMINAL_PROMPT", "0").expect("run env"),
            0
        );
    }

    #[test]
    fn run_cmd_env_surfaces_failure_exit_code() {
        let cmd = vec!["sh".into(), "-c".into(), "test -n \"$K\"".into()];
        assert_eq!(run_cmd_env(&cmd, "K", "").expect("run env"), 1);
    }

    #[test]
    fn run_cmd_env_not_found_returns_io_error() {
        assert!(run_cmd_env(&["nonexistent_cmd_zxy123".into()], "K", "v").is_err());
    }

    #[test]
    fn run_cmd_env_empty_returns_error() {
        assert!(run_cmd_env(&[], "K", "v").is_err());
    }

    #[test]
    fn process_runner_run_with_env_returns_zero() {
        assert_eq!(
            ProcessRunner
                .run_with_env(&["true".into()], "K", "v")
                .expect("run env"),
            0
        );
    }

    #[test]
    fn process_runner_probe_true_returns_zero() {
        assert_eq!(ProcessRunner.probe(&["true".into()]).expect("probe"), 0);
    }
}
