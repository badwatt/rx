use crate::domain::config::Config;
use crate::domain::error::Error;
use crate::system_update::ports::nix::Nix;
use crate::system_update::ports::user_io::UserIo;

/// Delete old generations (keeping `config.keep`) and free unreferenced paths.
pub(crate) fn run(config: &Config, nix: &dyn Nix, _ui: &dyn UserIo) -> Result<(), Error> {
    nix.prune(config.keep)?;
    nix.collect_garbage()?;
    Ok(())
}
