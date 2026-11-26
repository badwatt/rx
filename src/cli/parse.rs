use std::io::Write;

use crate::cli::Intent;
use crate::cli::completions::Shell;
use crate::cli::defaults::{default_config, expand_tilde};
use crate::domain::config::{Config, Step};
use crate::terminal::ansi;

/// Which [`Config`] field a value flag targets.
#[derive(Clone, Copy)]
enum ValueKey {
    Repo,
    Hostname,
    Keep,
    Jobs,
    Cores,
}

/// Write a parse error to `writer`. Error is intentionally discarded —
/// we're already in an error path and there's no useful recovery.
/// Parse errors are deliberately uncolored so they are safe when piped.
fn write_parse_error(writer: &mut dyn Write, msg: &str) {
    let _ = ansi::write_flush(writer, &format!("Error: {msg}\n"));
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
            write_parse_error(writer, &format!("Unknown argument \"{arg}\""));
            return Intent::Invalid;
        };
        i += 1 + consumed;
    }
    Intent::Run(config)
}

/// Parse a `--long` flag (or `--long=value`) at `args[i]`.
///
/// Returns the number of extra arguments consumed, or the terminal intent.
fn parse_long<'a>(
    arg: &'a str,
    args: &'a [String],
    i: usize,
    config: &mut Config,
    writer: &mut dyn Write,
) -> Result<usize, Intent> {
    // GNU-style `--flag=value`: the value travels in the same argv entry.
    let (name, inline) = match arg.split_once('=') {
        Some((name, value)) => (name, Some(value)),
        None => (arg, None),
    };
    match name {
        "--completions" => {
            let (value, _) = long_value(name, inline, args, i, writer)?;
            let Some(shell) = Shell::from_name(value) else {
                write_parse_error(
                    writer,
                    &format!("Unknown shell \"{value}\" (valid: {})", Shell::names()),
                );
                return Err(Intent::Invalid);
            };
            return Err(Intent::Completions(shell));
        }
        "--only" => {
            let (value, consumed) = long_value(name, inline, args, i, writer)?;
            config.set_only(parse_steps(value, writer)?);
            return Ok(consumed);
        }
        _ => {}
    }
    if let Some(key) = long_value_key(name) {
        let (value, consumed) = long_value(name, inline, args, i, writer)?;
        set_value(key, value, name, config, writer)?;
        return Ok(consumed);
    }
    // Everything else is a switch: it takes neither an inline nor a separate
    // value.
    if inline.is_some() {
        write_parse_error(writer, &format!("\"{name}\" flag does not take a value"));
        return Err(Intent::Invalid);
    }
    match name {
        "--help" => Err(Intent::Help),
        "--version" => Err(Intent::Version),
        "--update" => {
            config.set_update(true);
            Ok(0)
        }
        "--diff" => {
            config.set_diff(true);
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
        "--no-color" => Ok(0),
        _ => {
            write_parse_error(writer, &format!("Unknown flag \"{name}\""));
            Err(Intent::Invalid)
        }
    }
}

/// Map a long value-flag name to its [`ValueKey`], or `None` for other flags.
fn long_value_key(arg: &str) -> Option<ValueKey> {
    match arg {
        "--repo" => Some(ValueKey::Repo),
        "--hostname" => Some(ValueKey::Hostname),
        "--keep" => Some(ValueKey::Keep),
        "--jobs" => Some(ValueKey::Jobs),
        "--cores" => Some(ValueKey::Cores),
        _ => None,
    }
}

/// Resolve a long flag's value and how many argv entries it consumed.
///
/// `--flag=value` carries its value inline; plain `--flag value` consumes the
/// next argv entry.
fn long_value<'a>(
    name: &str,
    inline: Option<&'a str>,
    args: &'a [String],
    i: usize,
    writer: &mut dyn Write,
) -> Result<(&'a str, usize), Intent> {
    match inline {
        Some("") => {
            write_parse_error(writer, &format!("\"{name}\" flag requires an argument"));
            Err(Intent::Invalid)
        }
        Some(value) => Ok((value, 0)),
        None => Ok((take_value(args, i, name, writer)?, 1)),
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
    let mut rest = flags.chars().peekable();
    while let Some(flag) = rest.next() {
        match flag {
            'h' => return Err(Intent::Help),
            'v' => return Err(Intent::Version),
            'd' => config.set_diff(true),
            'u' => config.set_update(true),
            'y' => config.yes = true,
            _ => {
                let Some(key) = value_key(flag) else {
                    write_parse_error(writer, &format!("Unknown flag \"-{flag}\""));
                    return Err(Intent::Invalid);
                };
                let label = format!("-{flag}");
                // A value flag consumes the next argv, so anything after it in
                // the same cluster would be silently dropped. Reject instead.
                if rest.peek().is_some() {
                    write_parse_error(
                        writer,
                        &format!("\"{label}\" flag must be last in a cluster"),
                    );
                    return Err(Intent::Invalid);
                }
                let value = take_value(args, i, &label, writer)?;
                set_value(key, value, &label, config, writer)?;
                return Ok(1);
            }
        }
    }
    Ok(0)
}

/// Map a short value-flag character to its [`ValueKey`], or `None` for a
/// switch (or an unknown flag).
fn value_key(flag: char) -> Option<ValueKey> {
    match flag {
        'r' => Some(ValueKey::Repo),
        'n' => Some(ValueKey::Hostname),
        'k' => Some(ValueKey::Keep),
        'j' => Some(ValueKey::Jobs),
        'c' => Some(ValueKey::Cores),
        _ => None,
    }
}

/// Read the argument following a value flag, or report a missing value.
fn take_value<'a>(
    args: &'a [String],
    i: usize,
    label: &str,
    writer: &mut dyn Write,
) -> Result<&'a str, Intent> {
    let Some(next) = args.get(i + 1) else {
        write_parse_error(writer, &format!("\"{label}\" flag requires an argument"));
        return Err(Intent::Invalid);
    };
    // Another flag is not accepted as a value.
    if next.len() > 1 && next.starts_with('-') {
        write_parse_error(writer, &format!("\"{label}\" flag requires an argument"));
        return Err(Intent::Invalid);
    }
    Ok(next.as_str())
}

