use crate::commands;
use crate::config::Config;
use crate::error::Error;
use crate::ports::command::CommandRunner;
use crate::ports::user_io::UserIo;

/// Rebuild and switch the NixOS system for `config.hostname`.
pub fn run(config: &Config, cmd: &dyn CommandRunner, ui: &dyn UserIo) -> Result<(), Error> {
    ui.title("Nixos Rebuild")?;
    cmd.run(&commands::nix_rebuild(&config.repo, &config.hostname), true)?;
    Ok(())
}
