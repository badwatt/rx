//! Use-case slice: the NixOS update workflow.
//!
//! Orchestrates the update pipeline (pull → flake update → stage → rebuild →
//! cleanup → diff) through the [`Repo`], [`Nix`] and [`UserIo`] ports. Contains
//! no I/O and no knowledge of the underlying tools.

pub(crate) mod steps;

use crate::domain::config::{Config, Step};
use crate::domain::error::Error;
use crate::shared::osc::State;
use crate::system_update::ports::nix::Nix;
use crate::system_update::ports::repo::Repo;
use crate::system_update::ports::user_io::UserIo;

/// Run the full NixOS update workflow, bracketed by OSC 7501 status reports.
pub(crate) fn run(
    config: &Config,
    repo: &dyn Repo,
    nix: &dyn Nix,
    ui: &dyn UserIo,
) -> Result<(), Error> {
    ui.progress(State::Working, "Starting the system update")?;
    match execute(config, repo, nix, ui) {
        Ok(()) => {
            ui.progress(State::Done, "System update finished")?;
            Ok(())
        }
        Err(error) => {
            // Best effort: the step failure is the error `main` reports.
            let _ = ui.progress(State::Failed, &error.to_string());
            Err(error)
        }
    }
}

/// The pipeline proper; [`run`] adds the state reports around it.
fn execute(config: &Config, repo: &dyn Repo, nix: &dyn Nix, ui: &dyn UserIo) -> Result<(), Error> {
    config.validate()?;

    // Resolve the hostname before any side effect (pull, flake update,
    // staging): an undetected hostname must abort, not fail halfway through.
    // Only the rebuild needs it, so a run without that step does not care.
    let hostname = required_hostname(config)?;

    ui.title("RX Configuration")?;
    ui.config(config)?;

    if !config.yes {
        ui.progress(State::BlockedPermission, "Waiting for confirmation")?;
        if !ui.confirm(true, None)? {
            // Best-effort: the workflow is ending anyway.
            let _ = ui.output("Aborted.\n");
            return Ok(());
        }
        // The user answered, so the tab is working again.
        ui.progress(State::Working, "Starting the system update")?;
    }

    if config.runs(Step::Pull) {
        steps::pull::run(&config.repo, repo, ui)?;
    }

    if config.runs(Step::Flake) {
        steps::flake::run(config, nix, ui)?;
    }

    if config.runs(Step::Stage) && !steps::stage::run(&config.repo, config.yes, repo, ui)? {
        return Ok(());
    }

    if let Some(hostname) = hostname {
        steps::rebuild::run(config, hostname, nix, ui)?;
    }

    if config.runs(Step::Cleanup) {
        steps::cleanup::run(config, nix, ui)?;
    }

    if config.runs(Step::Diff) {
        steps::diff::run(nix, ui)?;
    }

    Ok(())
}

/// Hostname for the rebuild, or `None` when the rebuild is not selected.
fn required_hostname(config: &Config) -> Result<Option<&str>, Error> {
    if !config.runs(Step::Rebuild) {
        return Ok(None);
    }
    match config.hostname.as_deref() {
        Some(hostname) => Ok(Some(hostname)),
        None => Err(Error::HostnameUnknown),
    }
}

#[cfg(test)]
mod tests;
