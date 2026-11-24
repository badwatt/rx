use super::run_elevated;
use crate::domain::config::Config;
use crate::domain::error::Error;
use crate::system_update::ports::nix::Nix;
use crate::system_update::ports::user_io::UserIo;

/// Delete old generations (keeping `config.keep`) and free unreferenced paths.
pub(crate) fn run(config: &Config, nix: &dyn Nix, ui: &dyn UserIo) -> Result<(), Error> {
    run_elevated(nix, ui, config.dry_run, "Cleaning up old generations", &mut || {
        nix.prune(config.keep)?;
        nix.collect_garbage()
    })
}
