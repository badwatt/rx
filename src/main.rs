mod cli;
mod domain;
mod infra;
mod shared;
mod system_update;

use cli::Intent;
use cli::entry::{exit_code_for, palettes, report_error};
use domain::config::Config;
use domain::error::Error;
use infra::command::CommandRunner;
use infra::dry_run::DryRunRunner;
use infra::process::ProcessRunner;
use shared::ansi::Palette;
use std::env;
use std::io::{Write, stderr, stdout};
use system_update::adapters::git::GitRepo;
use system_update::adapters::nix::NixCli;
use system_update::adapters::terminal::TerminalIo;

fn main() {
    let args: Vec<String> = env::args().collect();
    let mut out = stdout();
    let mut err = stderr();
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
