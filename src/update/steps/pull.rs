use crate::commands;
use crate::error::Error;
use crate::ports::command::CommandRunner;
use crate::ports::user_io::UserIo;

/// Pull the latest changes in `repo`.
pub fn run(repo: &str, cmd: &dyn CommandRunner, ui: &dyn UserIo) -> Result<(), Error> {
    ui.title("Git Pull")?;
    let status = cmd.run(&commands::git_pull(repo), true)?;
    if status != 0 {
        ui.error("Failed to pull changes")?;
        return Err(Error::GitPullFailed);
    }
    Ok(())
}
