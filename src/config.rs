use std::fmt;

/// Default dotfiles repository path (before `~` expansion).
pub(crate) const DEFAULT_REPO: &str = "~/.dotfiles";
/// Default number of generations to keep during cleanup.
pub(crate) const DEFAULT_KEEP: u8 = 10;

/// Parsed CLI configuration.
#[expect(
    clippy::struct_excessive_bools,
    reason = "independent CLI flags are naturally boolean"
)]
#[derive(Debug, PartialEq, Clone)]
pub(crate) struct Config {
    pub(crate) repo: String,
    pub(crate) hostname: String,
    pub(crate) keep: u8,
    pub(crate) update: bool,
    pub(crate) diff: bool,
    /// Assume "yes" for all confirmation prompts.
    pub(crate) yes: bool,
    /// Print commands instead of executing them.
    pub(crate) dry_run: bool,
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

impl Config {
    /// Check invariants before running the workflow.
    /// Returns a user-facing message when the config is invalid.
    pub(crate) fn validate(&self) -> Option<&'static str> {
        if self.repo.is_empty() {
            return Some("repo path cannot be empty");
        }
        if self.hostname.is_empty() {
            return Some("hostname cannot be empty");
        }
        if self.keep == 0 {
            return Some("generations to keep must be > 0");
        }
        None
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

    #[test]
    fn validate_accepts_valid_config() {
        assert!(sample().validate().is_none());
    }

    #[test]
    fn validate_rejects_empty_repo() {
        let config = Config {
            repo: String::new(),
            ..sample()
        };
        assert_eq!(config.validate(), Some("repo path cannot be empty"));
    }

    #[test]
    fn validate_rejects_zero_keep() {
        let config = Config {
            keep: 0,
            ..sample()
        };
        assert_eq!(config.validate(), Some("generations to keep must be > 0"));
    }

    #[test]
    fn validate_rejects_empty_hostname() {
        let config = Config {
            hostname: String::new(),
            ..sample()
        };
        assert_eq!(config.validate(), Some("hostname cannot be empty"));
    }
}
