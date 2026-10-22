//! Use-case slice: the NixOS update workflow.
//!
//! Orchestrates the update pipeline (pull → flake update → stage → rebuild →
//! cleanup → diff) through the [`Repo`], [`Nix`] and [`UserIo`] ports. Contains
//! no I/O and no knowledge of the underlying tools.

pub(crate) mod steps;

use crate::domain::config::{Config, Step};
use crate::domain::error::Error;
use crate::system_update::ports::nix::Nix;
use crate::system_update::ports::repo::Repo;
use crate::system_update::ports::user_io::UserIo;

/// Run the full NixOS update workflow.
pub(crate) fn run(
    config: &Config,
    repo: &dyn Repo,
    nix: &dyn Nix,
    ui: &dyn UserIo,
) -> Result<(), Error> {
    if let Some(msg) = config.validate() {
        return Err(Error::InvalidConfig(msg));
    }

    // Resolve the hostname before any side effect (pull, flake update,
    // staging): an undetected hostname must abort, not fail halfway through.
    // Only the rebuild needs it, so a run without that step does not care.
    let hostname = required_hostname(config)?;

    ui.title("RX Configuration")?;
    ui.config(config)?;

    if !config.yes && !ui.confirm(true, None)? {
        // Best-effort: the workflow is ending anyway.
        let _ = ui.output("Aborted.\n");
        return Ok(());
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
