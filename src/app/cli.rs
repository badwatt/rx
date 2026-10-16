use std::io::Write;

use crate::core::ansi;
use crate::core::commands;
use crate::error::Error;

use crate::app::init::Config;

/// Trait abstracting external dependencies for testability.
///
/// `RealDeps` wires real stdin/stdout/process; tests use mock impls.
pub trait Deps {
    /// Run a command via argv (no shell, no injection risk).
    fn run_cmd(&self, cmd: &[String], output: bool) -> Result<i32, Error>;
    /// Run a shell pipeline string (only for commands requiring pipes).
    fn run_pipeline(&self, cmd: &str, output: bool) -> Result<i32, Error>;
    fn confirm(&self, default: bool, msg: Option<&str>) -> Result<bool, Error>;
    fn print_title(&self, text: &str) -> Result<(), Error>;
    fn config_print(&self, config: &Config) -> Result<(), Error>;
}

/// Zero-sized production dependency container.
pub struct RealDeps;

impl Deps for RealDeps {
    fn run_cmd(&self, cmd: &[String], output: bool) -> Result<i32, Error> {
        crate::core::process::run_cmd(cmd, output)
    }

    fn run_pipeline(&self, cmd: &str, output: bool) -> Result<i32, Error> {
        crate::core::process::run_pipeline(cmd, output)
    }

    fn confirm(&self, default: bool, msg: Option<&str>) -> Result<bool, Error> {
        let stdin = std::io::stdin();
        let mut reader = stdin.lock();
        let mut stdout = std::io::stdout();
        crate::core::ui::confirm(&mut reader, &mut stdout, default, msg)
    }

    fn print_title(&self, text: &str) -> Result<(), Error> {
        let mut stdout = std::io::stdout();
        crate::core::ui::print_title(&mut stdout, text)
    }

    fn config_print(&self, config: &Config) -> Result<(), Error> {
        let mut stdout = std::io::stdout();
        crate::app::init::config_print(&mut stdout, config)
    }
}

fn print_error(writer: &mut dyn Write, msg: &str) -> Result<(), Error> {
    ansi::write_flush(writer, &format!("{}{}{}\n", ansi::RED, msg, ansi::RESET))?;
    Ok(())
}

fn print_success(writer: &mut dyn Write, msg: &str) -> Result<(), Error> {
    ansi::write_flush(writer, &format!("{}{}{}\n", ansi::GREEN, msg, ansi::RESET))?;
    Ok(())
}

fn pull(writer: &mut dyn Write, repo: &str, deps: &dyn Deps) -> Result<(), Error> {
    deps.print_title("Git Pull")?;
    let status = deps.run_cmd(&commands::git_pull(repo), true)?;
    if status != 0 {
        print_error(writer, "Failed to pull changes")?;
        return Err(Error::GitPullFailed);
    }
    Ok(())
}

fn update(config: &Config, deps: &dyn Deps) -> Result<(), Error> {
    deps.print_title("Nix Update")?;
    deps.run_cmd(&commands::nix_update(&config.repo), true)?;
    Ok(())
}

fn stage(writer: &mut dyn Write, repo: &str, deps: &dyn Deps) -> Result<(), Error> {
    let diff_status = deps.run_cmd(&commands::git_diff(repo), false)?;
    if diff_status != 1 {
        return Ok(());
    }
    deps.print_title("Git Changes")?;
    deps.run_cmd(&commands::git_status(repo), true)?;
    if !deps.confirm(true, Some("Do you want to add these changes to the stage?"))? {
        print_error(writer, "Changes not added to stage")?;
        return Ok(());
    }
    match deps.run_cmd(&commands::git_add(repo), true) {
        Ok(_) => print_success(writer, "Changes added to git stage successfully")?,
        Err(e) => {
            ansi::write_flush(
                writer,
                &format!("Failed to add changes to the stage: {e}\n"),
            )?;
        }
    }
    Ok(())
}

fn rebuild(config: &Config, deps: &dyn Deps) -> Result<(), Error> {
    deps.print_title("Nixos Rebuild")?;
    deps.run_cmd(&commands::nix_rebuild(&config.repo, &config.hostname), true)?;
    Ok(())
}

fn cleanup(config: &Config, deps: &dyn Deps) -> Result<(), Error> {
    deps.run_cmd(&commands::nix_keep(config.keep), true)?;
    Ok(())
}

