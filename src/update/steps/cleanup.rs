use crate::commands;
use crate::config::Config;
use crate::error::Error;
use crate::ports::command::CommandRunner;
use crate::ports::user_io::UserIo;

/// Delete old generations, keeping `config.keep`.
pub fn run(config: &Config, cmd: &dyn CommandRunner, _ui: &dyn UserIo) -> Result<(), Error> {
    cmd.run(&commands::nix_keep(config.keep), true)?;
    Ok(())
}
