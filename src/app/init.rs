use std::fmt;
use std::io::Write;

use crate::core::ansi;
use crate::error::Error;

const VERSION: &str = env!("CARGO_PKG_VERSION");
const DEFAULT_REPO: &str = "~/.dotfiles";
const DEFAULT_KEEP: u8 = 10;

/// Expand `~` at the start of a path to the user's home directory.
/// Returns the path unchanged if `~` is not the leading character
/// or if `$HOME` is not set.
fn expand_tilde(path: &str) -> String {
    if let Some(rest) = path.strip_prefix('~')
        && let Ok(home) = std::env::var("HOME")
    {
        format!("{home}{rest}")
    } else {
        path.to_owned()
    }
}

/// Parsed CLI configuration.
#[derive(Debug, PartialEq)]
pub struct Config {
    pub repo: String,
    pub hostname: String,
    pub keep: u8,
    pub update: bool,
    pub diff: bool,
}

impl Config {
    /// Build defaults: repo `~/.dotfiles`, hostname from `/proc/sys/kernel/hostname`,
    /// keep `10`, update/diff `false`.
    pub fn defaults() -> Self {
        Self {
            repo: expand_tilde(DEFAULT_REPO),
            hostname: default_hostname(),
            keep: DEFAULT_KEEP,
            update: false,
            diff: false,
        }
    }
}

impl fmt::Display for Config {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Config {{ repo: {}, hostname: {}, keep: {}, update: {}, diff: {} }}",
            self.repo, self.hostname, self.keep, self.update, self.diff
        )
    }
}

/// Return the hostname: `/proc/sys/kernel/hostname` → `HOSTNAME` env → `hostname` cmd → `"unknown"`.
fn default_hostname() -> String {
    default_hostname_from(
        || std::fs::read_to_string(default_hostname_path()).ok(),
        || std::env::var("HOSTNAME").ok(),
        hostname_from_cmd,
    )
}

/// Read hostname from `hostname` command output.
fn hostname_from_cmd() -> Option<String> {
    hostname_from_cmd_inner(std::process::Command::new("hostname").output().ok())
}

/// Inner logic extracted for testability.
fn hostname_from_cmd_inner(output: Option<std::process::Output>) -> Option<String> {
    let out = output?;
    String::from_utf8(out.stdout).ok()
}

#[cfg(not(test))]
fn default_hostname_path() -> &'static str {
    "/proc/sys/kernel/hostname"
}

#[cfg(test)]
fn default_hostname_path() -> &'static str {
    "/nonexistent_hostname_path"
}

/// Parameterized hostname resolution for testability.
///
/// `proc_hostname` tries to read `/proc/sys/kernel/hostname`,
/// `env_hostname` falls back to the `HOSTNAME` env var,
/// `cmd_hostname` falls back to `hostname` command output,
/// and all failing yields `"unknown"`.
fn default_hostname_from(
    proc_hostname: impl FnOnce() -> Option<String>,
    env_hostname: impl FnOnce() -> Option<String>,
    cmd_hostname: impl FnOnce() -> Option<String>,
) -> String {
    proc_hostname()
        .map(|s| s.trim().to_owned())
        .or_else(env_hostname)
        .or_else(cmd_hostname)
        .unwrap_or_else(|| String::from("unknown"))
}

/// Print the help banner.
pub fn print_help(writer: &mut dyn Write) -> Result<(), Error> {
    ansi::write_flush(
        writer,
        "\n*****************************************************\n RX - A simple CLI tool to update your nixos system\n*****************************************************\n-r, --repo        : set repo path (default is $HOME/.dotfiles)\n-n, --hostname    : set hostname (default is OS hostname)\n-k, --keep        : set generations to keep (default is 10)\n-u, --update      : set update to true (default is false)\n-d, --diff        : set diff to true (default is false)\n-h, --help        : Display this help message\n-v, --version     : Display the current version\n\n",
    )?;
    Ok(())
}

