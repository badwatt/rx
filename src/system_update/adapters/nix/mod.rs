use crate::domain::config::BuildOptions;
use crate::domain::error::Error;
use crate::infra::command::CommandRunner;

mod argv;

use self::argv::{
    cmd_collect_garbage, cmd_diff_closures, cmd_prune, cmd_rebuild, cmd_update, rebuild_probe,
};
use crate::system_update::ports::nix::Nix;

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
