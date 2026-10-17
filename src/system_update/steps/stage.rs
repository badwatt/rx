use crate::domain::error::Error;
use crate::ports::repo::Repo;
use crate::ports::user_io::UserIo;

/// Detect changes (including untracked files) and, if confirmed, stage them.
pub(crate) fn run(repo: &str, yes: bool, vcs: &dyn Repo, ui: &dyn UserIo) -> Result<(), Error> {
    let output = vcs.changes(repo)?;
    if output.trim().is_empty() {
        return Ok(());
    }
    ui.title("Git Changes")?;
    ui.output(&output)?;
    if !yes && !ui.confirm(true, Some("Do you want to add these changes to the stage?"))? {
        ui.error("Changes not added to stage")?;
        return Ok(());
    }
    match vcs.add_all(repo) {
        Ok(()) => ui.success("Changes added to git stage successfully")?,
        Err(Error::Io(e)) => {
            ui.error(&format!("Failed to add changes to the stage: {e}"))?;
            return Err(Error::GitAddFailed);
        }
        Err(_) => {
            ui.error("Failed to add changes to the stage")?;
            return Err(Error::GitAddFailed);
        }
    }
    Ok(())
}