/// Print the version line.
pub fn print_version(writer: &mut dyn Write) -> Result<(), Error> {
    ansi::write_flush(
        writer,
        &format!("{}RX version: {}{}\n", ansi::YELLOW, VERSION, ansi::RESET),
    )?;
    Ok(())
}

fn print_config_line(
    writer: &mut dyn Write,
    label: &str,
    value: &str,
    new_line: bool,
) -> Result<(), Error> {
    ansi::write_flush(
        writer,
        &format!(
            "{}◉ {}{}{} = {}{}{}{}",
            ansi::CYAN,
            ansi::RED,
            label,
            ansi::RESET,
            ansi::CYAN,
            value,
            ansi::RESET,
            if new_line { "\n" } else { "" }
        ),
    )?;
    Ok(())
}

/// Print all `config` fields with ANSI styling.
pub fn config_print(writer: &mut dyn Write, config: &Config) -> Result<(), Error> {
    print_config_line(writer, "repo", &config.repo, true)?;
    print_config_line(writer, "hostname", &config.hostname, true)?;
    print_config_line(writer, "keep", &config.keep.to_string(), true)?;
    print_config_line(writer, "update", &config.update.to_string(), true)?;
    print_config_line(writer, "diff", &config.diff.to_string(), false)?;
    Ok(())
}

