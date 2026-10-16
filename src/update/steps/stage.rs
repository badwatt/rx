use crate::commands;
use crate::error::Error;
use crate::ports::command::CommandRunner;
use crate::ports::user_io::UserIo;

/// Detect changes (including untracked files) and, if confirmed, stage them.
pub(crate) fn run(
    repo: &str,
    yes: bool,
    cmd: &dyn CommandRunner,
    ui: &dyn UserIo,
) -> Result<(), Error> {
    let (status, output) = cmd.capture(&commands::git_status(repo))?;
    if status != 0 {
        return Err(Error::GitStatusFailed);
    }
    if output.trim().is_empty() {
        return Ok(());
    }
    ui.title("Git Changes")?;
    ui.output(&output)?;
    if !yes && !ui.confirm(true, Some("Do you want to add these changes to the stage?"))? {
        ui.error("Changes not added to stage")?;
        return Ok(());
    }
    match cmd.run(&commands::git_add(repo)) {
        Ok(0) => ui.success("Changes added to git stage successfully")?,
        Ok(_) => {
            ui.error("Failed to add changes to the stage")?;
            return Err(Error::GitAddFailed);
        }
        Err(e) => {
            ui.error(&format!("Failed to add changes to the stage: {e}"))?;
            return Err(Error::GitAddFailed);
        }
    }
    Ok(())
}