fn show_diff(deps: &dyn Deps) -> Result<(), Error> {
    deps.print_title("Nix Diff")?;
    deps.run_pipeline(commands::nix_diff(), true)?;
    Ok(())
}

/// Main CLI workflow: print config, confirm, pull, update, diff, rebuild.
pub fn cli(writer: &mut dyn Write, config: &Config, deps: &dyn Deps) -> Result<(), Error> {
    deps.print_title("RX Configuration")?;
    deps.config_print(config)?;

    if !deps.confirm(true, None)? {
        return Ok(());
    }

    pull(writer, &config.repo, deps)?;

    if config.update {
        update(config, deps)?;
    }

    stage(writer, &config.repo, deps)?;
    rebuild(config, deps)?;
    cleanup(config, deps)?;

    if config.diff {
        show_diff(deps)?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io;

    type RunCmdFn = Box<dyn Fn(&[String], bool) -> Result<i32, Error>>;
    type RunPipelineFn = Box<dyn Fn(&str, bool) -> Result<i32, Error>>;
    type ConfirmFn = Box<dyn Fn(bool, Option<&str>) -> Result<bool, Error>>;
    type PrintTitleFn = Box<dyn Fn(&str) -> Result<(), Error>>;
    type ConfigPrintFn = Box<dyn Fn(&Config) -> Result<(), Error>>;

    struct MockDeps {
        run_cmd_result: RunCmdFn,
        run_pipeline_result: RunPipelineFn,
        confirm_result: ConfirmFn,
        print_title_fn: PrintTitleFn,
        config_print_fn: ConfigPrintFn,
    }

    impl Deps for MockDeps {
        fn run_cmd(&self, cmd: &[String], output: bool) -> Result<i32, Error> {
            (self.run_cmd_result)(cmd, output)
        }
        fn run_pipeline(&self, cmd: &str, output: bool) -> Result<i32, Error> {
            (self.run_pipeline_result)(cmd, output)
        }
        fn confirm(&self, default: bool, msg: Option<&str>) -> Result<bool, Error> {
            (self.confirm_result)(default, msg)
        }
        fn print_title(&self, text: &str) -> Result<(), Error> {
            (self.print_title_fn)(text)
        }
        fn config_print(&self, config: &Config) -> Result<(), Error> {
            (self.config_print_fn)(config)
        }
    }

    fn default_config() -> Config {
        Config {
            repo: String::from("r"),
            hostname: String::from("h"),
            keep: 1,
            update: false,
            diff: false,
        }
    }

    fn default_config_diff() -> Config {
        Config {
            diff: true,
            ..default_config()
        }
    }

    fn mock_deps_ok() -> MockDeps {
        MockDeps {
            run_cmd_result: Box::new(|_, _| Ok(0)),
            run_pipeline_result: Box::new(|_, _| Ok(0)),
            confirm_result: Box::new(|_, _| Ok(true)),
            print_title_fn: Box::new(|_| Ok(())),
            config_print_fn: Box::new(|_| Ok(())),
        }
    }

    // ------------------------------------------------------------------
    // confirm path
    // ------------------------------------------------------------------

    #[test]
    fn cli_confirm_false_returns_early_ok() {
        let deps = MockDeps {
            confirm_result: Box::new(|_, _| Ok(false)),
            ..mock_deps_ok()
        };
        let mut buf = Vec::new();
        assert!(cli(&mut buf, &default_config(), &deps).is_ok());
    }

    #[test]
    fn cli_confirm_decline_add_changes_prints_not_added() {
        let deps = MockDeps {
            run_cmd_result: Box::new(|cmd, _| {
                if cmd.iter().any(|a| a.contains("--exit-code")) {
                    return Ok(1);
                }
                Ok(0)
            }),
            confirm_result: Box::new(|_, msg| if msg.is_some() { Ok(false) } else { Ok(true) }),
            ..mock_deps_ok()
        };
        let mut buf = Vec::new();
        assert!(cli(&mut buf, &default_config(), &deps).is_ok());
        let output = String::from_utf8_lossy(&buf);
        assert!(output.contains("Changes not added to stage"));
    }

    // ------------------------------------------------------------------
    // git pull
    // ------------------------------------------------------------------

    #[test]
    fn cli_git_pull_non_zero_returns_git_pull_failed() {
        let deps = MockDeps {
            run_cmd_result: Box::new(|_, _| Ok(1)),
            ..mock_deps_ok()
        };
        let mut buf = Vec::new();
        assert!(cli(&mut buf, &default_config(), &deps).is_err());
    }

    // ------------------------------------------------------------------
    // update / diff flags
    // ------------------------------------------------------------------

    #[test]
    fn cli_update_and_diff_true_runs_extra_commands() {
        let mut buf = Vec::new();
        let mut config = default_config();
        config.update = true;
        config.diff = true;
        assert!(cli(&mut buf, &config, &mock_deps_ok()).is_ok());
    }

    // ------------------------------------------------------------------
    // git diff / changes
    // ------------------------------------------------------------------

    #[test]
    fn cli_no_git_changes_skips_stage() {
        let deps = MockDeps {
            run_cmd_result: Box::new(|_, _| Ok(0)),
            ..mock_deps_ok()
        };
        let mut buf = Vec::new();
        assert!(cli(&mut buf, &default_config(), &deps).is_ok());
    }

    #[test]
    fn cli_git_add_failure_catches_error() {
        let deps = MockDeps {
            run_cmd_result: Box::new(|cmd, _| {
                if cmd.iter().any(|a| a.contains("add")) {
                    return Err(Error::Io(std::io::Error::other("mock error")));
                }
                if cmd.iter().any(|a| a.contains("--exit-code")) {
                    return Ok(1);
                }
                Ok(0)
            }),
            ..mock_deps_ok()
        };
        let mut buf = Vec::new();
        assert!(cli(&mut buf, &default_config(), &deps).is_ok());
        let output = String::from_utf8_lossy(&buf);
        assert!(output.contains("Failed to add changes to the stage"));
    }

    #[test]
    fn cli_git_add_success_prints_success_message() {
        let deps = MockDeps {
            run_cmd_result: Box::new(|cmd, _| {
                if cmd.iter().any(|a| a.contains("--exit-code")) {
                    Ok(1)
                } else {
                    Ok(0)
                }
            }),
            confirm_result: Box::new(|_, _| Ok(true)),
            ..mock_deps_ok()
        };
        let mut buf = Vec::new();
        assert!(cli(&mut buf, &default_config(), &deps).is_ok());
        let output = String::from_utf8_lossy(&buf);
        assert!(output.contains("Changes added to git stage successfully"));
    }

    // ------------------------------------------------------------------
    // error propagation via crate::test_helpers::FailingWriter
    // ------------------------------------------------------------------

    #[test]
    fn failing_writer_flush_method_is_callable() {
        let mut writer = crate::test_helpers::FailingWriter;
        assert!(io::Write::flush(&mut writer).is_ok());
    }

    #[test]
    fn cli_git_pull_non_zero_and_writer_fails_returns_error() {
        let deps = MockDeps {
            run_cmd_result: Box::new(|_, _| Ok(1)),
            ..mock_deps_ok()
        };
        let mut writer = crate::test_helpers::FailingWriter;
        assert!(cli(&mut writer, &default_config(), &deps).is_err());
    }

    #[test]
    fn cli_run_cmd_error_propagates_from_git_pull() {
        let deps = MockDeps {
            run_cmd_result: Box::new(|_, _| Err(Error::GitPullFailed)),
            ..mock_deps_ok()
        };
        let mut buf = Vec::new();
        assert!(cli(&mut buf, &default_config(), &deps).is_err());
    }

    #[test]
    fn cli_print_title_error_propagates_from_first_call() {
        let deps = MockDeps {
            print_title_fn: Box::new(|_| Err(Error::GitPullFailed)),
            ..mock_deps_ok()
        };
        let mut buf = Vec::new();
        assert!(cli(&mut buf, &default_config(), &deps).is_err());
    }

    #[test]
    fn cli_config_print_error_propagates_from_deps() {
        let deps = MockDeps {
            config_print_fn: Box::new(|_| Err(Error::GitPullFailed)),
            ..mock_deps_ok()
        };
        let mut buf = Vec::new();
        assert!(cli(&mut buf, &default_config(), &deps).is_err());
    }

    #[test]
    fn cli_confirm_error_propagates_from_deps() {
        let deps = MockDeps {
            confirm_result: Box::new(|_, _| Err(Error::GitPullFailed)),
            ..mock_deps_ok()
        };
        let mut buf = Vec::new();
        assert!(cli(&mut buf, &default_config(), &deps).is_err());
    }

    #[test]
    fn cli_stage_decline_with_failing_writer() {
        let deps = MockDeps {
            run_cmd_result: Box::new(|cmd, _| {
                if cmd.iter().any(|a| a.contains("--exit-code")) {
                    Ok(1)
                } else {
                    Ok(0)
                }
            }),
            confirm_result: Box::new(|_, msg| Ok(msg.is_none())),
            ..mock_deps_ok()
        };
        let mut writer = crate::test_helpers::FailingWriter;
        assert!(cli(&mut writer, &default_config(), &deps).is_err());
    }

    #[test]
    fn cli_git_add_success_with_failing_writer() {
        let deps = MockDeps {
            run_cmd_result: Box::new(|cmd, _| {
                if cmd.iter().any(|a| a.contains("--exit-code")) {
                    Ok(1)
                } else {
                    Ok(0)
                }
            }),
            confirm_result: Box::new(|_, _| Ok(true)),
            ..mock_deps_ok()
        };
        let mut writer = crate::test_helpers::FailingWriter;
        assert!(cli(&mut writer, &default_config(), &deps).is_err());
    }

    #[test]
    fn cli_git_add_failure_with_failing_writer() {
        let deps = MockDeps {
            run_cmd_result: Box::new(|cmd, _| {
                if cmd.iter().any(|a| a.contains("add")) {
                    Err(Error::Io(std::io::Error::other("mock")))
                } else if cmd.iter().any(|a| a.contains("--exit-code")) {
                    Ok(1)
                } else {
                    Ok(0)
                }
            }),
            confirm_result: Box::new(|_, _| Ok(true)),
            ..mock_deps_ok()
        };
        let mut writer = crate::test_helpers::FailingWriter;
        assert!(cli(&mut writer, &default_config(), &deps).is_err());
    }

    // ------------------------------------------------------------------
    // RealDeps — exercise the thin glue layer
    // ------------------------------------------------------------------

    #[test]
    fn real_deps_run_cmd_true_returns_zero() {
        let deps = RealDeps;
        assert_eq!(deps.run_cmd(&["true".into()], false).expect("run true"), 0);
    }

    #[test]
    fn real_deps_run_pipeline_true_returns_zero() {
        let deps = RealDeps;
        assert_eq!(deps.run_pipeline("true", false).expect("run true"), 0);
    }

    #[test]
    fn real_deps_print_title_does_not_panic() {
        let deps = RealDeps;
        deps.print_title("Test").expect("print title");
    }

    #[test]
    fn real_deps_config_print_does_not_panic() {
        let deps = RealDeps;
        let config = Config {
            repo: String::from("r"),
            hostname: String::from("h"),
            keep: 1,
            update: false,
            diff: false,
        };
        deps.config_print(&config).expect("config print");
    }

    // ------------------------------------------------------------------
    // Error propagation — cover every `?` after the first confirm
    // ------------------------------------------------------------------

    fn deps_print_title_fails_on(text: &str) -> MockDeps {
        let text = text.to_owned();
        MockDeps {
            print_title_fn: Box::new(move |t| {
                if t == text {
                    Err(Error::GitPullFailed)
                } else {
                    Ok(())
                }
            }),
            ..mock_deps_ok()
        }
    }

    fn deps_run_cmd_fails_on(sub: &str) -> MockDeps {
        let sub = sub.to_owned();
        MockDeps {
            run_cmd_result: Box::new(move |cmd, _| {
                if cmd.iter().any(|a| a.contains(&sub)) {
                    Err(Error::GitPullFailed)
                } else {
                    Ok(0)
                }
            }),
            ..mock_deps_ok()
        }
    }

    fn deps_run_pipeline_fails_on() -> MockDeps {
        MockDeps {
            run_pipeline_result: Box::new(|_, _| Err(Error::GitPullFailed)),
            ..mock_deps_ok()
        }
    }

    #[test]
    fn cli_print_title_git_pull_error_propagates() {
        let mut buf = Vec::new();
        assert!(
            cli(
                &mut buf,
                &default_config(),
                &deps_print_title_fails_on("Git Pull"),
            )
            .is_err()
        );
    }

    #[test]
    fn cli_run_cmd_git_pull_error_propagates() {
        let mut buf = Vec::new();
        assert!(cli(&mut buf, &default_config(), &deps_run_cmd_fails_on("pull"),).is_err());
    }

    #[test]
    fn cli_print_title_nix_update_error_propagates() {
        let mut mut_config = default_config();
        mut_config.update = true;
        let mut buf = Vec::new();
        assert!(
            cli(
                &mut buf,
                &mut_config,
                &deps_print_title_fails_on("Nix Update"),
            )
            .is_err()
        );
    }

    #[test]
    fn cli_run_cmd_nix_update_error_propagates() {
        let mut config = default_config();
        config.update = true;
        let mut buf = Vec::new();
        assert!(cli(&mut buf, &config, &deps_run_cmd_fails_on("flake"),).is_err());
    }

    #[test]
    fn cli_run_cmd_git_diff_error_propagates() {
        let mut buf = Vec::new();
        // git diff returns exit code != 1, so stage is skipped — no error path
        // Use a mock that fails on diff
        let deps = MockDeps {
            run_cmd_result: Box::new(|cmd, _| {
                if cmd.iter().any(|a| a.contains("--exit-code")) {
                    Err(Error::GitPullFailed)
                } else {
                    Ok(0)
                }
            }),
            ..mock_deps_ok()
        };
        assert!(cli(&mut buf, &default_config(), &deps).is_err());
    }

    #[test]
    fn cli_print_title_git_changes_error_propagates() {
        let deps = MockDeps {
            run_cmd_result: Box::new(|cmd, _| {
                if cmd.iter().any(|a| a.contains("--exit-code")) {
                    Ok(1)
                } else {
                    Ok(0)
                }
            }),
            print_title_fn: Box::new(|t| {
                if t == "Git Changes" {
                    Err(Error::GitPullFailed)
                } else {
                    Ok(())
                }
            }),
            ..mock_deps_ok()
        };
        let mut buf = Vec::new();
        assert!(cli(&mut buf, &default_config(), &deps).is_err());
    }

    #[test]
    fn cli_run_cmd_git_status_error_propagates() {
        let deps = MockDeps {
            run_cmd_result: Box::new(|cmd, _| {
                if cmd.iter().any(|a| a.contains("--exit-code")) {
                    Ok(1)
                } else if cmd.iter().any(|a| a.contains("--porcelain")) {
                    Err(Error::GitPullFailed)
                } else {
                    Ok(0)
                }
            }),
            ..mock_deps_ok()
        };
        let mut buf = Vec::new();
        assert!(cli(&mut buf, &default_config(), &deps).is_err());
    }

    #[test]
    fn cli_confirm_message_error_propagates() {
        let deps = MockDeps {
            run_cmd_result: Box::new(|cmd, _| {
                if cmd.iter().any(|a| a.contains("--exit-code")) {
                    Ok(1)
                } else {
                    Ok(0)
                }
            }),
            confirm_result: Box::new(|_, msg| {
                if msg.is_some() {
                    Err(Error::GitPullFailed)
                } else {
                    Ok(true)
                }
            }),
            ..mock_deps_ok()
        };
        let mut buf = Vec::new();
        assert!(cli(&mut buf, &default_config(), &deps).is_err());
    }

    #[test]
    fn cli_run_cmd_nixrebuild_error_propagates() {
        let mut buf = Vec::new();
        assert!(
            cli(
                &mut buf,
                &default_config(),
                &deps_run_cmd_fails_on("nixos-rebuild"),
            )
            .is_err()
        );
    }

    #[test]
    fn cli_run_cmd_nixkeep_error_propagates() {
        let mut buf = Vec::new();
        assert!(
            cli(
                &mut buf,
                &default_config(),
                &deps_run_cmd_fails_on("nix-env"),
            )
            .is_err()
        );
    }

    #[test]
    fn cli_run_pipeline_nix_diff_error_propagates() {
        let mut buf = Vec::new();
        assert!(
            cli(
                &mut buf,
                &default_config_diff(),
                &deps_run_pipeline_fails_on(),
            )
            .is_err()
        );
    }

    #[test]
    fn cli_print_title_nix_rebuild_error_propagates() {
        let mut buf = Vec::new();
        assert!(
            cli(
                &mut buf,
                &default_config(),
                &deps_print_title_fails_on("Nixos Rebuild"),
            )
            .is_err()
        );
    }

    #[test]
    fn cli_print_title_nix_diff_error_propagates() {
        let mut buf = Vec::new();
        assert!(
            cli(
                &mut buf,
                &default_config_diff(),
                &deps_print_title_fails_on("Nix Diff"),
            )
            .is_err()
        );
    }
}