/// Result of parsing command-line arguments.
#[derive(Debug, PartialEq)]
pub enum Parsed {
    Help,
    Version,
    Run(Config),
    Invalid,
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
pub fn parse_args(args: &[String], writer: &mut dyn Write) -> Parsed {
    if args.len() <= 1 {
        return Parsed::Run(Config::defaults());
    }

    if args.len() == 2 {
        match args[1].as_str() {
            "help" => return Parsed::Help,
            "version" => return Parsed::Version,
            _ => {}
        }
    }

    if !args[1].starts_with('-') {
        write_parse_error(writer, &format!("Unknown argument \"{}\"", args[1]));
        return Parsed::Invalid;
    }

    let mut config = Config::defaults();
    let mut i = 1;
    while i < args.len() {
        let arg = &args[i];
        if arg.starts_with("--") {
            // Long flags
            if arg == "--help" {
                return Parsed::Help;
            } else if arg == "--version" {
                return Parsed::Version;
            } else if arg == "--update" {
                config.update = true;
            } else if arg == "--diff" {
                config.diff = true;
            } else if arg == "--repo" || arg == "--hostname" || arg == "--keep" {
                if i + 1 >= args.len() {
                    write_parse_error(writer, &format!("\"{arg}\" flag requires an argument"));
                    return Parsed::Invalid;
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
                    return Parsed::Invalid;
                }
                i += 1;
            } else {
                write_parse_error(writer, &format!("Unknown flag \"{arg}\""));
                return Parsed::Invalid;
            }
        } else if arg.starts_with('-') {
            // Short flags
            let flags = arg.strip_prefix('-').unwrap_or(arg);
            let mut skip_next = false;
            for flag_char in flags.chars() {
                match flag_char {
                    'h' => return Parsed::Help,
                    'v' => return Parsed::Version,
                    'd' => config.diff = true,
                    'u' => config.update = true,
                    'r' | 'n' | 'k' => {
                        if i + 1 >= args.len() {
                            write_parse_error(
                                writer,
                                &format!("\"-{flag_char}\" flag requires an argument"),
                            );
                            return Parsed::Invalid;
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
                            return Parsed::Invalid;
                        }
                        skip_next = true;
                        break;
                    }
                    _ => {
                        write_parse_error(writer, &format!("Unknown flag \"-{flag_char}\""));
                        return Parsed::Invalid;
                    }
                }
            }
            if skip_next {
                i += 1;
            }
        }
        i += 1;
    }

    Parsed::Run(config)
}

/// Dispatch parsed arguments to help, version, CLI workflow, or error.
pub fn run(
    args: &[String],
    writer: &mut dyn Write,
    deps: &dyn crate::app::cli::Deps,
) -> Result<(), Error> {
    match parse_args(args, writer) {
        Parsed::Help => print_help(writer),
        Parsed::Version => print_version(writer),
        Parsed::Invalid => Err(Error::InvalidArgs),
        Parsed::Run(config) => crate::app::cli::cli(writer, &config, deps),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::cli::Deps;
    use std::io;

    fn args(from: &[&str]) -> Vec<String> {
        from.iter().copied().map(str::to_string).collect()
    }

    fn test_config() -> Config {
        Config {
            repo: String::from("r"),
            hostname: String::from("h"),
            keep: 1,
            update: false,
            diff: false,
        }
    }

    // ------------------------------------------------------------------
    // print_help / print_version
    // ------------------------------------------------------------------

    #[test]
    fn print_help_contains_rx() {
        let mut buf = Vec::new();
        assert!(print_help(&mut buf).is_ok());
        let s = String::from_utf8_lossy(&buf);
        assert!(s.contains("RX"));
    }

    #[test]
    fn print_version_contains_semver() {
        let mut buf = Vec::new();
        assert!(print_version(&mut buf).is_ok());
        let s = String::from_utf8_lossy(&buf);
        assert!(s.contains(VERSION));
    }

    // ------------------------------------------------------------------
    // config_print
    // ------------------------------------------------------------------

    #[test]
    fn config_print_renders_all_fields() {
        let mut buf = Vec::new();
        let config = Config {
            repo: String::from("~/.dotfiles"),
            hostname: String::from("nixos"),
            keep: 10,
            update: false,
            diff: true,
        };
        assert!(config_print(&mut buf, &config).is_ok());
        let s = String::from_utf8_lossy(&buf);
        assert!(s.contains("repo"));
        assert!(s.contains("hostname"));
        assert!(s.contains("keep"));
        assert!(s.contains("update"));
        assert!(s.contains("diff"));
    }

    #[test]
    fn config_print_last_field_has_no_trailing_newline() {
        let mut buf = Vec::new();
        let config = Config {
            repo: String::from("r"),
            hostname: String::from("h"),
            keep: 1,
            update: true,
            diff: false,
        };
        assert!(config_print(&mut buf, &config).is_ok());
        let s = String::from_utf8_lossy(&buf);
        assert!(!s.ends_with('\n'));
    }

    // ------------------------------------------------------------------
    // expand_tilde
    // ------------------------------------------------------------------

    #[test]
    fn expand_tilde_expands_home() {
        let home = std::env::var("HOME").expect("HOME set");
        assert_eq!(expand_tilde("~/.dotfiles"), format!("{home}/.dotfiles"));
    }

    #[test]
    fn expand_tilde_leaves_absolute_path() {
        assert_eq!(expand_tilde("/absolute/path"), "/absolute/path");
    }

    #[test]
    fn expand_tilde_leaves_relative_path() {
        assert_eq!(expand_tilde("relative/path"), "relative/path");
    }

    #[test]
    fn expand_tilde_tilde_only() {
        let home = std::env::var("HOME").expect("HOME set");
        assert_eq!(expand_tilde("~"), home);
    }

    // ------------------------------------------------------------------
    // parse_args — early-return paths
    // ------------------------------------------------------------------

    #[test]
    fn parse_no_args_yields_defaults() {
        let mut buf = Vec::new();
        assert_eq!(
            parse_args(&args(&["rx"]), &mut buf),
            Parsed::Run(Config::defaults())
        );
    }

    #[test]
    fn parse_help_flag_returns_help() {
        let mut buf = Vec::new();
        assert_eq!(parse_args(&args(&["rx", "-h"]), &mut buf), Parsed::Help);
    }

    #[test]
    fn parse_help_word_returns_help() {
        let mut buf = Vec::new();
        assert_eq!(parse_args(&args(&["rx", "help"]), &mut buf), Parsed::Help);
    }

    #[test]
    fn parse_version_flag_returns_version() {
        let mut buf = Vec::new();
        assert_eq!(parse_args(&args(&["rx", "-v"]), &mut buf), Parsed::Version);
    }

    #[test]
    fn parse_version_word_returns_version() {
        let mut buf = Vec::new();
        assert_eq!(
            parse_args(&args(&["rx", "version"]), &mut buf),
            Parsed::Version
        );
    }

    #[test]
    fn parse_unknown_argument_returns_error() {
        let mut buf = Vec::new();
        assert_eq!(
            parse_args(&args(&["rx", "unknown"]), &mut buf),
            Parsed::Invalid
        );
        assert!(String::from_utf8_lossy(&buf).contains("Unknown argument"));
    }

    // ------------------------------------------------------------------
    // parse_args — value flags
    // ------------------------------------------------------------------

    #[test]
    fn parse_r_missing_value_returns_error() {
        let mut buf = Vec::new();
        assert_eq!(parse_args(&args(&["rx", "-r"]), &mut buf), Parsed::Invalid);
        assert!(String::from_utf8_lossy(&buf).contains("requires an argument"));
    }

    #[test]
    fn parse_n_missing_value_returns_error() {
        let mut buf = Vec::new();
        assert_eq!(parse_args(&args(&["rx", "-n"]), &mut buf), Parsed::Invalid);
        assert!(String::from_utf8_lossy(&buf).contains("requires an argument"));
    }

    #[test]
    fn parse_k_non_numeric_returns_error() {
        let mut buf = Vec::new();
        assert_eq!(
            parse_args(&args(&["rx", "-k", "abc"]), &mut buf),
            Parsed::Invalid
        );
        assert!(String::from_utf8_lossy(&buf).contains("not numeric"));
    }

    #[test]
    fn parse_r_with_value_sets_repo() {
        let mut buf = Vec::new();
        let mut expected = Config::defaults();
        expected.repo = String::from("/path/to/repo");
        assert_eq!(
            parse_args(&args(&["rx", "-r", "/path/to/repo"]), &mut buf),
            Parsed::Run(expected)
        );
    }

    #[test]
    fn parse_n_with_value_sets_hostname() {
        let mut buf = Vec::new();
        let mut expected = Config::defaults();
        expected.hostname = String::from("myhost");
        assert_eq!(
            parse_args(&args(&["rx", "-n", "myhost"]), &mut buf),
            Parsed::Run(expected)
        );
    }

    #[test]
    fn parse_k_with_value_sets_keep() {
        let mut buf = Vec::new();
        let mut expected = Config::defaults();
        expected.keep = 5;
        assert_eq!(
            parse_args(&args(&["rx", "-k", "5"]), &mut buf),
            Parsed::Run(expected)
        );
    }

    // ------------------------------------------------------------------
    // parse_args — boolean flags
    // ------------------------------------------------------------------

    #[test]
    fn parse_d_flag_sets_diff() {
        let mut buf = Vec::new();
        let mut expected = Config::defaults();
        expected.diff = true;
        assert_eq!(
            parse_args(&args(&["rx", "-d"]), &mut buf),
            Parsed::Run(expected)
        );
    }

    #[test]
    fn parse_u_flag_sets_update() {
        let mut buf = Vec::new();
        let mut expected = Config::defaults();
        expected.update = true;
        assert_eq!(
            parse_args(&args(&["rx", "-u"]), &mut buf),
            Parsed::Run(expected)
        );
    }

    #[test]
    fn parse_d_and_u_together() {
        let mut buf = Vec::new();
        let mut expected = Config::defaults();
        expected.update = true;
        expected.diff = true;
        assert_eq!(
            parse_args(&args(&["rx", "-d", "-u"]), &mut buf),
            Parsed::Run(expected)
        );
    }

    #[test]
    fn parse_combined_flags_and_values() {
        let mut buf = Vec::new();
        let mut expected = Config::defaults();
        expected.repo = String::from("/my/repo");
        expected.hostname = String::from("myhost");
        expected.keep = 3;
        expected.update = true;
        expected.diff = true;
        assert_eq!(
            parse_args(
                &args(&[
                    "rx", "-d", "-u", "-r", "/my/repo", "-n", "myhost", "-k", "3",
                ]),
                &mut buf,
            ),
            Parsed::Run(expected)
        );
    }

    #[test]
    fn parse_k_then_h_early_returns_help() {
        let mut buf = Vec::new();
        assert_eq!(
            parse_args(&args(&["rx", "-k", "5", "-h"]), &mut buf),
            Parsed::Help
        );
    }

    #[test]
    fn parse_unknown_flag_returns_error() {
        let mut buf = Vec::new();
        assert_eq!(parse_args(&args(&["rx", "-x"]), &mut buf), Parsed::Invalid);
        assert!(String::from_utf8_lossy(&buf).contains("Unknown flag"));
    }

    #[test]
    fn parse_extra_non_flag_arg_ignored() {
        let mut buf = Vec::new();
        let mut expected = Config::defaults();
        expected.diff = true;
        assert_eq!(
            parse_args(&args(&["rx", "-d", "extra"]), &mut buf),
            Parsed::Run(expected)
        );
    }

    // ------------------------------------------------------------------
    // parse_args — long flags
    // ------------------------------------------------------------------

    #[test]
    fn parse_help_long_returns_help() {
        let mut buf = Vec::new();
        assert_eq!(parse_args(&args(&["rx", "--help"]), &mut buf), Parsed::Help);
    }

    #[test]
    fn parse_version_long_returns_version() {
        let mut buf = Vec::new();
        assert_eq!(
            parse_args(&args(&["rx", "--version"]), &mut buf),
            Parsed::Version
        );
    }

    #[test]
    fn parse_update_long_sets_update() {
        let mut buf = Vec::new();
        let mut expected = Config::defaults();
        expected.update = true;
        assert_eq!(
            parse_args(&args(&["rx", "--update"]), &mut buf),
            Parsed::Run(expected)
        );
    }

    #[test]
    fn parse_diff_long_sets_diff() {
        let mut buf = Vec::new();
        let mut expected = Config::defaults();
        expected.diff = true;
        assert_eq!(
            parse_args(&args(&["rx", "--diff"]), &mut buf),
            Parsed::Run(expected)
        );
    }

    #[test]
    fn parse_repo_long_sets_repo() {
        let mut buf = Vec::new();
        let mut expected = Config::defaults();
        expected.repo = String::from("/my/repo");
        assert_eq!(
            parse_args(&args(&["rx", "--repo", "/my/repo"]), &mut buf),
            Parsed::Run(expected)
        );
    }

    #[test]
    fn parse_hostname_long_sets_hostname() {
        let mut buf = Vec::new();
        let mut expected = Config::defaults();
        expected.hostname = String::from("myhost");
        assert_eq!(
            parse_args(&args(&["rx", "--hostname", "myhost"]), &mut buf),
            Parsed::Run(expected)
        );
    }

    #[test]
    fn parse_keep_long_sets_keep() {
        let mut buf = Vec::new();
        let mut expected = Config::defaults();
        expected.keep = 5;
        assert_eq!(
            parse_args(&args(&["rx", "--keep", "5"]), &mut buf),
            Parsed::Run(expected)
        );
    }

    #[test]
    fn parse_keep_long_non_numeric_returns_error() {
        let mut buf = Vec::new();
        assert_eq!(
            parse_args(&args(&["rx", "--keep", "abc"]), &mut buf),
            Parsed::Invalid
        );
        assert!(String::from_utf8_lossy(&buf).contains("not numeric"));
    }

    #[test]
    fn parse_repo_long_missing_value_returns_error() {
        let mut buf = Vec::new();
        assert_eq!(
            parse_args(&args(&["rx", "--repo"]), &mut buf),
            Parsed::Invalid
        );
        assert!(String::from_utf8_lossy(&buf).contains("requires an argument"));
    }

    #[test]
    fn parse_unknown_long_flag_returns_error() {
        let mut buf = Vec::new();
        assert_eq!(
            parse_args(&args(&["rx", "--unknown"]), &mut buf),
            Parsed::Invalid
        );
        assert!(String::from_utf8_lossy(&buf).contains("Unknown flag"));
    }
    // ------------------------------------------------------------------

    #[test]
    fn run_help_flag_routes_to_help() {
        let mut buf = Vec::new();
        run(&args(&["rx", "-h"]), &mut buf, &MockCliDeps).expect("run help");
        assert!(String::from_utf8_lossy(&buf).contains("RX"));
    }

    #[test]
    fn run_version_flag_routes_to_version() {
        let mut buf = Vec::new();
        run(&args(&["rx", "-v"]), &mut buf, &MockCliDeps).expect("run version");
    }

    #[test]
    fn run_unknown_flag_returns_error() {
        let mut buf = Vec::new();
        let result = run(&args(&["rx", "-x"]), &mut buf, &MockCliDeps);
        assert!(result.is_err());
        assert!(
            result
                .expect_err("invalid args")
                .to_string()
                .contains("Invalid arguments")
        );
    }

    #[test]
    fn run_no_args_reaches_cli() {
        let mut buf = Vec::new();
        run(&args(&["rx"]), &mut buf, &MockCliDeps).expect("run no args");
    }

    #[test]
    fn run_with_d_and_u_reaches_cli() {
        let mut buf = Vec::new();
        run(&args(&["rx", "-d", "-u"]), &mut buf, &MockCliDeps).expect("run flags");
    }

    // ------------------------------------------------------------------
    // Config / hostname
    // ------------------------------------------------------------------

    #[test]
    fn config_defaults_repo_and_keep() {
        let c = Config::defaults();
        let home = std::env::var("HOME").expect("HOME set");
        assert_eq!(c.repo, format!("{home}/.dotfiles"));
        assert_eq!(c.keep, DEFAULT_KEEP);
    }

    #[test]
    fn default_hostname_is_non_empty() {
        let c = Config::defaults();
        assert!(!c.hostname.is_empty());
    }

    #[test]
    fn config_display_contains_repo() {
        let c = Config {
            repo: String::from("repo"),
            hostname: String::from("host"),
            keep: 5,
            update: true,
            diff: false,
        };
        assert!(c.to_string().contains("repo"));
    }

    #[test]
    fn default_hostname_from_proc_succeeds() {
        let result = default_hostname_from(|| Some(String::from("myhost\n")), || None, || None);
        assert_eq!(result, "myhost");
    }

    #[test]
    fn default_hostname_from_falls_back_to_env() {
        let result = default_hostname_from(|| None, || Some(String::from("envhost")), || None);
        assert_eq!(result, "envhost");
    }

    #[test]
    fn default_hostname_from_falls_back_to_cmd() {
        let result = default_hostname_from(|| None, || None, || Some(String::from("cmdhost")));
        assert_eq!(result, "cmdhost");
    }

    #[test]
    fn default_hostname_from_falls_back_to_unknown() {
        let result = default_hostname_from(|| None, || None, || None);
        assert_eq!(result, "unknown");
    }

    // ------------------------------------------------------------------
    // error propagation via crate::test_helpers::FailingWriter
    // ------------------------------------------------------------------

    #[test]
    fn failing_writer_flush_returns_ok() {
        let mut w = crate::test_helpers::FailingWriter;
        assert!(io::Write::flush(&mut w).is_ok());
    }

    struct CountingWriter {
        fail_after: usize,
        count: usize,
    }

    impl Write for CountingWriter {
        fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
            self.count += 1;
            if self.count >= self.fail_after {
                Err(io::Error::other("fail"))
            } else {
                Ok(buf.len())
            }
        }
        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }

    #[test]
    fn print_help_error_propagation() {
        let mut writer = crate::test_helpers::FailingWriter;
        assert!(print_help(&mut writer).is_err());
    }

    #[test]
    fn print_version_error_propagation() {
        let mut writer = crate::test_helpers::FailingWriter;
        assert!(print_version(&mut writer).is_err());
    }

    #[test]
    fn config_print_error_propagation_line_1() {
        let mut writer = CountingWriter {
            fail_after: 1,
            count: 0,
        };
        let config = test_config();
        assert!(config_print(&mut writer, &config).is_err());
    }

    #[test]
    fn config_print_error_propagation_line_2() {
        let mut writer = CountingWriter {
            fail_after: 2,
            count: 0,
        };
        let config = test_config();
        assert!(config_print(&mut writer, &config).is_err());
    }

    #[test]
    fn config_print_error_propagation_line_3() {
        let mut writer = CountingWriter {
            fail_after: 3,
            count: 0,
        };
        let config = test_config();
        assert!(config_print(&mut writer, &config).is_err());
    }

    #[test]
    fn config_print_error_propagation_line_4() {
        let mut writer = CountingWriter {
            fail_after: 4,
            count: 0,
        };
        let config = test_config();
        assert!(config_print(&mut writer, &config).is_err());
    }

    #[test]
    fn config_print_error_propagation_line_5() {
        let mut writer = CountingWriter {
            fail_after: 5,
            count: 0,
        };
        let config = test_config();
        assert!(config_print(&mut writer, &config).is_err());
    }

    #[test]
    fn print_config_line_error_propagation() {
        let mut writer = crate::test_helpers::FailingWriter;
        assert!(print_config_line(&mut writer, "k", "v", true).is_err());
    }

    #[test]
    fn default_hostname_runs_without_panic() {
        let _ = default_hostname();
    }

    #[test]
    fn hostname_from_cmd_inner_returns_some_on_valid_output() {
        use std::os::unix::process::ExitStatusExt;
        let output = Some(std::process::Output {
            status: std::process::ExitStatus::from_raw(0),
            stdout: b"myhost\n".to_vec(),
            stderr: Vec::new(),
        });
        let result = hostname_from_cmd_inner(output);
        assert!(result.is_some());
        assert!(!result.expect("hostname").trim().is_empty());
    }

    #[test]
    fn hostname_from_cmd_inner_returns_none_on_bad_utf8() {
        use std::os::unix::process::ExitStatusExt;
        let output = std::process::Output {
            status: std::process::ExitStatus::from_raw(0),
            stdout: vec![0xff, 0xfe], // invalid UTF-8
            stderr: Vec::new(),
        };
        assert!(hostname_from_cmd_inner(Some(output)).is_none());
    }

    #[test]
    fn hostname_from_cmd_inner_returns_none_on_none() {
        assert!(hostname_from_cmd_inner(None).is_none());
    }

    #[test]
    fn config_display_full_format() {
        let c = Config {
            repo: String::from("r"),
            hostname: String::from("h"),
            keep: 1,
            update: true,
            diff: false,
        };
        assert_eq!(
            c.to_string(),
            "Config { repo: r, hostname: h, keep: 1, update: true, diff: false }"
        );
    }

    // ------------------------------------------------------------------
    // shared mock deps
    // ------------------------------------------------------------------

    struct MockCliDeps;

    impl Deps for MockCliDeps {
        fn run_cmd(&self, _: &[String], _: bool) -> Result<i32, Error> {
            Ok(0)
        }
        fn run_pipeline(&self, _: &str, _: bool) -> Result<i32, Error> {
            Ok(0)
        }
        fn confirm(&self, _: bool, _: Option<&str>) -> Result<bool, Error> {
            Ok(true)
        }
        fn print_title(&self, _: &str) -> Result<(), Error> {
            Ok(())
        }
        fn config_print(&self, _: &Config) -> Result<(), Error> {
            Ok(())
        }
    }
}
