use super::*;
use std::cell::RefCell;
use std::rc::Rc;

type RunFn = Box<dyn Fn(&[String]) -> Result<i32, Error>>;
type CaptureFn = Box<dyn Fn(&[String]) -> Result<(i32, String), Error>>;
type TitleFn = Box<dyn Fn(&str) -> Result<(), Error>>;
type ConfirmFn = Box<dyn Fn(bool, Option<&str>) -> Result<bool, Error>>;
type MsgFn = Box<dyn Fn(&str) -> Result<(), Error>>;
type ConfigFn = Box<dyn Fn(&Config) -> Result<(), Error>>;

struct MockCmd {
    run: RunFn,
    capture: CaptureFn,
}

impl CommandRunner for MockCmd {
    fn run(&self, cmd: &[String]) -> Result<i32, Error> {
        (self.run)(cmd)
    }
    fn capture(&self, cmd: &[String]) -> Result<(i32, String), Error> {
        (self.capture)(cmd)
    }
}

struct MockUi {
    title: TitleFn,
    confirm: ConfirmFn,
    error: MsgFn,
    success: MsgFn,
    output: MsgFn,
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
    fn output(&self, text: &str) -> Result<(), Error> {
        (self.output)(text)
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
        yes: false,
        dry_run: false,
        elevate: false,
    }
}

#[expect(
    clippy::unnecessary_wraps,
    reason = "mock must match the CommandRunner trait signature"
)]
fn no_run(_: &[String]) -> Result<i32, Error> {
    Ok(0)
}

#[expect(
    clippy::unnecessary_wraps,
    reason = "mock must match the CommandRunner trait signature"
)]
fn no_capture(_: &[String]) -> Result<(i32, String), Error> {
    Ok((0, String::new()))
}

fn ok_cmd() -> MockCmd {
    MockCmd {
        run: Box::new(no_run),
        capture: Box::new(no_capture),
    }
}

/// Command mock reporting uncommitted changes (non-empty `git status`).
fn changes_cmd() -> MockCmd {
    MockCmd {
        run: Box::new(|_| Ok(0)),
        capture: Box::new(|_| Ok((0, String::from(" M file\n")))),
    }
}

/// Command mock reporting changes but whose `run` fails on `sub`.
fn changes_run_fails_on(sub: &str) -> MockCmd {
    let sub = sub.to_owned();
    MockCmd {
        run: Box::new(move |cmd| {
            if cmd.iter().any(|a| a.contains(&sub)) {
                Err(Error::GitPullFailed)
            } else {
                Ok(0)
            }
        }),
        capture: Box::new(|_| Ok((0, String::from(" M file\n")))),
    }
}

/// Command mock whose `run` returns exit code 1 on `sub`.
fn run_nonzero_on(sub: &str) -> MockCmd {
    let sub = sub.to_owned();
    MockCmd {
        run: Box::new(move |cmd| {
            if cmd.iter().any(|a| a.contains(&sub)) {
                Ok(1)
            } else {
                Ok(0)
            }
        }),
        capture: Box::new(no_capture),
    }
}

/// Changes present and `run` returning exit code 1 on `sub`.
fn changes_run_nonzero_on(sub: &str) -> MockCmd {
    let sub = sub.to_owned();
    MockCmd {
        run: Box::new(move |cmd| {
            if cmd.iter().any(|a| a.contains(&sub)) {
                Ok(1)
            } else {
                Ok(0)
            }
        }),
        capture: Box::new(|_| Ok((0, String::from(" M file\n")))),
    }
}

/// Command mock whose `run` fails on `sub`.
fn run_fails_on(sub: &str) -> MockCmd {
    let sub = sub.to_owned();
    MockCmd {
        run: Box::new(move |cmd| {
            if cmd.iter().any(|a| a.contains(&sub)) {
                Err(Error::GitPullFailed)
            } else {
                Ok(0)
            }
        }),
        capture: Box::new(no_capture),
    }
}

/// Command mock whose `capture` fails on `sub`.
fn capture_fails_on(sub: &str) -> MockCmd {
    let sub = sub.to_owned();
    MockCmd {
        run: Box::new(no_run),
        capture: Box::new(move |cmd| {
            if cmd.iter().any(|a| a.contains(&sub)) {
                Err(Error::GitPullFailed)
            } else {
                Ok((0, String::new()))
            }
        }),
    }
}

