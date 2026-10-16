use std::io::Write;

use crate::ansi;
use crate::cli::Intent;
use crate::cli::defaults::{default_config, expand_tilde};

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

    if args.len() == 2 {
        match args[1].as_str() {
            "help" => return Intent::Help,
            "version" => return Intent::Version,
            _ => {}
        }
    }

    if !args[1].starts_with('-') {
        write_parse_error(writer, &format!("Unknown argument \"{}\"", args[1]));
        return Intent::Invalid;
    }

    let mut config = default_config();
    let mut i = 1;
    while i < args.len() {
        let arg = &args[i];
        if arg.starts_with("--") {
            // Long flags
            if arg == "--help" {
                return Intent::Help;
            } else if arg == "--version" {
                return Intent::Version;
            } else if arg == "--update" {
                config.update = true;
            } else if arg == "--diff" {
                config.diff = true;
            } else if arg == "--repo" || arg == "--hostname" || arg == "--keep" {
                if i + 1 >= args.len() {
                    write_parse_error(writer, &format!("\"{arg}\" flag requires an argument"));
                    return Intent::Invalid;
                }
                let value = args[i + 1].clone();
                if arg == "--repo" {
                    config.repo = expand_tilde(&value);
                } else if arg == "--hostname" {
                    config.hostname = value;
                } else if let Ok(num) = value.parse::<u8>() {
                    config.keep = num;
                } else {
                    write_parse_error(writer, "Value of \"--keep\" flag is not numeric.");
                    return Intent::Invalid;
                }
                i += 1;
            } else {
                write_parse_error(writer, &format!("Unknown flag \"{arg}\""));
                return Intent::Invalid;
            }
        } else if arg.starts_with('-') {
            // Short flags
            let flags = arg.strip_prefix('-').unwrap_or(arg);
            let mut skip_next = false;
            for flag_char in flags.chars() {
                match flag_char {
                    'h' => return Intent::Help,
                    'v' => return Intent::Version,
                    'd' => config.diff = true,
                    'u' => config.update = true,
                    'r' | 'n' | 'k' => {
                        if i + 1 >= args.len() {
                            write_parse_error(
                                writer,
                                &format!("\"-{flag_char}\" flag requires an argument"),
                            );
                            return Intent::Invalid;
                        }
                        let value = args[i + 1].clone();
                        if flag_char == 'r' {
                            config.repo = expand_tilde(&value);
                        } else if flag_char == 'n' {
                            config.hostname = value;
                        } else if let Ok(num) = value.parse::<u8>() {
                            config.keep = num;
                        } else {
                            write_parse_error(writer, "Value of \"-k\" flag is not numeric.");
                            return Intent::Invalid;
                        }
                        skip_next = true;
                        break;
                    }
                    _ => {
                        write_parse_error(writer, &format!("Unknown flag \"-{flag_char}\""));
                        return Intent::Invalid;
                    }
                }
            }
            if skip_next {
                i += 1;
            }
        }
        i += 1;
    }

    Intent::Run(config)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cli::defaults::default_config;

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
