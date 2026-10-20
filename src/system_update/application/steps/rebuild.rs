use crate::domain::config::Config;
use crate::domain::error::Error;
use crate::system_update::ports::nix::Nix;
use crate::system_update::ports::user_io::UserIo;

/// Rebuild and switch the NixOS system for `config.hostname`.
pub(crate) fn run(config: &Config, nix: &dyn Nix, ui: &dyn UserIo) -> Result<(), Error> {
    if config.hostname == "unknown" {
        return Err(Error::HostnameUnknown);
    }
    ui.title("Nixos Rebuild")?;
    nix.rebuild(&config.repo, &config.hostname)?;
    Ok(())
}
