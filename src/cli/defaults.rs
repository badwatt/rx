use crate::cli::hostname;
use crate::domain::config::{Config, DEFAULT_KEEP, DEFAULT_REPO};

/// Build a fresh [`Config`] from defaults: repo `~/.dotfiles`, hostname resolved
/// from the OS (`None` when undetectable), keep `10`, update/diff/yes/dry-run
/// `false`.
pub(crate) fn default_config() -> Config {
    Config {
        repo: expand_tilde(DEFAULT_REPO),
        hostname: hostname::resolve(),
        keep: DEFAULT_KEEP,
        update: false,
        diff: false,
        yes: false,
        dry_run: false,
        jobs: None,
        cores: None,
        only: None,
    }
}

/// Expand a leading `~` (alone or followed by `/`) to the user's home
/// directory. `~user` forms are left untouched, as are paths not starting with
/// `~` and expansions when `$HOME` is unset.
pub(crate) fn expand_tilde(path: &str) -> String {
    expand_tilde_with(path, std::env::var("HOME").ok().as_deref())
}

/// Pure form of [`expand_tilde`] with the home directory injected.
fn expand_tilde_with(path: &str, home: Option<&str>) -> String {
    let Some(rest) = path.strip_prefix('~') else {
        return path.to_owned();
    };
    // Only `~` and `~/...` are home-relative; `~user` must not be spliced onto
    // $HOME.
    if !rest.is_empty() && !rest.starts_with('/') {
        return path.to_owned();
    }
    match home {
        Some(home) => format!("{home}{rest}"),
        None => path.to_owned(),
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
    fn expand_tilde_leaves_user_path() {
        // `~user` is not home-relative; must not be spliced onto $HOME.
        assert_eq!(expand_tilde("~bob"), "~bob");
        assert_eq!(expand_tilde("~bob/dots"), "~bob/dots");
    }

    #[test]
    fn expand_tilde_with_missing_home_leaves_path() {
        assert_eq!(expand_tilde_with("~/.dotfiles", None), "~/.dotfiles");
    }

    #[test]
    fn expand_tilde_with_home_expands_tilde_forms() {
        assert_eq!(expand_tilde_with("~/x", Some("/home/u")), "/home/u/x");
        assert_eq!(expand_tilde_with("~", Some("/home/u")), "/home/u");
        assert_eq!(expand_tilde_with("~bob", Some("/home/u")), "~bob");
    }

    #[test]
    fn default_config_repo_and_keep() {
        let config = default_config();
        let home = std::env::var("HOME").expect("HOME set");
        assert_eq!(config.repo, format!("{home}/.dotfiles"));
        assert_eq!(config.keep, DEFAULT_KEEP);
    }

    #[test]
    fn default_config_hostname_is_never_the_empty_string() {
        assert_ne!(default_config().hostname.as_deref(), Some(""));
    }
}
