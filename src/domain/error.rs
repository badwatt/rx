use std::fmt;

/// Application-level error type.
///
/// Uses a manual `Display` + `Error` impl to keep the dependency tree empty.
/// `From<std::io::Error>` enables the `?` operator for IO fallibility.
#[derive(Debug)]
pub(crate) enum Error {
    Io(std::io::Error),
    GitPullFailed,
    GitStatusFailed,
    NotAGitRepo,
    FlakeUpdateFailed,
    RebuildFailed,
    HostnameUnknown,
    CleanupFailed,
    GitAddFailed,
    InvalidArgs,
    InvalidConfig(&'static str),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Io(e) => write!(f, "IO error: {e}"),
            Error::GitPullFailed => write!(f, "Failed to pull changes"),
            Error::GitStatusFailed => write!(f, "Failed to read repository status"),
            Error::NotAGitRepo => write!(f, "Repository is not a git working tree"),
            Error::FlakeUpdateFailed => write!(f, "Failed to update flake inputs"),
            Error::RebuildFailed => write!(f, "Nixos rebuild failed"),
            Error::HostnameUnknown => {
                write!(f, "Could not determine hostname; pass --hostname")
            }
            Error::CleanupFailed => write!(f, "Cleanup failed"),
            Error::GitAddFailed => write!(f, "Failed to add changes to the stage"),
            Error::InvalidArgs => write!(f, "Invalid arguments"),
            Error::InvalidConfig(msg) => write!(f, "{msg}"),
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Error::Io(e) => Some(e),
            Error::GitPullFailed
            | Error::GitStatusFailed
            | Error::NotAGitRepo
            | Error::FlakeUpdateFailed
            | Error::RebuildFailed
            | Error::HostnameUnknown
            | Error::CleanupFailed
            | Error::GitAddFailed
            | Error::InvalidArgs
            | Error::InvalidConfig(_) => None,
        }
    }
}

impl From<std::io::Error> for Error {
    fn from(e: std::io::Error) -> Self {
        Error::Io(e)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn display_io_error_contains_io_prefix() {
        let err = Error::Io(std::io::Error::other("test"));
        assert!(err.to_string().contains("IO error"));
    }

    #[test]
    fn display_git_pull_failed_matches_literal() {
        assert_eq!(Error::GitPullFailed.to_string(), "Failed to pull changes");
    }

    #[test]
    fn display_git_status_failed_matches_literal() {
        assert_eq!(
            Error::GitStatusFailed.to_string(),
            "Failed to read repository status"
        );
    }

    #[test]
    fn display_not_a_git_repo_matches_literal() {
        assert_eq!(
            Error::NotAGitRepo.to_string(),
            "Repository is not a git working tree"
        );
    }

    #[test]
    fn display_hostname_unknown_matches_literal() {
        assert_eq!(
            Error::HostnameUnknown.to_string(),
            "Could not determine hostname; pass --hostname"
        );
    }

    #[test]
    fn display_flake_update_failed_matches_literal() {
        assert_eq!(
            Error::FlakeUpdateFailed.to_string(),
            "Failed to update flake inputs"
        );
    }

    #[test]
    fn display_rebuild_failed_matches_literal() {
        assert_eq!(Error::RebuildFailed.to_string(), "Nixos rebuild failed");
    }

    #[test]
    fn display_cleanup_failed_matches_literal() {
        assert_eq!(Error::CleanupFailed.to_string(), "Cleanup failed");
    }

    #[test]
    fn display_git_add_failed_matches_literal() {
        assert_eq!(
            Error::GitAddFailed.to_string(),
            "Failed to add changes to the stage"
        );
    }

    #[test]
    fn display_invalid_config_matches_message() {
        assert_eq!(
            Error::InvalidConfig("repo path cannot be empty").to_string(),
            "repo path cannot be empty"
        );
    }

    #[test]
    fn display_invalid_args_matches_literal() {
        assert_eq!(Error::InvalidArgs.to_string(), "Invalid arguments");
    }

    #[test]
    fn source_io_error_yields_some() {
        let err = Error::Io(std::io::Error::other("test"));
        assert!(std::error::Error::source(&err).is_some());
    }

    #[test]
    fn source_non_io_variants_yield_none() {
        assert!(std::error::Error::source(&Error::GitPullFailed).is_none());
        assert!(std::error::Error::source(&Error::GitStatusFailed).is_none());
        assert!(std::error::Error::source(&Error::NotAGitRepo).is_none());
        assert!(std::error::Error::source(&Error::FlakeUpdateFailed).is_none());
        assert!(std::error::Error::source(&Error::RebuildFailed).is_none());
        assert!(std::error::Error::source(&Error::HostnameUnknown).is_none());
        assert!(std::error::Error::source(&Error::CleanupFailed).is_none());
        assert!(std::error::Error::source(&Error::GitAddFailed).is_none());
        assert!(std::error::Error::source(&Error::InvalidArgs).is_none());
        assert!(std::error::Error::source(&Error::InvalidConfig("x")).is_none());
    }

    #[test]
    fn from_io_error_produces_io_variant() {
        let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "file");
        let err: Error = io_err.into();
        assert_eq!(
            std::mem::discriminant(&err),
            std::mem::discriminant(&Error::Io(std::io::Error::other("")))
        );
    }
}
