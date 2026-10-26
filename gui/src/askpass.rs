//! `sudo` askpass helper.
//!
//! `rxui --askpass` is what `SUDO_ASKPASS` points at.  It prints the secret
//! that the app stashed in `$XDG_RUNTIME_DIR` (mode 0600, tmpfs) and exits;
//! nothing is passed through argv or the environment.

use std::process::ExitCode;

use crate::pipeline;

/// Print the stashed secret for `sudo`'s askpass mechanism.
///
/// The path is derived from the uid because sudo resets the environment
/// before executing a command; a variable set on `rx` may not survive.
pub(crate) fn run() -> ExitCode {
    let path = pipeline::secret_path();
    match std::fs::read_to_string(&path) {
        Ok(secret) => {
            print!("{}", secret.trim_end_matches('\n'));
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("rxui: cannot read {}: {error}", path.display());
            ExitCode::FAILURE
        }
    }
}
