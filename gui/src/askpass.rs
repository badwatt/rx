//! `sudo` askpass helper.
//!
//! `rxui --askpass` is what `SUDO_ASKPASS` points at.  It prints the secret
//! the app stashed under the user's runtime dir (mode 0600) and exits; the
//! path is derived from the uid because sudo resets the environment.

use std::process::ExitCode;

use crate::pipeline;

/// Print the stashed secret for `sudo`'s askpass mechanism.
pub(crate) fn run() -> ExitCode {
    let Some(path) = pipeline::secret_path() else {
        eprintln!("rxui: cannot determine the askpass path");
        return ExitCode::FAILURE;
    };
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
