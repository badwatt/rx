//! Use-case slice: the NixOS update workflow.
//!
//! Orchestrates the update pipeline (pull → flake update → stage → rebuild →
//! cleanup → diff) through the [`CommandRunner`] and [`UserIo`] ports. Contains
//! no I/O of its own.

pub(crate) mod steps;

use crate::config::Config;
use crate::error::Error;
use crate::ports::command::CommandRunner;
use crate::ports::user_io::UserIo;

/// Run the full NixOS update workflow.
pub(crate) fn run(config: &Config, cmd: &dyn CommandRunner, ui: &dyn UserIo) -> Result<(), Error> {
    if let Some(msg) = config.validate() {
        return Err(Error::InvalidConfig(msg));
    }
    ui.title("RX Configuration")?;
    ui.config(config)?;

    if !config.yes && !ui.confirm(true, None)? {
        return Ok(());
    }

    steps::pull::run(&config.repo, cmd, ui)?;

    if config.update {
        steps::flake::run(config, cmd, ui)?;
    }

    steps::stage::run(&config.repo, config.yes, cmd, ui)?;
    steps::rebuild::run(config, cmd, ui)?;
    steps::cleanup::run(config, cmd, ui)?;

    if config.diff {
        steps::diff::run(cmd, ui)?;
    }

    Ok(())
}

#[cfg(test)]
mod tests;
