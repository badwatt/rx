use std::fmt;

/// Default dotfiles repository path (before `~` expansion).
pub const DEFAULT_REPO: &str = "~/.dotfiles";
/// Default number of generations to keep during cleanup.
pub const DEFAULT_KEEP: u8 = 10;

/// Parsed CLI configuration.
#[expect(
    clippy::struct_excessive_bools,
    reason = "independent CLI flags are naturally boolean"
)]
#[derive(Debug, PartialEq, Clone)]
pub struct Config {
    pub repo: String,
    pub hostname: String,
    pub keep: u8,
    pub update: bool,
    pub diff: bool,
    /// Assume "yes" for all confirmation prompts.
    pub yes: bool,
    /// Print commands instead of executing them.
    pub dry_run: bool,
    /// Use `nixos-rebuild --elevate=sudo --no-reexec` (NixOS >= 25.11).
    pub elevate: bool,
}

impl fmt::Display for Config {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Config {{ repo: {}, hostname: {}, keep: {}, update: {}, diff: {} }}",
            self.repo, self.hostname, self.keep, self.update, self.diff
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> Config {
        Config {
            repo: String::from("repo"),
            hostname: String::from("host"),
            keep: 5,
            update: true,
            diff: false,
            yes: false,
            dry_run: false,
            elevate: false,
        }
    }

    #[test]
    fn config_display_contains_repo() {
        assert!(sample().to_string().contains("repo"));
    }

    #[test]
    fn config_display_full_format() {
        assert_eq!(
            sample().to_string(),
            "Config { repo: repo, hostname: host, keep: 5, update: true, diff: false }"
        );
    }
}
