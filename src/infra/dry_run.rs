//! `--dry-run` adapter: prints commands instead of executing them.
//!
//! Probes are delegated to a real runner: they are read-only by contract, and
//! echoing them would both add noise and make every probe "succeed", hiding the
//! detected `nixos-rebuild` flavor from the preview.

use std::io::Write;

use crate::domain::error::Error;
use crate::infra::command::CommandRunner;
use crate::shared::ansi;

/// Adapter that echoes each command instead of running it.
pub(crate) struct DryRunRunner<'a> {
    probe_runner: &'a dyn CommandRunner,
}

impl<'a> DryRunRunner<'a> {
    pub(crate) fn new(probe_runner: &'a dyn CommandRunner) -> Self {
        Self { probe_runner }
    }
}

impl CommandRunner for DryRunRunner<'_> {
    fn run(&self, cmd: &[String]) -> Result<i32, Error> {
        let mut out = std::io::stdout();
        // Echo failure is non-critical; mirrors `write_parse_error`.
        let _ = print_command(&mut out, cmd);
        Ok(0)
    }

    fn run_with_env(&self, cmd: &[String], var: &str, value: &str) -> Result<i32, Error> {
        let mut out = std::io::stdout();
        // Echo failure is non-critical; mirrors `write_parse_error`.
        let _ = print_command_env(&mut out, cmd, var, value);
        Ok(0)
    }

    fn capture(&self, cmd: &[String]) -> Result<(i32, String), Error> {
        let mut out = std::io::stdout();
        let _ = print_command(&mut out, cmd);
        Ok((0, String::new()))
    }

    fn probe(&self, cmd: &[String]) -> Result<i32, Error> {
        self.probe_runner.probe(cmd)
    }
}

/// Render `cmd` as a copy-pasteable shell command line.
fn render(cmd: &[String]) -> String {
    cmd.iter()
        .map(|arg| quote_arg(arg))
        .collect::<Vec<_>>()
        .join(" ")
}

/// Single-quote `arg` unless it is already a simple shell word.
///
/// Embedded single quotes become `'\''`, so the preview survives a paste.
fn quote_arg(arg: &str) -> String {
    let simple = !arg.is_empty()
        && arg
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || "_@%+=:,./-".contains(c));
    if simple {
        return arg.to_string();
    }
    let mut out = String::with_capacity(arg.len() + 2);
    out.push('\'');
    for c in arg.chars() {
        if c == '\'' {
            out.push_str("'\\''");
        } else {
            out.push(c);
        }
    }
    out.push('\'');
    out
}

/// Print a command as `$ prog arg1 arg2 …`.
fn print_command(writer: &mut dyn Write, cmd: &[String]) -> Result<(), Error> {
    ansi::write_flush(writer, &format!("$ {}\n", render(cmd)))?;
    Ok(())
}

/// Print a command with an environment assignment, as
/// `$ VAR=value prog arg1 arg2 …`.
fn print_command_env(
    writer: &mut dyn Write,
    cmd: &[String],
    var: &str,
    value: &str,
) -> Result<(), Error> {
    let (var, value) = (quote_arg(var), quote_arg(value));
    ansi::write_flush(writer, &format!("$ {var}={value} {}\n", render(cmd)))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shared::test_helpers::StubRunner;

    #[test]
    fn print_command_writes_joined_argv() {
        let mut buf = Vec::new();
        assert!(print_command(&mut buf, &["git".into(), "pull".into()]).is_ok());
        assert_eq!(buf, b"$ git pull\n");
    }

    #[test]
    fn print_command_env_writes_assignment_and_argv() {
        let mut buf = Vec::new();
        assert!(print_command_env(&mut buf, &["git".into(), "pull".into()], "K", "v").is_ok());
        assert_eq!(buf, b"$ K=v git pull\n");
    }

    #[test]
    fn print_command_error_propagation() {
        let mut writer = crate::shared::test_helpers::FailingWriter;
        assert!(print_command(&mut writer, &["git".into()]).is_err());
    }

    #[test]
    fn print_command_env_error_propagation() {
        let mut writer = crate::shared::test_helpers::FailingWriter;
        assert!(print_command_env(&mut writer, &["git".into()], "K", "v").is_err());
    }

    #[test]
    fn dry_run_runner_run_does_not_panic() {
        let probes = StubRunner::new();
        assert_eq!(
            DryRunRunner::new(&probes)
                .run(&["true".into()])
                .expect("run"),
            0
        );
    }

    #[test]
    fn dry_run_runner_capture_returns_empty() {
        let probes = StubRunner::new();
        let (status, out) = DryRunRunner::new(&probes)
            .capture(&["true".into()])
            .expect("capture");
        assert_eq!(status, 0);
        assert_eq!(out, "");
    }

    #[test]
    fn dry_run_runner_run_with_env_prints_env_assignment() {
        let probes = StubRunner::new();
        assert_eq!(
            DryRunRunner::new(&probes)
                .run_with_env(&["git".into(), "pull".into()], "K", "v")
                .expect("run env"),
            0
        );
    }

    #[test]
    fn dry_run_runner_delegates_probes_to_the_real_runner() {
        let probes = StubRunner::new().with_probe(|_| Ok(1));
        assert_eq!(
            DryRunRunner::new(&probes)
                .probe(&["nixos-rebuild".into()])
                .expect("probe"),
            1
        );
    }

    #[test]
    fn print_command_quotes_arguments_that_need_it() {
        let mut buf = Vec::new();
        let cmd = vec![
            String::from("prog"),
            String::from("two words"),
            String::from("it's"),
            String::new(),
        ];
        assert!(print_command(&mut buf, &cmd).is_ok());
        assert_eq!(
            String::from_utf8_lossy(&buf),
            "$ prog 'two words' 'it'\\''s' ''\n"
        );
    }

    #[test]
    fn print_command_env_quotes_the_value() {
        let mut buf = Vec::new();
        let cmd = vec![String::from("git"), String::from("pull")];
        assert!(print_command_env(&mut buf, &cmd, "VAR", "a b").is_ok());
        assert_eq!(String::from_utf8_lossy(&buf), "$ VAR='a b' git pull\n");
    }
}
