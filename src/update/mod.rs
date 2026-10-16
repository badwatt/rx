//! Use-case slice: the NixOS update workflow.
//!
//! Orchestrates the update pipeline (pull → flake update → stage → rebuild →
//! cleanup → diff) through the [`CommandRunner`] and [`UserIo`] ports. Contains
//! no I/O of its own.

pub mod steps;

use crate::config::Config;
use crate::error::Error;
use crate::ports::command::CommandRunner;
use crate::ports::user_io::UserIo;

/// Run the full NixOS update workflow.
pub fn run(config: &Config, cmd: &dyn CommandRunner, ui: &dyn UserIo) -> Result<(), Error> {
    ui.title("RX Configuration")?;
    ui.config(config)?;

    if !ui.confirm(true, None)? {
        return Ok(());
    }

    steps::pull::run(&config.repo, cmd, ui)?;

    if config.update {
        steps::flake::run(config, cmd, ui)?;
    }

    steps::stage::run(&config.repo, cmd, ui)?;
    steps::rebuild::run(config, cmd, ui)?;
    steps::cleanup::run(config, cmd, ui)?;

    if config.diff {
        steps::diff::run(cmd, ui)?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;
    use std::rc::Rc;

    type RunFn = Box<dyn Fn(&[String], bool) -> Result<i32, Error>>;
    type PipelineFn = Box<dyn Fn(&str, bool) -> Result<i32, Error>>;
    type TitleFn = Box<dyn Fn(&str) -> Result<(), Error>>;
    type ConfirmFn = Box<dyn Fn(bool, Option<&str>) -> Result<bool, Error>>;
    type MsgFn = Box<dyn Fn(&str) -> Result<(), Error>>;
    type ConfigFn = Box<dyn Fn(&Config) -> Result<(), Error>>;

    struct MockCmd {
        run: RunFn,
        pipeline: PipelineFn,
    }

    impl CommandRunner for MockCmd {
        fn run(&self, cmd: &[String], output: bool) -> Result<i32, Error> {
            (self.run)(cmd, output)
        }
        fn run_pipeline(&self, cmd: &str, output: bool) -> Result<i32, Error> {
            (self.pipeline)(cmd, output)
        }
    }

    struct MockUi {
        title: TitleFn,
        confirm: ConfirmFn,
        error: MsgFn,
        success: MsgFn,
        message: MsgFn,
        config: ConfigFn,
    }

    impl UserIo for MockUi {
        fn title(&self, text: &str) -> Result<(), Error> {
            (self.title)(text)
        }
        fn confirm(&self, default: bool, msg: Option<&str>) -> Result<bool, Error> {
            (self.confirm)(default, msg)
        }
        fn error(&self, msg: &str) -> Result<(), Error> {
            (self.error)(msg)
        }
        fn success(&self, msg: &str) -> Result<(), Error> {
            (self.success)(msg)
        }
        fn message(&self, msg: &str) -> Result<(), Error> {
            (self.message)(msg)
        }
        fn config(&self, config: &Config) -> Result<(), Error> {
            (self.config)(config)
        }
    }

    #[derive(Clone, Default)]
    struct Log(Rc<RefCell<Vec<String>>>);

    impl Log {
        fn record(&self, msg: &str) {
            self.0.borrow_mut().push(msg.to_owned());
        }
        fn contains(&self, needle: &str) -> bool {
            self.0.borrow().iter().any(|m| m.contains(needle))
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

    fn ok_cmd() -> MockCmd {
        MockCmd {
            run: Box::new(|_, _| Ok(0)),
            pipeline: Box::new(|_, _| Ok(0)),
        }
    }

    /// UI where every method succeeds and error/success/message are logged.
    fn ok_ui(log: &Log) -> MockUi {
        let error_log = log.clone();
        let success_log = log.clone();
        let message_log = log.clone();
        MockUi {
            title: Box::new(|_| Ok(())),
            confirm: Box::new(|_, _| Ok(true)),
            error: Box::new(move |m| {
                error_log.record(m);
                Ok(())
            }),
            success: Box::new(move |m| {
                success_log.record(m);
                Ok(())
            }),
            message: Box::new(move |m| {
                message_log.record(m);
                Ok(())
            }),
            config: Box::new(|_| Ok(())),
        }
    }

    fn cmd_fails_on(sub: &str) -> MockCmd {
        let sub = sub.to_owned();
        MockCmd {
            run: Box::new(move |cmd, _| {
                if cmd.iter().any(|a| a.contains(&sub)) {
                    Err(Error::GitPullFailed)
                } else {
                    Ok(0)
                }
            }),
            pipeline: Box::new(|_, _| Ok(0)),
        }
    }

    fn title_fails_on(text: &str) -> MockUi {
        let text = text.to_owned();
        MockUi {
            title: Box::new(move |t| {
                if t == text {
                    Err(Error::GitPullFailed)
                } else {
                    Ok(())
                }
            }),
            ..ok_ui(&Log::default())
        }
    }

    /// Command mock where `git diff` reports changes (exit 1) and everything
    /// else succeeds.
    fn cmd_with_changes() -> MockCmd {
        MockCmd {
            run: Box::new(|cmd, _| {
                if cmd.iter().any(|a| a.contains("--exit-code")) {
                    Ok(1)
                } else {
                    Ok(0)
                }
            }),
            pipeline: Box::new(|_, _| Ok(0)),
        }
    }

    // ------------------------------------------------------------------
    // happy paths
    // ------------------------------------------------------------------

    #[test]
    fn run_confirm_false_returns_early_ok() {
        let ui = MockUi {
            confirm: Box::new(|_, _| Ok(false)),
            ..ok_ui(&Log::default())
        };
        assert!(run(&default_config(), &ok_cmd(), &ui).is_ok());
    }

    #[test]
    fn run_no_git_changes_skips_stage() {
        let log = Log::default();
        let ui = ok_ui(&log);
        let cmd = MockCmd {
            run: Box::new(|_, _| Ok(0)),
            pipeline: Box::new(|_, _| Ok(0)),
        };
        assert!(run(&default_config(), &cmd, &ui).is_ok());
        assert!(!log.contains("Changes added"));
    }

    #[test]
    fn run_update_and_diff_true_runs_extra_commands() {
        let config = Config {
            update: true,
            diff: true,
            ..default_config()
        };
        assert!(run(&config, &ok_cmd(), &ok_ui(&Log::default())).is_ok());
    }

    #[test]
    fn run_stage_add_success_prints_success() {
        let log = Log::default();
        let ui = ok_ui(&log);
        assert!(run(&default_config(), &cmd_with_changes(), &ui).is_ok());
        assert!(log.contains("Changes added to git stage successfully"));
    }

    #[test]
    fn run_stage_decline_prints_not_added() {
        let log = Log::default();
        let ui = MockUi {
            confirm: Box::new(|_, msg| if msg.is_some() { Ok(false) } else { Ok(true) }),
            ..ok_ui(&log)
        };
        assert!(run(&default_config(), &cmd_with_changes(), &ui).is_ok());
        assert!(log.contains("Changes not added to stage"));
    }

    #[test]
    fn run_stage_add_failure_prints_failure_message() {
        let log = Log::default();
        let ui = ok_ui(&log);
        let cmd = MockCmd {
            run: Box::new(|cmd, _| {
                if cmd.iter().any(|a| a.contains("add")) {
                    Err(Error::Io(std::io::Error::other("mock error")))
                } else if cmd.iter().any(|a| a.contains("--exit-code")) {
                    Ok(1)
                } else {
                    Ok(0)
                }
            }),
            pipeline: Box::new(|_, _| Ok(0)),
        };
        assert!(run(&default_config(), &cmd, &ui).is_ok());
        assert!(log.contains("Failed to add changes to the stage"));
    }

    // ------------------------------------------------------------------
    // git pull
    // ------------------------------------------------------------------

    #[test]
    fn run_git_pull_non_zero_returns_error() {
        let cmd = MockCmd {
            run: Box::new(|_, _| Ok(1)),
            pipeline: Box::new(|_, _| Ok(0)),
        };
        assert!(run(&default_config(), &cmd, &ok_ui(&Log::default())).is_err());
    }

    // ------------------------------------------------------------------
    // error propagation — top-level UI calls
    // ------------------------------------------------------------------

    #[test]
    fn run_title_error_propagates_from_first_call() {
        let ui = MockUi {
            title: Box::new(|_| Err(Error::GitPullFailed)),
            ..ok_ui(&Log::default())
        };
        assert!(run(&default_config(), &ok_cmd(), &ui).is_err());
    }

    #[test]
    fn run_config_error_propagates() {
        let ui = MockUi {
            config: Box::new(|_| Err(Error::GitPullFailed)),
            ..ok_ui(&Log::default())
        };
        assert!(run(&default_config(), &ok_cmd(), &ui).is_err());
    }

    #[test]
    fn run_confirm_error_propagates() {
        let ui = MockUi {
            confirm: Box::new(|_, _| Err(Error::GitPullFailed)),
            ..ok_ui(&Log::default())
        };
        assert!(run(&default_config(), &ok_cmd(), &ui).is_err());
    }

    // ------------------------------------------------------------------
    // per-step error propagation
    // ------------------------------------------------------------------

    #[test]
    fn run_pull_title_error_propagates() {
        let ui = title_fails_on("Git Pull");
        assert!(run(&default_config(), &ok_cmd(), &ui).is_err());
    }

    #[test]
    fn run_pull_cmd_error_propagates() {
        assert!(
            run(
                &default_config(),
                &cmd_fails_on("pull"),
                &ok_ui(&Log::default())
            )
            .is_err()
        );
    }

    #[test]
    fn run_pull_error_output_error_propagates() {
        let cmd = MockCmd {
            run: Box::new(|_, _| Ok(1)),
            pipeline: Box::new(|_, _| Ok(0)),
        };
        let ui = MockUi {
            error: Box::new(|_| Err(Error::GitPullFailed)),
            ..ok_ui(&Log::default())
        };
        assert!(run(&default_config(), &cmd, &ui).is_err());
    }

    #[test]
    fn run_flake_title_error_propagates() {
        let config = Config {
            update: true,
            ..default_config()
        };
        let ui = title_fails_on("Nix Update");
        assert!(run(&config, &ok_cmd(), &ui).is_err());
    }

    #[test]
    fn run_flake_cmd_error_propagates() {
        let config = Config {
            update: true,
            ..default_config()
        };
        assert!(run(&config, &cmd_fails_on("flake"), &ok_ui(&Log::default())).is_err());
    }

    #[test]
    fn run_stage_diff_cmd_error_propagates() {
        let cmd = MockCmd {
            run: Box::new(|cmd, _| {
                if cmd.iter().any(|a| a.contains("--exit-code")) {
                    Err(Error::GitPullFailed)
                } else {
                    Ok(0)
                }
            }),
            pipeline: Box::new(|_, _| Ok(0)),
        };
        assert!(run(&default_config(), &cmd, &ok_ui(&Log::default())).is_err());
    }

    #[test]
    fn run_stage_title_error_propagates() {
        let ui = title_fails_on("Git Changes");
        assert!(run(&default_config(), &cmd_with_changes(), &ui).is_err());
    }

    #[test]
    fn run_stage_status_cmd_error_propagates() {
        let cmd = MockCmd {
            run: Box::new(|cmd, _| {
                if cmd.iter().any(|a| a.contains("--exit-code")) {
                    Ok(1)
                } else if cmd.iter().any(|a| a.contains("--porcelain")) {
                    Err(Error::GitPullFailed)
                } else {
                    Ok(0)
                }
            }),
            pipeline: Box::new(|_, _| Ok(0)),
        };
        assert!(run(&default_config(), &cmd, &ok_ui(&Log::default())).is_err());
    }

    #[test]
    fn run_stage_confirm_error_propagates() {
        let ui = MockUi {
            confirm: Box::new(|_, msg| {
                if msg.is_some() {
                    Err(Error::GitPullFailed)
                } else {
                    Ok(true)
                }
            }),
            ..ok_ui(&Log::default())
        };
        assert!(run(&default_config(), &cmd_with_changes(), &ui).is_err());
    }

    #[test]
    fn run_stage_decline_output_error_propagates() {
        let ui = MockUi {
            confirm: Box::new(|_, msg| if msg.is_some() { Ok(false) } else { Ok(true) }),
            error: Box::new(|_| Err(Error::GitPullFailed)),
            ..ok_ui(&Log::default())
        };
        assert!(run(&default_config(), &cmd_with_changes(), &ui).is_err());
    }

    #[test]
    fn run_stage_add_success_output_error_propagates() {
        let ui = MockUi {
            success: Box::new(|_| Err(Error::GitPullFailed)),
            ..ok_ui(&Log::default())
        };
        assert!(run(&default_config(), &cmd_with_changes(), &ui).is_err());
    }

    #[test]
    fn run_stage_add_failure_output_error_propagates() {
        let ui = MockUi {
            message: Box::new(|_| Err(Error::GitPullFailed)),
            ..ok_ui(&Log::default())
        };
        let cmd = MockCmd {
            run: Box::new(|cmd, _| {
                if cmd.iter().any(|a| a.contains("add")) {
                    Err(Error::Io(std::io::Error::other("mock")))
                } else if cmd.iter().any(|a| a.contains("--exit-code")) {
                    Ok(1)
                } else {
                    Ok(0)
                }
            }),
            pipeline: Box::new(|_, _| Ok(0)),
        };
        assert!(run(&default_config(), &cmd, &ui).is_err());
    }

    #[test]
    fn run_rebuild_title_error_propagates() {
        let ui = title_fails_on("Nixos Rebuild");
        assert!(run(&default_config(), &ok_cmd(), &ui).is_err());
    }

    #[test]
    fn run_rebuild_cmd_error_propagates() {
        assert!(
            run(
                &default_config(),
                &cmd_fails_on("nixos-rebuild"),
                &ok_ui(&Log::default())
            )
            .is_err()
        );
    }

    #[test]
    fn run_cleanup_cmd_error_propagates() {
        assert!(
            run(
                &default_config(),
                &cmd_fails_on("nix-env"),
                &ok_ui(&Log::default())
            )
            .is_err()
        );
    }

    #[test]
    fn run_diff_title_error_propagates() {
        let config = Config {
            diff: true,
            ..default_config()
        };
        let ui = title_fails_on("Nix Diff");
        assert!(run(&config, &ok_cmd(), &ui).is_err());
    }

    #[test]
    fn run_diff_pipeline_error_propagates() {
        let config = Config {
            diff: true,
            ..default_config()
        };
        let cmd = MockCmd {
            run: Box::new(|_, _| Ok(0)),
            pipeline: Box::new(|_, _| Err(Error::GitPullFailed)),
        };
        assert!(run(&config, &cmd, &ok_ui(&Log::default())).is_err());
    }
}
