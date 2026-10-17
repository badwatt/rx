use crate::domain::error::Error;

/// Adapter-internal seam: run external commands via argv.
///
/// Not a domain port — only the `git`/`nix` adapters use it, so `--dry-run`
/// can decorate a single implementation instead of every semantic port.
pub(crate) trait CommandRunner {
    /// Run a command inheriting stdio and return its exit code.
    fn run(&self, cmd: &[String]) -> Result<i32, Error>;
    /// Run a command and capture its exit code and stdout.
    fn capture(&self, cmd: &[String]) -> Result<(i32, String), Error>;
    /// Run a command with stdout/stderr discarded, returning its exit code.
    fn probe(&self, cmd: &[String]) -> Result<i32, Error>;
}
