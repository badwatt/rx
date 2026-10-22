//! Driving adapter: command-line interface.
//!
//! Parses arguments into an [`Intent`]; the composition root (`main`) wires
//! the ports and runs the workflow.

pub(crate) mod completions;
pub(crate) mod defaults;
pub(crate) mod help;
pub(crate) mod hostname;
pub(crate) mod parse;

use crate::cli::completions::Shell;
use crate::domain::config::Config;

/// Result of interpreting command-line arguments.
#[derive(Debug, PartialEq)]
pub(crate) enum Intent {
    Help,
    Version,
    Completions(Shell),
    Run(Config),
    Invalid,
}
