use crate::domain::error::Error;
use crate::system_update::ports::repo::Repo;
use crate::system_update::ports::user_io::UserIo;

/// Detect changes (including untracked files) and, if confirmed, stage them.
///
/// Returns `false` when the user declines staging: nothing was added, so the
/// workflow must not go on to rebuild a stale flake.
pub(crate) fn run(repo: &str, yes: bool, vcs: &dyn Repo, ui: &dyn UserIo) -> Result<bool, Error> {
    let output = vcs.changes(repo)?;
    if output.trim().is_empty() {
        return Ok(true);
    }
    ui.title("Git Changes")?;
    ui.output(&output)?;
    if !yes && !ui.confirm(true, Some("Do you want to add these changes to the stage?"))? {
        // Best-effort: the workflow is ending anyway.
        let _ = ui.output("Changes not added to stage; aborting\n");
        return Ok(false);
    }
    vcs.add_all(repo)?;
    ui.success("Changes added to git stage successfully")?;
    Ok(true)
}
