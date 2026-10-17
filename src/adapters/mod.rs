//! Driven adapters: concrete implementations of the ports.
//!
//! `command` is an adapter-internal seam: the git/nix adapters are built on
//! the low-level [`command::CommandRunner`], so `--dry-run` decorates a
//! single implementation instead of every semantic port.

pub(crate) mod command;
pub(crate) mod dry_run;
pub(crate) mod git;
pub(crate) mod nix;
pub(crate) mod process;
pub(crate) mod terminal;
