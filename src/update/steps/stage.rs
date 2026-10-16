use crate::commands;
use crate::error::Error;
use crate::ports::command::CommandRunner;
use crate::ports::user_io::UserIo;

/// Detect changes (including untracked files) and, if confirmed, stage them.
pub fn run(repo: &str, cmd: &dyn CommandRunner, ui: &dyn UserIo) -> Result<(), Error> {
    let status = cmd.capture(&commands::git_status(repo))?;
    if status.trim().is_empty() {
        return Ok(());
    }
    ui.title("Git Changes")?;
    ui.output(&status)?;
    if !ui.confirm(true, Some("Do you want to add these changes to the stage?"))? {
        ui.error("Changes not added to stage")?;
        return Ok(());
    }
    match cmd.run(&commands::git_add(repo)) {
        Ok(_) => ui.success("Changes added to git stage successfully")?,
        Err(e) => ui.error(&format!("Failed to add changes to the stage: {e}"))?,
    }
    Ok(())
}
