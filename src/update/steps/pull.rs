use crate::commands;
use crate::error::Error;
use crate::ports::command::CommandRunner;
use crate::ports::user_io::UserIo;

/// Verify `repo` is a git working tree, then pull the latest changes.
pub(crate) fn run(repo: &str, cmd: &dyn CommandRunner, ui: &dyn UserIo) -> Result<(), Error> {
    ui.title("Git Pull")?;
    let (rev_status, _) = cmd.capture(&commands::git_rev_parse(repo))?;
    if rev_status != 0 {
        return Err(Error::NotAGitRepo);
    }
    let status = cmd.run(&commands::git_pull(repo))?;
    if status != 0 {
        ui.error("Failed to pull changes")?;
        return Err(Error::GitPullFailed);
    }
    Ok(())
}
