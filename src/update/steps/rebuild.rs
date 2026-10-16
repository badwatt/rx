use crate::commands;
use crate::commands::RebuildFlavor;
use crate::config::Config;
use crate::error::Error;
use crate::ports::command::CommandRunner;
use crate::ports::user_io::UserIo;

/// Rebuild and switch the NixOS system for `config.hostname`.
pub(crate) fn run(config: &Config, cmd: &dyn CommandRunner, ui: &dyn UserIo) -> Result<(), Error> {
    if config.hostname == "unknown" {
        return Err(Error::HostnameUnknown);
    }
    ui.title("Nixos Rebuild")?;
    let flavor = detect_flavor(cmd);
    let status = cmd.run(&commands::nix_rebuild(
        &config.repo,
        &config.hostname,
        flavor,
    ))?;
    if status != 0 {
        return Err(Error::RebuildFailed);
    }
    Ok(())
}

/// Probe the installed `nixos-rebuild`: exit code 0 means the `-ng` rewrite.
/// Any other outcome (non-zero, spawn error) conservatively means classic.
fn detect_flavor(cmd: &dyn CommandRunner) -> RebuildFlavor {
    if matches!(cmd.probe(&commands::rebuild_probe()), Ok(0)) {
        RebuildFlavor::Ng
    } else {
        RebuildFlavor::Classic
    }
}
