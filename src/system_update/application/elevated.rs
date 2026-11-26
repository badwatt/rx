//! Running an elevated command with the sudo prompt announced.

use crate::domain::error::Error;
use crate::system_update::ports::nix::Nix;
use crate::system_update::ports::user_io::UserIo;
use crate::terminal::osc::State;

/// Run `command`, reporting `blocked:auth` while sudo asks for a password and
/// `working` for the command itself.
///
/// The prompt is pulled forward with `sudo -v`, so `blocked` covers exactly the
/// password question instead of the whole command: `nixos-rebuild -ng` builds
/// for minutes before its activation elevates. A build longer than sudo's
/// timestamp timeout can still ask again, and that prompt is not announced.
///
/// A dry run never touches sudo: no probe, no `sudo -v`, no keep-alive thread.
pub(super) fn run_elevated(
    nix: &dyn Nix,
    ui: &dyn UserIo,
    dry_run: bool,
    working: &str,
    command: &mut dyn FnMut() -> Result<(), Error>,
) -> Result<(), Error> {
    if !dry_run && nix.password_pending()? {
        ui.progress(State::BlockedAuth, "Waiting for your sudo password")?;
        nix.refresh_credentials()?;
    }
    ui.progress(State::Working, working)?;
    command()
}
