use crate::domain::error::Error;

/// Driven port: version-control operations on the dotfiles repository.
pub(crate) trait Repo {
    /// Whether `repo` is a git working tree.
    fn is_repo(&self, repo: &str) -> Result<bool, Error>;
    /// Pull the latest changes.
    fn pull(&self, repo: &str) -> Result<(), Error>;
    /// `git status --porcelain` output (empty when clean).
    fn changes(&self, repo: &str) -> Result<String, Error>;
    /// Stage all changes.
    fn add_all(&self, repo: &str) -> Result<(), Error>;
}
