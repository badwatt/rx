//! Ports: the interfaces the application core depends on.
//!
//! `repo` and `nix` are semantic driven ports (version control and Nix
//! operations); `user_io` is the presentation/interaction port. The
//! low-level argv runner is an adapter-internal seam, not a port.

pub(crate) mod nix;
pub(crate) mod repo;
pub(crate) mod user_io;
