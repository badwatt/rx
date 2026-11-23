use crate::domain::config::BuildOptions;
use crate::domain::error::Error;
use std::time::Duration;

use crate::infra::command::CommandRunner;
use crate::infra::process::ProcessRunner;

mod argv;

use self::argv::{
    cmd_collect_garbage, cmd_diff_closures, cmd_prune, cmd_rebuild, cmd_update, rebuild_probe,
    sudo_keep_alive, sudo_probe, sudo_validate,
};
use crate::system_update::ports::nix::Nix;

/// How often the background thread renews the cached sudo credentials.
const KEEP_ALIVE_INTERVAL: Duration = Duration::from_secs(60);

/// Renews the sudo ticket; returns `false` once it cannot be renewed.
type RefreshFn = dyn Fn() -> bool + Send + Sync + 'static;

/// Keep the cached sudo credentials fresh in the background.
///
/// `sudo -v` extends the ticket for `timestamp_timeout` only (five minutes by
/// default), and a `nixos-rebuild -ng` build can outlast it: the activation
/// would then ask for a password that no report announces. A thread that
/// renews the ticket removes that prompt. It ends with the process.
fn spawn_keep_alive(interval: Duration, refresh: Box<RefreshFn>) -> std::thread::JoinHandle<()> {
    std::thread::spawn(move || {
        while refresh() {
            std::thread::sleep(interval);
        }
    })
}

/// Renew the sudo ticket without a prompt; `false` when sudo stops obliging.
fn refresh_sudo_ticket() -> bool {
    // `.ok() == Some(0)` keeps the outcome a plain value: a `matches!` arm
    // would add a region only one of the two outcomes ever executes.
    ProcessRunner.probe(&sudo_keep_alive()).ok() == Some(0)
}

/// Which `nixos-rebuild` implementation is installed.
///
/// The classic shell script elevates the whole command and skips the
/// self-rebuild re-exec with `--fast`; the `-ng` rewrite elevates only the
/// activation with `--elevate=sudo` and spells the skip `--no-reexec`.
#[derive(Debug, PartialEq, Clone, Copy)]
enum RebuildFlavor {
    Classic,
    Ng,
}

/// [`Nix`] adapter backed by the `nix` / `nixos-rebuild` CLIs.
pub(crate) struct NixCli<'a> {
    runner: &'a dyn CommandRunner,
}

impl<'a> NixCli<'a> {
    pub(crate) fn new(runner: &'a dyn CommandRunner) -> Self {
        Self { runner }
    }

    /// Probe the installed `nixos-rebuild`: exit code 0 means the `-ng`
    /// rewrite. Any other outcome (non-zero, spawn error) is conservatively
    /// treated as classic.
    fn detect_flavor(&self) -> RebuildFlavor {
        if matches!(self.runner.probe(&rebuild_probe()), Ok(0)) {
            RebuildFlavor::Ng
        } else {
            RebuildFlavor::Classic
        }
    }
}

impl Nix for NixCli<'_> {
    fn update(&self, repo: &str) -> Result<(), Error> {
        let status = self.runner.run(&cmd_update(repo))?;
        if status == 0 {
            Ok(())
        } else {
            Err(Error::FlakeUpdateFailed)
        }
    }

    fn rebuild(&self, repo: &str, hostname: &str, options: BuildOptions) -> Result<(), Error> {
        let flavor = self.detect_flavor();
        let status = self
            .runner
            .run(&cmd_rebuild(repo, hostname, flavor, options))?;
        if status == 0 {
            Ok(())
        } else {
            Err(Error::RebuildFailed)
        }
    }

    fn password_pending(&self) -> Result<bool, Error> {
        // `sudo -n true` only succeeds when no password is needed; any other
        // outcome means a prompt is coming.
        Ok(!matches!(self.runner.probe(&sudo_probe()), Ok(0)))
    }

    fn refresh_credentials(&self) -> Result<(), Error> {
        let status = self.runner.run(&sudo_validate())?;
        if status != 0 {
            return Err(Error::PasswordFailed);
        }
        // Detached on purpose: the ticket must stay fresh while the elevated
        // command runs, and the thread dies with the process.
        let _ = spawn_keep_alive(KEEP_ALIVE_INTERVAL, Box::new(refresh_sudo_ticket));
        Ok(())
    }

    fn prune(&self, keep: u8) -> Result<(), Error> {
        let status = self.runner.run(&cmd_prune(keep))?;
        if status == 0 {
            Ok(())
        } else {
            Err(Error::CleanupFailed)
        }
    }

    fn collect_garbage(&self) -> Result<(), Error> {
        let status = self.runner.run(&cmd_collect_garbage())?;
        if status == 0 {
            Ok(())
        } else {
            Err(Error::CleanupFailed)
        }
    }

    fn diff_closures(&self) -> Result<String, Error> {
        let (status, output) = self.runner.capture(&cmd_diff_closures())?;
        if status == 0 {
            Ok(output)
        } else {
            Err(Error::DiffFailed)
        }
    }
}
