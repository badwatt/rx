//! Composition root for `rx`.
//!
//! Parses the arguments, selects the adapters (process or dry-run) and runs
//! the update workflow. Process policy lives in `cli::entry`; every external
//! command lives in the adapters.

mod cli;
mod domain;
mod infra;
mod system_update;
mod terminal;

#[cfg(test)]
mod test_support;

use cli::Intent;
use cli::entry::{exit_code_for, palettes, report_error};
use domain::config::Config;
use domain::error::Error;
use infra::command::CommandRunner;
use infra::dry_run::DryRunRunner;
use infra::process::ProcessRunner;
use std::env;
use std::io::{Write, stderr, stdout};
use system_update::adapters::git::GitRepo;
use system_update::adapters::nix::NixCli;
use system_update::adapters::terminal::TerminalIo;
use terminal::ansi::Palette;

fn main() {
    let Ok(args) = utf8_args(env::args_os().collect()) else {
        let (_, err_palette) = palettes(None);
        // Best effort: exiting anyway.
        let _ = writeln!(
            stderr(),
            "{}Error: arguments must be valid UTF-8{}",
            err_palette.red,
            err_palette.reset
        );
        std::process::exit(exit_code_for(&Error::InvalidArgs));
    };
    let mut out = stdout();
    let mut err = stderr();
    let color = cli::parse::color_arg(&args);
    let (out_palette, err_palette) = palettes(color);
    if let Err(e) = run(&args, &mut out, &mut err, out_palette) {
        report_error(&mut err, err_palette, &e);
        std::process::exit(exit_code_for(&e));
    }
}

/// Convert the process arguments, rejecting non-UTF-8 instead of panicking.
///
/// `env::args()` panics on an argument that is not valid UTF-8; a CLI must
/// report that as invalid input instead.
fn utf8_args(args: Vec<std::ffi::OsString>) -> Result<Vec<String>, std::ffi::OsString> {
    args.into_iter()
        .map(std::ffi::OsString::into_string)
        .collect()
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
    let ui = TerminalIo::new(out_palette, config.dry_run);
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
    fn run_parse_error_goes_to_err_writer() {
        let mut out = Vec::new();
        let mut err = Vec::new();
        let result = super::run(&args(&["rx", "-x"]), &mut out, &mut err, Palette::PLAIN);
        assert!(result.is_err());
        assert_eq!(String::from_utf8_lossy(&out), "");
        assert!(String::from_utf8_lossy(&err).contains("Unknown flag"));
    }
    #[test]
    fn utf8_args_keeps_valid_arguments() {
        let args = utf8_args(vec![std::ffi::OsString::from("rx")]).expect("valid args");
        assert_eq!(args, vec![String::from("rx")]);
    }

    #[test]
    fn utf8_args_rejects_a_non_utf8_argument() {
        use std::os::unix::ffi::OsStringExt;
        let args = vec![
            std::ffi::OsString::from("rx"),
            std::ffi::OsString::from_vec(vec![0xff]),
        ];
        assert!(utf8_args(args).is_err());
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
