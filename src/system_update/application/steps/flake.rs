use crate::domain::config::Config;
use crate::domain::error::Error;
use crate::system_update::ports::nix::Nix;
use crate::system_update::ports::user_io::UserIo;

/// Update the flake inputs for `config.repo`.
pub(crate) fn run(config: &Config, nix: &dyn Nix, ui: &dyn UserIo) -> Result<(), Error> {
    ui.title("Nix Update")?;
    nix.update(&config.repo)?;
    Ok(())
}
