use crate::commands;
use crate::config::Config;
use crate::error::Error;
use crate::ports::command::CommandRunner;
use crate::ports::user_io::UserIo;

/// Delete old generations (keeping `config.keep`) and free unreferenced paths.
pub(crate) fn run(config: &Config, cmd: &dyn CommandRunner, _ui: &dyn UserIo) -> Result<(), Error> {
    let keep_status = cmd.run(&commands::nix_keep(config.keep))?;
    if keep_status != 0 {
        return Err(Error::CleanupFailed);
    }
    let gc_status = cmd.run(&commands::nix_collect_garbage())?;
    if gc_status != 0 {
        return Err(Error::CleanupFailed);
    }
    Ok(())
}