/// Assign `value` to the field selected by `key`.
fn set_value(
    key: ValueKey,
    value: &str,
    label: &str,
    config: &mut Config,
    writer: &mut dyn Write,
) -> Result<(), Intent> {
    match key {
        ValueKey::Repo => config.repo = expand_tilde(value),
        ValueKey::Hostname => config.hostname = Some(value.to_owned()),
        ValueKey::Keep => config.keep = parse_bounded(value, label, 1u8, u8::MAX, writer)?,
        ValueKey::Jobs => {
            config.jobs = Some(parse_bounded(value, label, 1u16, u16::MAX, writer)?);
        }
        ValueKey::Cores => {
            config.cores = Some(parse_bounded(value, label, 1u16, u16::MAX, writer)?);
        }
    }
    Ok(())
}

/// Parse a comma-separated `--only` step list, or report an unknown name.
fn parse_steps(value: &str, writer: &mut dyn Write) -> Result<Vec<Step>, Intent> {
    let mut steps = Vec::new();
    for name in value.split(',') {
        let Some(step) = Step::from_name(name.trim()) else {
            write_parse_error(
                writer,
                &format!("Unknown step \"{name}\" (valid: {})", Step::names()),
            );
            return Err(Intent::Invalid);
        };
        steps.push(step);
    }
    Ok(steps)
}

/// Report the accepted range for a numeric flag.
fn write_range_error<T: std::fmt::Display>(
    writer: &mut dyn Write,
    label: &str,
    lowest: T,
    highest: T,
) {
    write_parse_error(
        writer,
        &format!("Value of \"{label}\" flag must be between {lowest} and {highest}."),
    );
}

/// Parse a numeric flag value of at least `lowest`, or report an error.
///
/// `highest` only names the bound in the message: the target type already caps
/// the value, so an overflow is the only way to exceed it. Telling "not
/// numeric" apart from out of range keeps `--keep 300` honest.
fn parse_bounded<T>(
    value: &str,
    label: &str,
    lowest: T,
    highest: T,
    writer: &mut dyn Write,
) -> Result<T, Intent>
where
    T: std::str::FromStr<Err = std::num::ParseIntError> + PartialOrd + std::fmt::Display,
{
    match value.parse::<T>() {
        Ok(num) if num >= lowest => Ok(num),
        Ok(_) => {
            write_range_error(writer, label, lowest, highest);
            Err(Intent::Invalid)
        }
        Err(e) if e.kind() == &std::num::IntErrorKind::PosOverflow => {
            write_range_error(writer, label, lowest, highest);
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
