use crate::domain::error::Error;

/// Driven port: Nix / NixOS operations.
pub(crate) trait Nix {
    /// Update the flake inputs for `repo`.
    fn update(&self, repo: &str) -> Result<(), Error>;
    /// Rebuild and switch the system for `hostname`.
    fn rebuild(&self, repo: &str, hostname: &str) -> Result<(), Error>;
    /// Delete old generations, keeping `keep`.
    fn prune(&self, keep: u8) -> Result<(), Error>;
    /// Free unreferenced store paths.
    fn collect_garbage(&self) -> Result<(), Error>;
    /// Raw `nix profile diff-closures` output.
    fn diff_closures(&self) -> Result<String, Error>;
}
