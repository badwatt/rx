use crate::commands;
use crate::config::Config;
use crate::error::Error;
use crate::ports::command::CommandRunner;
use crate::ports::user_io::UserIo;

/// Rebuild and switch the NixOS system for `config.hostname`.
pub(crate) fn run(config: &Config, cmd: &dyn CommandRunner, ui: &dyn UserIo) -> Result<(), Error> {
    if config.hostname == "unknown" {
        return Err(Error::HostnameUnknown);
    }
    ui.title("Nixos Rebuild")?;
    let status = cmd.run(&commands::nix_rebuild(
        &config.repo,
        &config.hostname,
        config.elevate,
    ))?;
    if status != 0 {
        return Err(Error::RebuildFailed);
    }
    Ok(())
}
