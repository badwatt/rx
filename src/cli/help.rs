use std::io::Write;

use crate::domain::error::Error;
use crate::shared::ansi;

const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Print the help banner.
pub(crate) fn print_help(writer: &mut dyn Write) -> Result<(), Error> {
    ansi::write_flush(
        writer,
        "\n*****************************************************\n RX - A simple CLI tool to update your nixos system\n*****************************************************\n-r, --repo        : set repo path (default is $HOME/.dotfiles)\n-n, --hostname    : set hostname (default is OS hostname)\n-k, --keep        : set generations to keep (default is 10)\n-j, --jobs        : cap Nix max-jobs for the rebuild\n-c, --cores       : cap Nix cores for the rebuild\n-u, --update      : set update to true (default is false)\n-d, --diff        : set diff to true (default is false)\n-y, --yes         : assume yes for all prompts (default is false)\n    --dry-run     : print commands without executing them\n-h, --help        : Display this help message\n-v, --version     : Display the current version\n\n",
    )?;
    Ok(())
}

/// Print the version line.
pub(crate) fn print_version(writer: &mut dyn Write) -> Result<(), Error> {
    ansi::write_flush(
        writer,
        &format!("{}RX version: {}{}\n", ansi::YELLOW, VERSION, ansi::RESET),
    )?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn print_help_contains_rx() {
        let mut buf = Vec::new();
        assert!(print_help(&mut buf).is_ok());
        assert!(String::from_utf8_lossy(&buf).contains("RX"));
    }

    #[test]
    fn print_version_contains_semver() {
        let mut buf = Vec::new();
        assert!(print_version(&mut buf).is_ok());
        assert!(String::from_utf8_lossy(&buf).contains(VERSION));
    }

    #[test]
    fn print_help_error_propagation() {
        let mut writer = crate::shared::test_helpers::FailingWriter;
        assert!(print_help(&mut writer).is_err());
    }

    #[test]
    fn print_version_error_propagation() {
        let mut writer = crate::shared::test_helpers::FailingWriter;
        assert!(print_version(&mut writer).is_err());
    }
}
