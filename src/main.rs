mod cli;
mod domain;
mod infra;
mod shared;
mod system_update;

use cli::Intent;
use domain::config::Config;
use domain::error::Error;
use infra::command::CommandRunner;
use infra::dry_run::DryRunRunner;
use infra::process::ProcessRunner;
use shared::ansi::Palette;
use std::io::Write;
use std::{env, io::stdout};
use system_update::adapters::git::GitRepo;
use system_update::adapters::nix::NixCli;
use system_update::adapters::terminal::TerminalIo;

fn exit_code_for(error: &Error) -> i32 {
    match error {
        Error::Io(e) if e.kind() == std::io::ErrorKind::BrokenPipe => 141,
        Error::InvalidArgs | Error::InvalidConfig(_) => 2,
        _ => 1,
    }
}

/// A closed stdout (e.g. `rx | head`) is a normal way to stop, so it must not
/// be reported as an error.
fn is_quiet(error: &Error) -> bool {
    matches!(error, Error::Io(e) if e.kind() == std::io::ErrorKind::BrokenPipe)
}

/// Print a terminal error unless it is quiet. Returns whether anything was
/// printed.
fn report_error(writer: &mut dyn Write, palette: Palette, error: &Error) -> bool {
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
fn palettes(no_color: bool) -> (Palette, Palette) {
    if no_color {
        (Palette::PLAIN, Palette::PLAIN)
    } else {
        (Palette::for_stdout(), Palette::for_stderr())
    }
}

fn main() {
    let args: Vec<String> = env::args().collect();
    let mut out = stdout();
    let mut err = std::io::stderr();
    let no_color = args.iter().any(|arg| arg == "--no-color");
    let (out_palette, err_palette) = palettes(no_color);
    if let Err(e) = run(&args, &mut out, &mut err, out_palette) {
        report_error(&mut err, err_palette, &e);
        std::process::exit(exit_code_for(&e));
    }
}

/// Parse the arguments and dispatch to help/version or the update workflow.
///
/// Parse diagnostics go to `err`; help, prompts and status go to `out`.
fn run(
    args: &[String],
    out: &mut dyn Write,
    err: &mut dyn Write,
    out_palette: Palette,
) -> Result<(), Error> {
    match cli::parse::parse(args, err) {
        Intent::Help => cli::help::print_help(out),
        Intent::Version => cli::help::print_version(out, out_palette),
        Intent::Completions(shell) => cli::completions::print(out, shell),
        Intent::Invalid => Err(Error::InvalidArgs),
        Intent::Run(config) => run_workflow(&config, out_palette),
    }
}

/// Composition root for the workflow: wire the adapters and run the use case.
fn run_workflow(config: &Config, out_palette: Palette) -> Result<(), Error> {
    let ui = TerminalIo::new(out_palette);
    let real = ProcessRunner;
    let dry = DryRunRunner::new(&real);
    let runner: &dyn CommandRunner = if config.dry_run { &dry } else { &real };
    let repo = GitRepo::new(runner);
    let nix = NixCli::new(runner);
    system_update::application::run(config, &repo, &nix, &ui)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn args(from: &[&str]) -> Vec<String> {
        from.iter().copied().map(str::to_string).collect()
    }

    /// Run with the colored palette so existing assertions keep working.
    fn run(args: &[String], out: &mut dyn Write) -> Result<(), Error> {
        super::run(args, out, &mut Vec::new(), Palette::COLOR)
    }

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

    #[test]
    fn run_parse_error_goes_to_err_writer() {
        let mut out = Vec::new();
        let mut err = Vec::new();
        let result = super::run(&args(&["rx", "-x"]), &mut out, &mut err, Palette::PLAIN);
        assert!(result.is_err());
        assert_eq!(String::from_utf8_lossy(&out), "");
        assert!(String::from_utf8_lossy(&err).contains("Unknown flag"));
    }

    #[test]
    fn run_help_routes_to_help() {
        let mut buf = Vec::new();
        run(&args(&["rx", "-h"]), &mut buf).expect("help");
        assert!(String::from_utf8_lossy(&buf).contains("RX"));
    }

    #[test]
    fn run_version_routes_to_version() {
        let mut buf = Vec::new();
        run(&args(&["rx", "-v"]), &mut buf).expect("version");
    }

    #[test]
    fn run_unknown_flag_returns_invalid_args() {
        let mut buf = Vec::new();
        let result = run(&args(&["rx", "-x"]), &mut buf);
        assert!(result.is_err());
    }

    #[test]
    fn run_workflow_dry_run_succeeds() {
        let mut buf = Vec::new();
        run(
            &args(&[
                "rx",
                "--dry-run",
                "-y",
                "--hostname",
                "testhost",
                "--repo",
                "/tmp",
            ]),
            &mut buf,
        )
        .expect("dry run");
    }

    #[test]
    fn run_workflow_invalid_config_errors() {
        let mut buf = Vec::new();
        let result = run(
            &args(&[
                "rx",
                "--dry-run",
                "-y",
                "--hostname",
                "testhost",
                "--repo",
                "",
            ]),
            &mut buf,
        );
        assert!(result.is_err());
    }
}
