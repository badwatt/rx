use std::io::Write;

use crate::domain::error::Error;
use crate::shared::ansi::{self, Palette};

/// Project version, read from the repository-root `VERSION` file (single source
/// of truth). `Cargo.toml` only carries a placeholder version.
const VERSION_FILE: &str = include_str!("../../VERSION");

/// Version string without the file's trailing newline.
fn version() -> &'static str {
    VERSION_FILE.trim()
}

/// Print the help banner.
pub(crate) fn print_help(writer: &mut dyn Write) -> Result<(), Error> {
    ansi::write_flush(
        writer,
        "\n*****************************************************\n RX - A simple CLI tool to update your NixOS system\n*****************************************************\n-r, --repo        : set repo path (default is $HOME/.dotfiles)\n-n, --hostname    : set hostname (default is OS hostname)\n-k, --keep        : set generations to keep (default is 10)\n-j, --jobs        : cap Nix max-jobs for the rebuild\n-c, --cores       : cap Nix cores for the rebuild\n-u, --update      : set update to true (default is false)\n-d, --diff        : set diff to true (default is false)\n-y, --yes         : assume yes for all prompts (default is false)\n    --dry-run     : print commands without executing them\n    --no-color    : disable colored output\n-h, --help        : Display this help message\n-v, --version     : Display the current version\n\n",
    )?;
    Ok(())
}

/// Print the version line.
pub(crate) fn print_version(writer: &mut dyn Write, palette: Palette) -> Result<(), Error> {
    ansi::write_flush(
        writer,
        &format!(
            "{}RX version: {}{}\n",
            palette.yellow,
            version(),
            palette.reset
        ),
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
    fn print_version_prints_file_version() {
        let mut buf = Vec::new();
        assert!(print_version(&mut buf, Palette::PLAIN).is_ok());
        assert_eq!(
            String::from_utf8_lossy(&buf),
            format!("RX version: {}\n", version())
        );
    }

    #[test]
    fn print_help_error_propagation() {
        let mut writer = crate::shared::test_helpers::FailingWriter;
        assert!(print_help(&mut writer).is_err());
    }

    #[test]
    fn print_version_error_propagation() {
        let mut writer = crate::shared::test_helpers::FailingWriter;
        assert!(print_version(&mut writer, Palette::COLOR).is_err());
    }
}
