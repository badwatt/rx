use crate::error::Error;

/// Driven port: execute external commands.
///
/// Production wires [`crate::adapters::process::ProcessRunner`]; tests inject
/// mock implementations.
pub trait CommandRunner {
    /// Run a command via argv list, inheriting stdio, and return its exit code.
    fn run(&self, cmd: &[String]) -> Result<i32, Error>;
    /// Run a command via argv list and capture its exit code and stdout.
    fn capture(&self, cmd: &[String]) -> Result<(i32, String), Error>;
}
