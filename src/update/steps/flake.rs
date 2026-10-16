use crate::commands;
use crate::config::Config;
use crate::error::Error;
use crate::ports::command::CommandRunner;
use crate::ports::user_io::UserIo;

/// Update the flake inputs for `config.repo`.
pub fn run(config: &Config, cmd: &dyn CommandRunner, ui: &dyn UserIo) -> Result<(), Error> {
    ui.title("Nix Update")?;
    cmd.run(&commands::nix_update(&config.repo), true)?;
    Ok(())
}
