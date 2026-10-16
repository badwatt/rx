//! `--dry-run` adapter: prints commands instead of executing them.

use std::io::Write;

use crate::ansi;
use crate::error::Error;
use crate::ports::command::CommandRunner;

/// Adapter that echoes each command instead of running it.
pub(crate) struct DryRunRunner;

impl CommandRunner for DryRunRunner {
    fn run(&self, cmd: &[String]) -> Result<i32, Error> {
        let mut out = std::io::stdout();
        // Echo failure is non-critical; mirrors `write_parse_error`.
        let _ = print_command(&mut out, cmd);
        Ok(0)
    }

    fn capture(&self, cmd: &[String]) -> Result<(i32, String), Error> {
        let mut out = std::io::stdout();
        let _ = print_command(&mut out, cmd);
        Ok((0, String::new()))
    }
}

/// Print a command as `$ prog arg1 arg2 …`.
fn print_command(writer: &mut dyn Write, cmd: &[String]) -> Result<(), Error> {
    ansi::write_flush(writer, &format!("$ {}\n", cmd.join(" ")))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn print_command_writes_joined_argv() {
        let mut buf = Vec::new();
        assert!(print_command(&mut buf, &["git".into(), "pull".into()]).is_ok());
        assert_eq!(buf, b"$ git pull\n");
    }

    #[test]
    fn print_command_error_propagation() {
        let mut writer = crate::test_helpers::FailingWriter;
        assert!(print_command(&mut writer, &["git".into()]).is_err());
    }

    #[test]
    fn dry_run_runner_run_does_not_panic() {
        assert_eq!(DryRunRunner.run(&["true".into()]).expect("run"), 0);
    }

    #[test]
    fn dry_run_runner_capture_returns_empty() {
        let (status, out) = DryRunRunner.capture(&["true".into()]).expect("capture");
        assert_eq!(status, 0);
        assert!(out.is_empty());
    }
}
