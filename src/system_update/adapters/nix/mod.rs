use crate::domain::config::BuildOptions;
use crate::domain::error::Error;
use std::time::Duration;

use crate::infra::command::CommandRunner;
use crate::infra::exit::ensure_success;
use crate::infra::process::ProcessRunner;

mod argv;

use self::argv::{
    cmd_collect_garbage, cmd_diff_closures, cmd_nom, cmd_nom_probe, cmd_prune, cmd_rebuild,
    cmd_update, rebuild_probe, sudo_keep_alive, sudo_probe, sudo_validate,
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

    /// Whether `nom` can be run; a failed probe means it is not installed.
    fn nom_available(&self) -> bool {
        self.runner.probe(&cmd_nom_probe()).is_ok()
    }
}

impl Nix for NixCli<'_> {
    fn update(&self, repo: &str, command: Option<&[String]>) -> Result<(), Error> {
        let status = match command {
            Some(argv) => self.runner.run_in(repo, argv)?,
            None => self.runner.run(&cmd_update(repo))?,
        };
        ensure_success(status, Error::FlakeUpdateFailed)
    }

    fn rebuild(
        &self,
        repo: &str,
        hostname: &str,
        options: BuildOptions,
        nom: Option<bool>,
    ) -> Result<(), Error> {
        let flavor = self.detect_flavor();
        let command = cmd_rebuild(repo, hostname, flavor, options);
        let use_nom = match nom {
            Some(false) => false,
            Some(true) if !self.nom_available() => return Err(Error::NomMissing),
            Some(true) => true,
            None => self.nom_available(),
        };
        let status = if use_nom {
            self.runner.run_through(&command, &cmd_nom())?
        } else {
            self.runner.run(&command)?
        };
        ensure_success(status, Error::RebuildFailed)
    }

    fn password_pending(&self) -> Result<bool, Error> {
        // `sudo -n true` only succeeds when no password is needed; any other
        // outcome means a prompt is coming.
        Ok(!matches!(self.runner.probe(&sudo_probe()), Ok(0)))
    }

    fn refresh_credentials(&self) -> Result<(), Error> {
        let status = self.runner.run(&sudo_validate())?;
        ensure_success(status, Error::PasswordFailed)?;
        // Detached on purpose: the ticket must stay fresh while the elevated
        // command runs, and the thread dies with the process.
        let _ = spawn_keep_alive(KEEP_ALIVE_INTERVAL, Box::new(refresh_sudo_ticket));
        Ok(())
    }

    fn prune(&self, keep: u8) -> Result<(), Error> {
        let status = self.runner.run(&cmd_prune(keep))?;
        ensure_success(status, Error::CleanupFailed)
    }

    fn collect_garbage(&self) -> Result<(), Error> {
        let status = self.runner.run(&cmd_collect_garbage())?;
        ensure_success(status, Error::CleanupFailed)
    }

    fn diff_closures(&self) -> Result<String, Error> {
        let (status, output) = self.runner.capture(&cmd_diff_closures())?;
        ensure_success(status, Error::DiffFailed)?;
        Ok(output)
    }
}
