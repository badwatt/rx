//! `sudo` askpass helper.
//!
//! `rxui --askpass` is what `SUDO_ASKPASS` points at.  It prints the secret
//! that the app stashed in `$XDG_RUNTIME_DIR` (mode 0600, tmpfs) and exits;
//! nothing is passed through argv or the environment.

use std::process::ExitCode;

/// Path of the stash, as exported to the child `rx` processes.
pub(crate) const FILE_VAR: &str = "RXUI_ASKPASS_FILE";

/// Print the stashed secret for `sudo -A`.
pub(crate) fn run() -> ExitCode {
    let Ok(path) = std::env::var(FILE_VAR) else {
        return ExitCode::FAILURE;
    };
    match std::fs::read_to_string(path) {
        Ok(secret) => {
            print!("{}", secret.trim_end_matches('\n'));
            ExitCode::SUCCESS
        }
        Err(_) => ExitCode::FAILURE,
    }
}
