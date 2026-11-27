use crate::domain::config::BuildOptions;
use crate::domain::error::Error;

/// Driven port: Nix / NixOS operations.
pub(crate) trait Nix {
    /// Update the flake inputs for `repo`.
    ///
    /// `command` replaces the default `nix flake update` when the user
    /// configured one; it runs at the repo root.
    fn update(&self, repo: &str, command: Option<&[String]>) -> Result<(), Error>;
    /// Rebuild and switch the system for `hostname`.
    ///
    /// `nom` pipes the rebuild's log through `nix-output-monitor`: `None`
    /// when it is installed, `Some(true)` to insist, `Some(false)` to never.
    fn rebuild(
        &self,
        repo: &str,
        hostname: &str,
        options: BuildOptions,
        nom: Option<bool>,
    ) -> Result<(), Error>;
    /// Whether the next elevated command will ask for a sudo password.
    fn password_pending(&self) -> Result<bool, Error>;
    /// Prompt for and cache the sudo credentials now, so a later elevated
    /// command does not have to block.
    fn refresh_credentials(&self) -> Result<(), Error>;
    /// Delete old generations, keeping `keep`.
    fn prune(&self, keep: u8) -> Result<(), Error>;
    /// Free unreferenced store paths.
    fn collect_garbage(&self) -> Result<(), Error>;
    /// Raw `nix profile diff-closures` output.
    fn diff_closures(&self) -> Result<String, Error>;
}
