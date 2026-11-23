//! Individual steps of the update pipeline.

use crate::domain::error::Error;
use crate::shared::osc::State;
use crate::system_update::ports::nix::Nix;
use crate::system_update::ports::user_io::UserIo;

pub(crate) mod cleanup;
pub(crate) mod diff;
pub(crate) mod flake;
pub(crate) mod pull;
pub(crate) mod rebuild;
pub(crate) mod stage;

/// Run `command`, reporting a `blocked:auth` state when it will ask for a
/// sudo password and `working` otherwise.
///
/// The probe cannot tell *when* a command prompts (the `nixos-rebuild -ng`
/// build runs before its activation), so a pending prompt marks the whole
/// command as blocked, and the state goes back to `working` when it returns.
pub(super) fn run_elevated(
    nix: &dyn Nix,
    ui: &dyn UserIo,
    working: &str,
    command: &mut dyn FnMut() -> Result<(), Error>,
) -> Result<(), Error> {
    if !nix.password_pending()? {
        ui.progress(State::Working, working)?;
        return command();
    }
    ui.progress(State::BlockedAuth, "Waiting for your sudo password")?;
    let result = command();
    // Best effort: the command error is the one `main` reports.
    let cleared = ui.progress(State::Working, working);
    result.and(cleared)
}
