//! Ports: the interfaces the application core depends on.
//!
//! `command` is a driven port (external command execution),
//! `user_io` is a driven port (user presentation and interaction).

pub mod command;
pub mod user_io;
