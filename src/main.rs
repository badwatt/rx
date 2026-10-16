mod adapters;
mod ansi;
mod cli;
mod commands;
mod config;
mod error;
mod ports;
mod update;

#[cfg(test)]
mod test_helpers;

use adapters::process::ProcessRunner;
use adapters::terminal::TerminalIo;
use std::{env, io::stdout};

fn exit_code_for(error: &error::Error) -> i32 {
    match error {
        error::Error::InvalidArgs => 2,
        _ => 1,
    }
}

fn main() {
    let args: Vec<String> = env::args().collect();
    let mut out = stdout();
    let runner = ProcessRunner;
    let ui = TerminalIo;
    if let Err(e) = cli::dispatch(&args, &mut out, &runner, &ui) {
        eprintln!("Error: {e}");
        std::process::exit(exit_code_for(&e));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exit_code_invalid_args_is_2() {
        assert_eq!(exit_code_for(&crate::error::Error::InvalidArgs), 2);
    }

    #[test]
    fn exit_code_git_pull_failed_is_1() {
        assert_eq!(exit_code_for(&crate::error::Error::GitPullFailed), 1);
    }

    #[test]
    fn exit_code_io_error_is_1() {
        assert_eq!(
            exit_code_for(&crate::error::Error::Io(std::io::Error::other("x"))),
            1
        );
    }
}
