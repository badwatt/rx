//! Driving adapter: command-line interface.
//!
//! Parses arguments into an [`Intent`] and dispatches it to the update
//! workflow or to help/version output.

pub mod defaults;
pub mod help;
pub mod hostname;
pub mod parse;

use std::io::Write;

use crate::config::Config;
use crate::error::Error;
use crate::ports::command::CommandRunner;
use crate::ports::user_io::UserIo;
use crate::update;

/// Result of interpreting command-line arguments.
#[derive(Debug, PartialEq)]
pub enum Intent {
    Help,
    Version,
    Run(Config),
    Invalid,
}

/// Dispatch parsed arguments to help, version, workflow, or error.
pub fn dispatch(
    args: &[String],
    writer: &mut dyn Write,
    cmd: &dyn CommandRunner,
    ui: &dyn UserIo,
) -> Result<(), Error> {
    match parse::parse(args, writer) {
        Intent::Help => help::print_help(writer),
        Intent::Version => help::print_version(writer),
        Intent::Invalid => Err(Error::InvalidArgs),
        Intent::Run(config) => update::run(&config, cmd, ui),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct OkCmd;

    impl CommandRunner for OkCmd {
        fn run(&self, _: &[String], _: bool) -> Result<i32, Error> {
            Ok(0)
        }
        fn run_pipeline(&self, _: &str, _: bool) -> Result<i32, Error> {
            Ok(0)
        }
    }

    struct OkUi;

    impl UserIo for OkUi {
        fn title(&self, _: &str) -> Result<(), Error> {
            Ok(())
        }
        fn confirm(&self, _: bool, _: Option<&str>) -> Result<bool, Error> {
            Ok(true)
        }
        fn error(&self, _: &str) -> Result<(), Error> {
            Ok(())
        }
        fn success(&self, _: &str) -> Result<(), Error> {
            Ok(())
        }
        fn message(&self, _: &str) -> Result<(), Error> {
            Ok(())
        }
        fn config(&self, _: &Config) -> Result<(), Error> {
            Ok(())
        }
    }

    fn args(from: &[&str]) -> Vec<String> {
        from.iter().copied().map(str::to_string).collect()
    }

    #[test]
    fn dispatch_help_flag_routes_to_help() {
        let mut buf = Vec::new();
        dispatch(&args(&["rx", "-h"]), &mut buf, &OkCmd, &OkUi).expect("help");
        assert!(String::from_utf8_lossy(&buf).contains("RX"));
    }

    #[test]
    fn dispatch_version_flag_routes_to_version() {
        let mut buf = Vec::new();
        dispatch(&args(&["rx", "-v"]), &mut buf, &OkCmd, &OkUi).expect("version");
    }

    #[test]
    fn dispatch_unknown_flag_returns_invalid_args() {
        let mut buf = Vec::new();
        let result = dispatch(&args(&["rx", "-x"]), &mut buf, &OkCmd, &OkUi);
        assert!(result.is_err());
        assert!(
            result
                .expect_err("invalid args")
                .to_string()
                .contains("Invalid arguments")
        );
    }

    #[test]
    fn dispatch_no_args_reaches_workflow() {
        let mut buf = Vec::new();
        dispatch(&args(&["rx"]), &mut buf, &OkCmd, &OkUi).expect("workflow");
    }

    #[test]
    fn dispatch_flags_reach_workflow() {
        let mut buf = Vec::new();
        dispatch(&args(&["rx", "-d", "-u"]), &mut buf, &OkCmd, &OkUi).expect("flags");
    }

    #[test]
    fn ok_adapters_cover_all_methods() {
        assert!(OkCmd.run(&[], false).is_ok());
        assert!(OkCmd.run_pipeline("true", false).is_ok());
        assert!(OkUi.title("t").is_ok());
        assert!(OkUi.confirm(true, None).is_ok());
        assert!(OkUi.error("e").is_ok());
        assert!(OkUi.success("s").is_ok());
        assert!(OkUi.message("m").is_ok());
        assert!(OkUi.config(&defaults::default_config()).is_ok());
    }
}
