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

    // Abort before any side effect (pull, flake update, staging) when the
    // hostname is unknown: the rebuild needs it for the flake attribute.
    let Some(hostname) = config.hostname.as_deref() else {
        return Err(Error::HostnameUnknown);
    };

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

    if config.runs(Step::Rebuild) {
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

#[cfg(test)]
mod tests;
