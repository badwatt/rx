use crate::error::Error;

/// Driven port: execute external commands.
///
/// Production wires [`crate::adapters::process::ProcessRunner`]; tests inject
/// mock implementations.
pub trait CommandRunner {
    /// Run a command via argv list (no shell, no injection risk).
    fn run(&self, cmd: &[String], output: bool) -> Result<i32, Error>;
    /// Run a shell pipeline string (only for commands requiring pipes).
    fn run_pipeline(&self, cmd: &str, output: bool) -> Result<i32, Error>;
}
