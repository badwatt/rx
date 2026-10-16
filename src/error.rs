use std::fmt;

/// Application-level error type.
///
/// Uses a manual `Display` + `Error` impl to keep the dependency tree empty.
/// `From<std::io::Error>` enables the `?` operator for IO fallibility.
#[derive(Debug)]
pub enum Error {
    Io(std::io::Error),
    GitPullFailed,
    FlakeUpdateFailed,
    RebuildFailed,
    CleanupFailed,
    GitAddFailed,
    InvalidArgs,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Io(e) => write!(f, "IO error: {e}"),
            Error::GitPullFailed => write!(f, "Failed to pull changes"),
            Error::FlakeUpdateFailed => write!(f, "Failed to update flake inputs"),
            Error::RebuildFailed => write!(f, "Nixos rebuild failed"),
            Error::CleanupFailed => write!(f, "Cleanup failed"),
            Error::GitAddFailed => write!(f, "Failed to add changes to the stage"),
            Error::InvalidArgs => write!(f, "Invalid arguments"),
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Error::Io(e) => Some(e),
            Error::GitPullFailed
            | Error::FlakeUpdateFailed
            | Error::RebuildFailed
            | Error::CleanupFailed
            | Error::GitAddFailed
            | Error::InvalidArgs => None,
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
        assert!(std::error::Error::source(&Error::FlakeUpdateFailed).is_none());
        assert!(std::error::Error::source(&Error::RebuildFailed).is_none());
        assert!(std::error::Error::source(&Error::CleanupFailed).is_none());
        assert!(std::error::Error::source(&Error::GitAddFailed).is_none());
        assert!(std::error::Error::source(&Error::InvalidArgs).is_none());
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
