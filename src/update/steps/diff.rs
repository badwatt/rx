use crate::commands;
use crate::error::Error;
use crate::ports::command::CommandRunner;
use crate::ports::user_io::UserIo;

/// Show the closure diff between the previous and current generation.
pub fn run(cmd: &dyn CommandRunner, ui: &dyn UserIo) -> Result<(), Error> {
    ui.title("Nix Diff")?;
    let (_, output) = cmd.capture(&commands::nix_diff_closures())?;
    ui.output(&commands::tail_from_last_version(&output))?;
    Ok(())
}
