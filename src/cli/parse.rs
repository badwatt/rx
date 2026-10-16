use std::io::Write;

use crate::ansi;
use crate::cli::Intent;
use crate::cli::defaults::{default_config, expand_tilde};
use crate::config::Config;

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
pub fn parse(args: &[String], writer: &mut dyn Write) -> Intent {
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
mod tests {
    use super::*;

    fn args(from: &[&str]) -> Vec<String> {
        from.iter().copied().map(str::to_string).collect()
    }

    // ------------------------------------------------------------------
    // early-return paths
    // ------------------------------------------------------------------

    #[test]
    fn parse_no_args_yields_defaults() {
        let mut buf = Vec::new();
        assert_eq!(
            parse(&args(&["rx"]), &mut buf),
            Intent::Run(default_config())
        );
    }

    #[test]
    fn parse_help_flag_returns_help() {
        let mut buf = Vec::new();
        assert_eq!(parse(&args(&["rx", "-h"]), &mut buf), Intent::Help);
    }

    #[test]
    fn parse_help_word_returns_help() {
        let mut buf = Vec::new();
        assert_eq!(parse(&args(&["rx", "help"]), &mut buf), Intent::Help);
    }

    #[test]
    fn parse_version_flag_returns_version() {
        let mut buf = Vec::new();
        assert_eq!(parse(&args(&["rx", "-v"]), &mut buf), Intent::Version);
    }

    #[test]
    fn parse_version_word_returns_version() {
        let mut buf = Vec::new();
        assert_eq!(parse(&args(&["rx", "version"]), &mut buf), Intent::Version);
    }

    #[test]
    fn parse_unknown_argument_returns_error() {
        let mut buf = Vec::new();
        assert_eq!(parse(&args(&["rx", "unknown"]), &mut buf), Intent::Invalid);
        assert!(String::from_utf8_lossy(&buf).contains("Unknown argument"));
    }

    // ------------------------------------------------------------------
    // value flags
    // ------------------------------------------------------------------

    #[test]
    fn parse_r_missing_value_returns_error() {
        let mut buf = Vec::new();
        assert_eq!(parse(&args(&["rx", "-r"]), &mut buf), Intent::Invalid);
        assert!(String::from_utf8_lossy(&buf).contains("requires an argument"));
    }

    #[test]
    fn parse_n_missing_value_returns_error() {
        let mut buf = Vec::new();
        assert_eq!(parse(&args(&["rx", "-n"]), &mut buf), Intent::Invalid);
        assert!(String::from_utf8_lossy(&buf).contains("requires an argument"));
    }

    #[test]
    fn parse_k_non_numeric_returns_error() {
        let mut buf = Vec::new();
        assert_eq!(
            parse(&args(&["rx", "-k", "abc"]), &mut buf),
            Intent::Invalid
        );
        assert!(String::from_utf8_lossy(&buf).contains("not numeric"));
    }

    #[test]
    fn parse_k_zero_returns_error() {
        let mut buf = Vec::new();
        assert_eq!(parse(&args(&["rx", "-k", "0"]), &mut buf), Intent::Invalid);
        assert!(String::from_utf8_lossy(&buf).contains("at least 1"));
    }

    #[test]
    fn parse_r_with_value_sets_repo() {
        let mut buf = Vec::new();
        let mut expected = default_config();
        expected.repo = String::from("/path/to/repo");
        assert_eq!(
            parse(&args(&["rx", "-r", "/path/to/repo"]), &mut buf),
            Intent::Run(expected)
        );
    }

    #[test]
    fn parse_n_with_value_sets_hostname() {
        let mut buf = Vec::new();
        let mut expected = default_config();
        expected.hostname = String::from("myhost");
        assert_eq!(
            parse(&args(&["rx", "-n", "myhost"]), &mut buf),
            Intent::Run(expected)
        );
    }

    #[test]
    fn parse_k_with_value_sets_keep() {
        let mut buf = Vec::new();
        let mut expected = default_config();
        expected.keep = 5;
        assert_eq!(
            parse(&args(&["rx", "-k", "5"]), &mut buf),
            Intent::Run(expected)
        );
    }

    // ------------------------------------------------------------------
    // boolean flags
    // ------------------------------------------------------------------

    #[test]
    fn parse_d_flag_sets_diff() {
        let mut buf = Vec::new();
        let mut expected = default_config();
        expected.diff = true;
        assert_eq!(parse(&args(&["rx", "-d"]), &mut buf), Intent::Run(expected));
    }

    #[test]
    fn parse_u_flag_sets_update() {
        let mut buf = Vec::new();
        let mut expected = default_config();
        expected.update = true;
        assert_eq!(parse(&args(&["rx", "-u"]), &mut buf), Intent::Run(expected));
    }

    #[test]
    fn parse_d_and_u_together() {
        let mut buf = Vec::new();
        let mut expected = default_config();
        expected.update = true;
        expected.diff = true;
        assert_eq!(
            parse(&args(&["rx", "-d", "-u"]), &mut buf),
            Intent::Run(expected)
        );
    }

    #[test]
    fn parse_combined_flags_and_values() {
        let mut buf = Vec::new();
        let mut expected = default_config();
        expected.repo = String::from("/my/repo");
        expected.hostname = String::from("myhost");
        expected.keep = 3;
        expected.update = true;
        expected.diff = true;
        assert_eq!(
            parse(
                &args(&[
                    "rx", "-d", "-u", "-r", "/my/repo", "-n", "myhost", "-k", "3",
                ]),
                &mut buf,
            ),
            Intent::Run(expected)
        );
    }

    #[test]
    fn parse_k_then_h_early_returns_help() {
        let mut buf = Vec::new();
        assert_eq!(
            parse(&args(&["rx", "-k", "5", "-h"]), &mut buf),
            Intent::Help
        );
    }

    #[test]
    fn parse_unknown_flag_returns_error() {
        let mut buf = Vec::new();
        assert_eq!(parse(&args(&["rx", "-x"]), &mut buf), Intent::Invalid);
        assert!(String::from_utf8_lossy(&buf).contains("Unknown flag"));
    }

    #[test]
    fn parse_extra_non_flag_arg_ignored() {
        let mut buf = Vec::new();
        let mut expected = default_config();
        expected.diff = true;
        assert_eq!(
            parse(&args(&["rx", "-d", "extra"]), &mut buf),
            Intent::Run(expected)
        );
    }

    // ------------------------------------------------------------------
    // long flags
    // ------------------------------------------------------------------

    #[test]
    fn parse_help_long_returns_help() {
        let mut buf = Vec::new();
        assert_eq!(parse(&args(&["rx", "--help"]), &mut buf), Intent::Help);
    }

    #[test]
    fn parse_version_long_returns_version() {
        let mut buf = Vec::new();
        assert_eq!(
            parse(&args(&["rx", "--version"]), &mut buf),
            Intent::Version
        );
    }

    #[test]
    fn parse_update_long_sets_update() {
        let mut buf = Vec::new();
        let mut expected = default_config();
        expected.update = true;
        assert_eq!(
            parse(&args(&["rx", "--update"]), &mut buf),
            Intent::Run(expected)
        );
    }

    #[test]
    fn parse_diff_long_sets_diff() {
        let mut buf = Vec::new();
        let mut expected = default_config();
        expected.diff = true;
        assert_eq!(
            parse(&args(&["rx", "--diff"]), &mut buf),
            Intent::Run(expected)
        );
    }

    #[test]
    fn parse_repo_long_sets_repo() {
        let mut buf = Vec::new();
        let mut expected = default_config();
        expected.repo = String::from("/my/repo");
        assert_eq!(
            parse(&args(&["rx", "--repo", "/my/repo"]), &mut buf),
            Intent::Run(expected)
        );
    }

    #[test]
    fn parse_hostname_long_sets_hostname() {
        let mut buf = Vec::new();
        let mut expected = default_config();
        expected.hostname = String::from("myhost");
        assert_eq!(
            parse(&args(&["rx", "--hostname", "myhost"]), &mut buf),
            Intent::Run(expected)
        );
    }

    #[test]
    fn parse_keep_long_sets_keep() {
        let mut buf = Vec::new();
        let mut expected = default_config();
        expected.keep = 5;
        assert_eq!(
            parse(&args(&["rx", "--keep", "5"]), &mut buf),
            Intent::Run(expected)
        );
    }

    #[test]
    fn parse_keep_long_non_numeric_returns_error() {
        let mut buf = Vec::new();
        assert_eq!(
            parse(&args(&["rx", "--keep", "abc"]), &mut buf),
            Intent::Invalid
        );
        assert!(String::from_utf8_lossy(&buf).contains("not numeric"));
    }

    #[test]
    fn parse_keep_long_zero_returns_error() {
        let mut buf = Vec::new();
        assert_eq!(
            parse(&args(&["rx", "--keep", "0"]), &mut buf),
            Intent::Invalid
        );
        assert!(String::from_utf8_lossy(&buf).contains("at least 1"));
    }

    #[test]
    fn parse_repo_long_missing_value_returns_error() {
        let mut buf = Vec::new();
        assert_eq!(parse(&args(&["rx", "--repo"]), &mut buf), Intent::Invalid);
        assert!(String::from_utf8_lossy(&buf).contains("requires an argument"));
    }

    #[test]
    fn parse_unknown_long_flag_returns_error() {
        let mut buf = Vec::new();
        assert_eq!(
            parse(&args(&["rx", "--unknown"]), &mut buf),
            Intent::Invalid
        );
        assert!(String::from_utf8_lossy(&buf).contains("Unknown flag"));
    }
}
