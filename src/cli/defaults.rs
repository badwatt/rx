use crate::cli::hostname;
use crate::cli::nixos;
use crate::config::{Config, DEFAULT_KEEP, DEFAULT_REPO};

/// Build a fresh [`Config`] from defaults: repo `~/.dotfiles`, hostname resolved
/// from the OS, keep `10`, update/diff `false`, elevation detected from the
/// running NixOS release.
pub(crate) fn default_config() -> Config {
    Config {
        repo: expand_tilde(DEFAULT_REPO),
        hostname: hostname::resolve(),
        keep: DEFAULT_KEEP,
        update: false,
        diff: false,
        yes: false,
        dry_run: false,
        elevate: nixos::supports_elevate(),
    }
}

/// Expand `~` at the start of a path to the user's home directory.
/// Returns the path unchanged if `~` is not the leading character
/// or if `$HOME` is not set.
pub(crate) fn expand_tilde(path: &str) -> String {
    if let Some(rest) = path.strip_prefix('~')
        && let Ok(home) = std::env::var("HOME")
    {
        format!("{home}{rest}")
    } else {
        path.to_owned()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn expand_tilde_expands_home() {
        let home = std::env::var("HOME").expect("HOME set");
        assert_eq!(expand_tilde("~/.dotfiles"), format!("{home}/.dotfiles"));
    }

    #[test]
    fn expand_tilde_leaves_absolute_path() {
        assert_eq!(expand_tilde("/absolute/path"), "/absolute/path");
    }

    #[test]
    fn expand_tilde_leaves_relative_path() {
        assert_eq!(expand_tilde("relative/path"), "relative/path");
    }

    #[test]
    fn expand_tilde_tilde_only() {
        let home = std::env::var("HOME").expect("HOME set");
        assert_eq!(expand_tilde("~"), home);
    }

    #[test]
    fn default_config_repo_and_keep() {
        let config = default_config();
        let home = std::env::var("HOME").expect("HOME set");
        assert_eq!(config.repo, format!("{home}/.dotfiles"));
        assert_eq!(config.keep, DEFAULT_KEEP);
    }

    #[test]
    fn default_config_hostname_is_non_empty() {
        assert_ne!(default_config().hostname, "");
    }
}
