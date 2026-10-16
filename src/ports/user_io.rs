use crate::config::Config;
use crate::error::Error;

/// Driven port: user-facing presentation and interaction.
///
/// Production wires [`crate::adapters::terminal::TerminalIo`]; tests inject
/// mock implementations.
pub trait UserIo {
    /// Print a banner title.
    fn title(&self, text: &str) -> Result<(), Error>;
    /// Prompt for a yes/no confirmation, returning `default` on empty input.
    fn confirm(&self, default: bool, msg: Option<&str>) -> Result<bool, Error>;
    /// Print an error message.
    fn error(&self, msg: &str) -> Result<(), Error>;
    /// Print a success message.
    fn success(&self, msg: &str) -> Result<(), Error>;
    /// Print command output verbatim (no extra newline).
    fn output(&self, text: &str) -> Result<(), Error>;
    /// Print the current configuration.
    fn config(&self, config: &Config) -> Result<(), Error>;
}
