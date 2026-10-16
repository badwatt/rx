use crate::commands;
use crate::error::Error;
use crate::ports::command::CommandRunner;
use crate::ports::user_io::UserIo;

/// Detect changes and, if confirmed, stage them in `repo`.
pub fn run(repo: &str, cmd: &dyn CommandRunner, ui: &dyn UserIo) -> Result<(), Error> {
    let diff_status = cmd.run(&commands::git_diff(repo), false)?;
    if diff_status != 1 {
        return Ok(());
    }
    ui.title("Git Changes")?;
    cmd.run(&commands::git_status(repo), true)?;
    if !ui.confirm(true, Some("Do you want to add these changes to the stage?"))? {
        ui.error("Changes not added to stage")?;
        return Ok(());
    }
    match cmd.run(&commands::git_add(repo), true) {
        Ok(_) => ui.success("Changes added to git stage successfully")?,
        Err(e) => ui.error(&format!("Failed to add changes to the stage: {e}"))?,
    }
    Ok(())
}
