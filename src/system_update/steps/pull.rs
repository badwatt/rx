use crate::domain::error::Error;
use crate::ports::repo::Repo;
use crate::ports::user_io::UserIo;

/// Ensure `repo` is a git working tree, then pull the latest changes.
pub(crate) fn run(repo: &str, vcs: &dyn Repo, ui: &dyn UserIo) -> Result<(), Error> {
    ui.title("Git Pull")?;
    if !vcs.is_repo(repo)? {
        return Err(Error::NotAGitRepo);
    }
    let result = vcs.pull(repo);
    if let Err(Error::GitPullFailed) = &result {
        ui.error("Failed to pull changes")?;
    }
    result
}