/// Command mock whose `capture` returns exit code 1 on `sub`.
fn capture_nonzero_on(sub: &str) -> MockCmd {
    let sub = sub.to_owned();
    MockCmd {
        run: Box::new(no_run),
        capture: Box::new(move |cmd| {
            if cmd.iter().any(|a| a.contains(&sub)) {
                Ok((1, String::new()))
            } else {
                Ok((0, String::new()))
            }
        }),
    }
}

/// UI where every method succeeds; confirm/error/success/output are logged.
fn ok_ui(log: &Log) -> MockUi {
    let confirm_log = log.clone();
    let error_log = log.clone();
    let success_log = log.clone();
    let output_log = log.clone();
    MockUi {
        title: Box::new(|_| Ok(())),
        confirm: Box::new(move |_, _| {
            confirm_log.record("confirm");
            Ok(true)
        }),
        error: Box::new(move |m| {
            error_log.record(m);
            Ok(())
        }),
        success: Box::new(move |m| {
            success_log.record(m);
            Ok(())
        }),
        output: Box::new(move |m| {
            output_log.record(m);
            Ok(())
        }),
        config: Box::new(|_| Ok(())),
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
fn run_yes_true_skips_top_confirmation() {
    let log = Log::default();
    let ui = ok_ui(&log);
    let config = Config {
        yes: true,
        ..default_config()
    };
    assert!(run(&config, &ok_cmd(), &ui).is_ok());
    assert!(!log.contains("confirm"));
}

#[test]
fn run_yes_true_skips_stage_confirmation() {
    let log = Log::default();
    let ui = ok_ui(&log);
    let config = Config {
        yes: true,
        ..default_config()
    };
    assert!(run(&config, &changes_cmd(), &ui).is_ok());
    assert!(log.contains("Changes added to git stage successfully"));
    assert!(!log.contains("confirm"));
}

#[test]
fn run_no_git_changes_skips_stage() {
    let log = Log::default();
    let ui = ok_ui(&log);
    assert!(run(&default_config(), &ok_cmd(), &ui).is_ok());
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
    assert!(run(&default_config(), &changes_cmd(), &ui).is_ok());
    assert!(log.contains("Changes added to git stage successfully"));
}

#[test]
fn run_stage_decline_prints_not_added() {
    let log = Log::default();
    let ui = MockUi {
        confirm: Box::new(|_, msg| if msg.is_some() { Ok(false) } else { Ok(true) }),
        ..ok_ui(&log)
    };
    assert!(run(&default_config(), &changes_cmd(), &ui).is_ok());
    assert!(log.contains("Changes not added to stage"));
}

#[test]
fn run_stage_add_failure_prints_failure_message() {
    let log = Log::default();
    let ui = ok_ui(&log);
    let cmd = changes_run_fails_on("add");
    assert!(run(&default_config(), &cmd, &ui).is_err());
    assert!(log.contains("Failed to add changes to the stage"));
}

#[test]
fn run_stage_add_nonzero_returns_error() {
    let log = Log::default();
    let ui = ok_ui(&log);
    assert!(run(&default_config(), &changes_run_nonzero_on("add"), &ui).is_err());
    assert!(log.contains("Failed to add changes to the stage"));
}

#[test]
fn run_stage_add_nonzero_output_error_propagates() {
    let ui = MockUi {
        error: Box::new(|_| Err(Error::GitPullFailed)),
        ..ok_ui(&Log::default())
    };
    assert!(run(&default_config(), &changes_run_nonzero_on("add"), &ui).is_err());
}

// ------------------------------------------------------------------
// git pull
// ------------------------------------------------------------------

#[test]
fn run_git_pull_non_zero_returns_error() {
    let cmd = MockCmd {
        run: Box::new(|_| Ok(1)),
        capture: Box::new(no_capture),
    };
    assert!(run(&default_config(), &cmd, &ok_ui(&Log::default())).is_err());
}

#[test]
fn run_not_a_git_repo_returns_error() {
    assert!(
        run(
            &default_config(),
            &capture_nonzero_on("rev-parse"),
            &ok_ui(&Log::default())
        )
        .is_err()
    );
}

#[test]
fn run_pull_rev_parse_capture_error_propagates() {
    assert!(
        run(
            &default_config(),
            &capture_fails_on("rev-parse"),
            &ok_ui(&Log::default())
        )
        .is_err()
    );
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
            &run_fails_on("pull"),
            &ok_ui(&Log::default())
        )
        .is_err()
    );
}

