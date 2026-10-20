use std::io::Write;

use crate::cli::Intent;
use crate::cli::defaults::{default_config, expand_tilde};
use crate::domain::config::Config;
use crate::shared::ansi;

/// Which [`Config`] field a value flag targets.
#[derive(Clone, Copy)]
enum ValueKey {
    Repo,
    Hostname,
    Keep,
}

/// Write a parse error to `writer`. Error is intentionally discarded —
/// we're already in an error path and there's no useful recovery.
fn write_parse_error(writer: &mut dyn Write, msg: &str) {
    let _ = ansi::write_flush(
        writer,
        &format!("{}Error: {}{}\n", ansi::RED, msg, ansi::RESET),
    );
}

/// Parse `args` (including the program name at index 0).
///
/// Prints error messages directly to `writer` for invalid flags/arguments.
pub(crate) fn parse(args: &[String], writer: &mut dyn Write) -> Intent {
    if args.len() <= 1 {
        return Intent::Run(default_config());
    }

    if let Some(intent) = word_intent(args) {
        return intent;
    }

    if !args[1].starts_with('-') {
        write_parse_error(writer, &format!("Unknown argument \"{}\"", args[1]));
        return Intent::Invalid;
    }

    parse_flags(args, writer)
}

/// Handle the bare `help` / `version` subcommand words.
fn word_intent(args: &[String]) -> Option<Intent> {
    if args.len() != 2 {
        return None;
    }
    match args[1].as_str() {
        "help" => Some(Intent::Help),
        "version" => Some(Intent::Version),
        _ => None,
    }
}

/// Walk the argument list, folding flags into a [`Config`].
fn parse_flags(args: &[String], writer: &mut dyn Write) -> Intent {
    let mut config = default_config();
    let mut i = 1;
    while i < args.len() {
        let arg = &args[i];
        let consumed = if arg.starts_with("--") {
            match parse_long(arg, args, i, &mut config, writer) {
                Ok(n) => n,
                Err(intent) => return intent,
            }
        } else if arg.starts_with('-') {
            match parse_short(arg, args, i, &mut config, writer) {
                Ok(n) => n,
                Err(intent) => return intent,
            }
        } else {
            0
        };
        i += 1 + consumed;
    }
    Intent::Run(config)
}

/// Parse a `--long` flag at `args[i]`.
///
/// Returns the number of extra arguments consumed, or the terminal intent.
fn parse_long(
    arg: &str,
    args: &[String],
    i: usize,
    config: &mut Config,
    writer: &mut dyn Write,
) -> Result<usize, Intent> {
    match arg {
        "--help" => Err(Intent::Help),
        "--version" => Err(Intent::Version),
        "--update" => {
            config.update = true;
            Ok(0)
        }
        "--diff" => {
            config.diff = true;
            Ok(0)
        }
        "--yes" => {
            config.yes = true;
            Ok(0)
        }
        "--dry-run" => {
            config.dry_run = true;
            Ok(0)
        }
        "--repo" | "--hostname" | "--keep" => {
            let value = take_value(args, i, arg, writer)?;
            set_value(long_value_key(arg), value, arg, config, writer)
        }
        _ => {
            write_parse_error(writer, &format!("Unknown flag \"{arg}\""));
            Err(Intent::Invalid)
        }
    }
}

/// Map a long value-flag name to its [`ValueKey`].
fn long_value_key(arg: &str) -> ValueKey {
    match arg {
        "--repo" => ValueKey::Repo,
        "--hostname" => ValueKey::Hostname,
        _ => ValueKey::Keep,
    }
}

/// Parse a `-short` flag cluster at `args[i]`.
///
/// Returns the number of extra arguments consumed, or the terminal intent.
fn parse_short(
    arg: &str,
    args: &[String],
    i: usize,
    config: &mut Config,
    writer: &mut dyn Write,
) -> Result<usize, Intent> {
    let flags = arg.strip_prefix('-').unwrap_or(arg);
    for flag in flags.chars() {
        match flag {
            'h' => return Err(Intent::Help),
            'v' => return Err(Intent::Version),
            'd' => config.diff = true,
            'u' => config.update = true,
            'y' => config.yes = true,
            'r' | 'n' | 'k' => {
                let label = format!("-{flag}");
                let value = take_value(args, i, &label, writer)?;
                set_value(value_key(flag), value, &label, config, writer)?;
                return Ok(1);
            }
            _ => {
                write_parse_error(writer, &format!("Unknown flag \"-{flag}\""));
                return Err(Intent::Invalid);
            }
        }
    }
    Ok(0)
}

/// Map a short value-flag character to its [`ValueKey`].
fn value_key(flag: char) -> ValueKey {
    match flag {
        'r' => ValueKey::Repo,
        'n' => ValueKey::Hostname,
        _ => ValueKey::Keep,
    }
}

/// Read the argument following a value flag, or report a missing value.
fn take_value(
    args: &[String],
    i: usize,
    label: &str,
    writer: &mut dyn Write,
) -> Result<String, Intent> {
    if i + 1 >= args.len() {
        write_parse_error(writer, &format!("\"{label}\" flag requires an argument"));
        Err(Intent::Invalid)
    } else {
        Ok(args[i + 1].clone())
    }
}

/// Assign `value` to the field selected by `key`.
fn set_value(
    key: ValueKey,
    value: String,
    label: &str,
    config: &mut Config,
    writer: &mut dyn Write,
) -> Result<usize, Intent> {
    match key {
        ValueKey::Repo => config.repo = expand_tilde(&value),
        ValueKey::Hostname => config.hostname = value,
        ValueKey::Keep => config.keep = parse_keep(&value, label, writer)?,
    }
    Ok(1)
}

/// Parse a numeric `--keep` / `-k` value (>= 1), or report an error.
fn parse_keep(value: &str, label: &str, writer: &mut dyn Write) -> Result<u8, Intent> {
    match value.parse::<u8>() {
        Ok(num) if num >= 1 => Ok(num),
        Ok(_) => {
            write_parse_error(
                writer,
                &format!("Value of \"{label}\" flag must be at least 1."),
            );
            Err(Intent::Invalid)
        }
        Err(_) => {
            write_parse_error(
                writer,
                &format!("Value of \"{label}\" flag is not numeric."),
            );
            Err(Intent::Invalid)
        }
    }
}

#[cfg(test)]
mod tests;
