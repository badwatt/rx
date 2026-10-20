//! Generic process-execution mechanism. Not business, and not a semantic port.
//!
//! `command` is an adapter-internal seam: the feature adapters (git, nix,
//! terminal) are built on [`command::CommandRunner`], so `--dry-run` decorates
//! a single implementation instead of every semantic port.

pub(crate) mod command;
pub(crate) mod dry_run;
pub(crate) mod process;
