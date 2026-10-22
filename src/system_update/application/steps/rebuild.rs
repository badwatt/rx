use crate::domain::config::Config;
use crate::domain::error::Error;
use crate::system_update::ports::nix::Nix;
use crate::system_update::ports::user_io::UserIo;

/// Rebuild and switch the NixOS system for `hostname`.
pub(crate) fn run(
    config: &Config,
    hostname: &str,
    nix: &dyn Nix,
    ui: &dyn UserIo,
) -> Result<(), Error> {
    ui.title("NixOS Rebuild")?;
    nix.rebuild(&config.repo, hostname, config.build_options())?;
    Ok(())
}
