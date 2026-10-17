use crate::domain::config::Config;
use crate::domain::error::Error;
use crate::ports::nix::Nix;
use crate::ports::user_io::UserIo;

/// Update the flake inputs for `config.repo`.
pub(crate) fn run(config: &Config, nix: &dyn Nix, ui: &dyn UserIo) -> Result<(), Error> {
    ui.title("Nix Update")?;
    nix.update(&config.repo)?;
    Ok(())
}