#[test]
fn run_pull_error_output_error_propagates() {
    let cmd = MockCmd {
        run: Box::new(|_| Ok(1)),
        capture: Box::new(no_capture),
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
    assert!(run(&config, &run_fails_on("flake"), &ok_ui(&Log::default())).is_err());
}

#[test]
fn run_flake_nonzero_returns_error() {
    let config = Config {
        update: true,
        ..default_config()
    };
    assert!(run(&config, &run_nonzero_on("flake"), &ok_ui(&Log::default())).is_err());
}

#[test]
fn run_stage_capture_error_propagates() {
    assert!(
        run(
            &default_config(),
            &capture_fails_on("--porcelain"),
            &ok_ui(&Log::default())
        )
        .is_err()
    );
}

#[test]
fn run_stage_git_status_nonzero_returns_error() {
    assert!(
        run(
            &default_config(),
            &capture_nonzero_on("--porcelain"),
            &ok_ui(&Log::default())
        )
        .is_err()
    );
}

#[test]
fn run_stage_title_error_propagates() {
    let ui = title_fails_on("Git Changes");
    assert!(run(&default_config(), &changes_cmd(), &ui).is_err());
}

#[test]
fn run_stage_output_error_propagates() {
    let ui = MockUi {
        output: Box::new(|_| Err(Error::GitPullFailed)),
        ..ok_ui(&Log::default())
    };
    assert!(run(&default_config(), &changes_cmd(), &ui).is_err());
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
    assert!(run(&default_config(), &changes_cmd(), &ui).is_err());
}

#[test]
fn run_stage_decline_output_error_propagates() {
    let ui = MockUi {
        confirm: Box::new(|_, msg| if msg.is_some() { Ok(false) } else { Ok(true) }),
        error: Box::new(|_| Err(Error::GitPullFailed)),
        ..ok_ui(&Log::default())
    };
    assert!(run(&default_config(), &changes_cmd(), &ui).is_err());
}

#[test]
fn run_stage_add_success_output_error_propagates() {
    let ui = MockUi {
        success: Box::new(|_| Err(Error::GitPullFailed)),
        ..ok_ui(&Log::default())
    };
    assert!(run(&default_config(), &changes_cmd(), &ui).is_err());
}

#[test]
fn run_stage_add_failure_output_error_propagates() {
    let ui = MockUi {
        error: Box::new(|_| Err(Error::GitPullFailed)),
        ..ok_ui(&Log::default())
    };
    assert!(run(&default_config(), &changes_run_fails_on("add"), &ui).is_err());
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
            &run_fails_on("nixos-rebuild"),
            &ok_ui(&Log::default())
        )
        .is_err()
    );
}

#[test]
fn run_rebuild_nonzero_returns_error() {
    assert!(
        run(
            &default_config(),
            &run_nonzero_on("nixos-rebuild"),
            &ok_ui(&Log::default())
        )
        .is_err()
    );
}

#[test]
fn run_rebuild_hostname_unknown_returns_error() {
    let config = Config {
        hostname: String::from("unknown"),
        ..default_config()
    };
    assert!(run(&config, &ok_cmd(), &ok_ui(&Log::default())).is_err());
}

#[test]
fn run_cleanup_cmd_error_propagates() {
    assert!(
        run(
            &default_config(),
            &run_fails_on("nix-env"),
            &ok_ui(&Log::default())
        )
        .is_err()
    );
}

#[test]
fn run_cleanup_gc_cmd_error_propagates() {
    assert!(
        run(
            &default_config(),
            &run_fails_on("nix-collect-garbage"),
            &ok_ui(&Log::default())
        )
        .is_err()
    );
}

#[test]
fn run_cleanup_keep_nonzero_returns_error() {
    assert!(
        run(
            &default_config(),
            &run_nonzero_on("nix-env"),
            &ok_ui(&Log::default())
        )
        .is_err()
    );
}

#[test]
fn run_cleanup_gc_nonzero_returns_error() {
    assert!(
        run(
            &default_config(),
            &run_nonzero_on("nix-collect-garbage"),
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
fn run_diff_capture_error_propagates() {
    let config = Config {
        diff: true,
        ..default_config()
    };
    assert!(
        run(
            &config,
            &capture_fails_on("diff-closures"),
            &ok_ui(&Log::default())
        )
        .is_err()
    );
}

#[test]
fn run_diff_output_error_propagates() {
    let config = Config {
        diff: true,
        ..default_config()
    };
    let ui = MockUi {
        output: Box::new(|_| Err(Error::GitPullFailed)),
        ..ok_ui(&Log::default())
    };
    assert!(run(&config, &ok_cmd(), &ui).is_err());
}
