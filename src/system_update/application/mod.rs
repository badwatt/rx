//! Use-case slice: the NixOS update workflow.
//!
//! Orchestrates the update pipeline (pull → flake update → stage → rebuild →
//! cleanup → diff) through the [`Repo`], [`Nix`] and [`UserIo`] ports. Contains
//! no I/O and no knowledge of the underlying tools.

pub(crate) mod steps;

use crate::domain::config::Config;
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
    // hostname could not be resolved or was forced to the sentinel.
    if config.hostname == "unknown" {
        return Err(Error::HostnameUnknown);
    }

    ui.title("RX Configuration")?;
    ui.config(config)?;

    if !config.yes && !ui.confirm(true, None)? {
        return Ok(());
    }

    steps::pull::run(&config.repo, repo, ui)?;

    if config.update {
        steps::flake::run(config, nix, ui)?;
    }

    steps::stage::run(&config.repo, config.yes, repo, ui)?;
    steps::rebuild::run(config, nix, ui)?;
    steps::cleanup::run(config, nix, ui)?;

    if config.diff {
        steps::diff::run(nix, ui)?;
    }

    Ok(())
}

#[cfg(test)]
mod tests;
