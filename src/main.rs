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
use std::io::Write;
use std::{env, io::stdout};
use system_update::adapters::git::GitRepo;
use system_update::adapters::nix::NixCli;
use system_update::adapters::terminal::TerminalIo;

fn exit_code_for(error: &Error) -> i32 {
    match error {
        Error::InvalidArgs | Error::InvalidConfig(_) => 2,
        _ => 1,
    }
}

fn main() {
    let args: Vec<String> = env::args().collect();
    let mut out = stdout();
    if let Err(e) = run(&args, &mut out) {
        eprintln!("Error: {e}");
        std::process::exit(exit_code_for(&e));
    }
}

/// Parse the arguments and dispatch to help/version or the update workflow.
fn run(args: &[String], out: &mut dyn Write) -> Result<(), Error> {
    match cli::parse::parse(args, out) {
        Intent::Help => cli::help::print_help(out),
        Intent::Version => cli::help::print_version(out),
        Intent::Invalid => Err(Error::InvalidArgs),
        Intent::Run(config) => run_workflow(&config),
    }
}

/// Composition root for the workflow: wire the adapters and run the use case.
fn run_workflow(config: &Config) -> Result<(), Error> {
    let ui = TerminalIo;
    let runner: &dyn CommandRunner = if config.dry_run {
        &DryRunRunner
    } else {
        &ProcessRunner
    };
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
