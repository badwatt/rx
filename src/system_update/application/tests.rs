use std::cell::RefCell;
use std::rc::Rc;

use super::*;
use crate::domain::config::{BuildOptions, Pipeline, Step};
// ------------------------------------------------------------------
// Named defaults (shared, so their branches merge across tests)
// ------------------------------------------------------------------

#[expect(
    clippy::unnecessary_wraps,
    reason = "mock must match the port trait signature"
)]
fn yes_is_repo(_: &str) -> Result<bool, Error> {
    Ok(true)
}

#[expect(
    clippy::unnecessary_wraps,
    reason = "mock must match the port trait signature"
)]
fn ok_str(_: &str) -> Result<(), Error> {
    Ok(())
}

#[expect(
    clippy::unnecessary_wraps,
    reason = "mock must match the port trait signature"
)]
fn no_changes(_: &str) -> Result<String, Error> {
    Ok(String::new())
}

#[expect(
    clippy::unnecessary_wraps,
    reason = "mock must match the port trait signature"
)]
fn ok_rebuild(_: &str, _: &str, _: BuildOptions) -> Result<(), Error> {
    Ok(())
}

#[expect(
    clippy::unnecessary_wraps,
    reason = "mock must match the port trait signature"
)]
fn ok_prune(_: u8) -> Result<(), Error> {
    Ok(())
}

#[expect(
    clippy::unnecessary_wraps,
    reason = "mock must match the port trait signature"
)]
fn ok_gc() -> Result<(), Error> {
    Ok(())
}

#[expect(
    clippy::unnecessary_wraps,
    reason = "mock must match the port trait signature"
)]
fn no_diff() -> Result<String, Error> {
    Ok(String::new())
}

#[expect(
    clippy::unnecessary_wraps,
    reason = "mock must match the port trait signature"
)]
fn no_password_pending() -> Result<bool, Error> {
    Ok(false)
}

fn pull_errs(_: &str) -> Result<(), Error> {
    Err(Error::GitPullFailed)
}

fn rebuild_errs(_: &str, _: &str, _: BuildOptions) -> Result<(), Error> {
    Err(Error::RebuildFailed)
}

// ------------------------------------------------------------------
// Mock ports
// ------------------------------------------------------------------

type IsRepoFn = Box<dyn Fn(&str) -> Result<bool, Error>>;
type StrFn = Box<dyn Fn(&str) -> Result<(), Error>>;
type ChangesFn = Box<dyn Fn(&str) -> Result<String, Error>>;

struct MockRepo {
    is_repo: IsRepoFn,
    pull: StrFn,
    changes: ChangesFn,
    add_all: StrFn,
}

impl Repo for MockRepo {
    fn is_repo(&self, repo: &str) -> Result<bool, Error> {
        (self.is_repo)(repo)
    }
    fn pull(&self, repo: &str) -> Result<(), Error> {
        (self.pull)(repo)
    }
    fn changes(&self, repo: &str) -> Result<String, Error> {
        (self.changes)(repo)
    }
    fn add_all(&self, repo: &str) -> Result<(), Error> {
        (self.add_all)(repo)
    }
}

impl Default for MockRepo {
    fn default() -> Self {
        Self {
            is_repo: Box::new(yes_is_repo),
            pull: Box::new(ok_str),
            changes: Box::new(no_changes),
            add_all: Box::new(ok_str),
        }
    }
}

type RebuildFn = Box<dyn Fn(&str, &str, BuildOptions) -> Result<(), Error>>;
type PendingFn = Box<dyn Fn() -> Result<bool, Error>>;
type RefreshFn = Box<dyn Fn() -> Result<(), Error>>;
type PruneFn = Box<dyn Fn(u8) -> Result<(), Error>>;
type GcFn = Box<dyn Fn() -> Result<(), Error>>;
type DiffFn = Box<dyn Fn() -> Result<String, Error>>;

struct MockNix {
    update: StrFn,
    rebuild: RebuildFn,
    password_pending: PendingFn,
    refresh_credentials: RefreshFn,
    prune: PruneFn,
    gc: GcFn,
    diff: DiffFn,
}

impl Nix for MockNix {
    fn update(&self, repo: &str) -> Result<(), Error> {
        (self.update)(repo)
    }
    fn rebuild(&self, repo: &str, hostname: &str, options: BuildOptions) -> Result<(), Error> {
        (self.rebuild)(repo, hostname, options)
    }
    fn password_pending(&self) -> Result<bool, Error> {
        (self.password_pending)()
    }
    fn refresh_credentials(&self) -> Result<(), Error> {
        (self.refresh_credentials)()
    }
    fn prune(&self, keep: u8) -> Result<(), Error> {
        (self.prune)(keep)
    }
    fn collect_garbage(&self) -> Result<(), Error> {
        (self.gc)()
    }
    fn diff_closures(&self) -> Result<String, Error> {
        (self.diff)()
    }
}

impl Default for MockNix {
    fn default() -> Self {
        Self {
            update: Box::new(ok_str),
            rebuild: Box::new(ok_rebuild),
            password_pending: Box::new(no_password_pending),
            refresh_credentials: Box::new(ok_gc),
            prune: Box::new(ok_prune),
            gc: Box::new(ok_gc),
            diff: Box::new(no_diff),
        }
    }
}

// ------------------------------------------------------------------
// Mock UI
// ------------------------------------------------------------------

type TitleFn = Box<dyn Fn(&str) -> Result<(), Error>>;
type ProgressFn = Box<dyn Fn(State, &str) -> Result<(), Error>>;
type ConfirmFn = Box<dyn Fn(bool, Option<&str>) -> Result<bool, Error>>;
type MsgFn = Box<dyn Fn(&str) -> Result<(), Error>>;
type ConfigFn = Box<dyn Fn(&Config) -> Result<(), Error>>;

struct MockUi {
    title: TitleFn,
    progress: ProgressFn,
    confirm: ConfirmFn,
    success: MsgFn,
    output: MsgFn,
    config: ConfigFn,
}

impl UserIo for MockUi {
    fn title(&self, text: &str) -> Result<(), Error> {
        (self.title)(text)
    }
    fn progress(&self, state: State, msg: &str) -> Result<(), Error> {
        (self.progress)(state, msg)
    }
    fn confirm(&self, default: bool, msg: Option<&str>) -> Result<bool, Error> {
        (self.confirm)(default, msg)
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
    fn position(&self, needle: &str) -> Option<usize> {
        self.0.borrow().iter().position(|m| m.contains(needle))
    }
    fn last_position(&self, needle: &str) -> Option<usize> {
        self.0.borrow().iter().rposition(|m| m.contains(needle))
    }
}

fn default_config() -> Config {
    Config {
        pipeline: Pipeline::Default {
            update: false,
            diff: false,
        },
        repo: String::from("r"),
        hostname: Some(String::from("h")),
        keep: 1,
        yes: false,
        dry_run: false,
        jobs: None,
        cores: None,
    }
}

/// UI where every method succeeds; confirm/error/success/output are logged.
fn ok_ui(log: &Log) -> MockUi {
    let confirm_log = log.clone();
    let title_log = log.clone();
    let progress_log = log.clone();
    let success_log = log.clone();
    let output_log = log.clone();
    MockUi {
        title: Box::new(move |text| {
            title_log.record(text);
            Ok(())
        }),
        progress: Box::new(move |state, msg| {
            progress_log.record(&format!("{state:?} {msg}"));
            Ok(())
        }),
        confirm: Box::new(move |_, _| {
            confirm_log.record("confirm");
            Ok(true)
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

fn repo_with_changes() -> MockRepo {
    MockRepo {
        changes: Box::new(|_| Ok(String::from(" M file\n"))),
        ..MockRepo::default()
    }
}

/// Run with the default (all-ok) ports.
fn run_ok(config: &Config, ui: &dyn UserIo) -> Result<(), Error> {
    run(config, &MockRepo::default(), &MockNix::default(), ui)
}

// ------------------------------------------------------------------
// happy paths
// ------------------------------------------------------------------

#[test]
fn run_confirm_false_returns_early_ok() {
    let log = Log::default();
    let ui = MockUi {
        confirm: Box::new(|_, _| Ok(false)),
        ..ok_ui(&log)
    };
    assert!(run_ok(&default_config(), &ui).is_ok());
    assert!(log.contains("Aborted."));
}

#[test]
fn run_reports_idle_when_the_prompt_is_declined() {
    let log = Log::default();
    let ui = MockUi {
        confirm: Box::new(|_, _| Ok(false)),
        ..ok_ui(&log)
    };
    assert!(run_ok(&default_config(), &ui).is_ok());
    assert!(log.contains("Idle"));
    assert!(!log.contains("Done"));
}

#[test]
fn run_reports_idle_when_staging_is_declined() {
    let log = Log::default();
    let ui = MockUi {
        confirm: Box::new(|_, msg| if msg.is_some() { Ok(false) } else { Ok(true) }),
        ..ok_ui(&log)
    };
    assert!(run(&default_config(), &repo_with_changes(), &MockNix::default(), &ui).is_ok());
    assert!(log.contains("Idle"));
}

#[test]
fn run_invalid_config_claims_no_state() {
    let log = Log::default();
    let config = Config {
        repo: String::new(),
        ..default_config()
    };
    assert!(run_ok(&config, &ok_ui(&log)).is_err());
    assert!(!log.contains("Working"));
    assert!(!log.contains("Failed"));
}

#[test]
fn run_yes_true_skips_top_confirmation() {
    let log = Log::default();
    let config = Config {
        yes: true,
        ..default_config()
    };
    assert!(run_ok(&config, &ok_ui(&log)).is_ok());
    assert!(!log.contains("confirm"));
}

#[test]
fn run_yes_true_skips_stage_confirmation() {
    let log = Log::default();
    let config = Config {
        yes: true,
        ..default_config()
    };
    let repo = repo_with_changes();
    assert!(run(&config, &repo, &MockNix::default(), &ok_ui(&log)).is_ok());
    assert!(log.contains("Changes added to git stage successfully"));
    assert!(!log.contains("confirm"));
}

#[test]
fn run_no_git_changes_skips_stage() {
    let log = Log::default();
    assert!(run_ok(&default_config(), &ok_ui(&log)).is_ok());
    assert!(!log.contains("Changes added"));
}

#[test]
fn run_update_and_diff_true_runs_extra_commands() {
    let config = Config {
        pipeline: Pipeline::Default {
            update: true,
            diff: true,
        },
        ..default_config()
    };
    assert!(run_ok(&config, &ok_ui(&Log::default())).is_ok());
}

#[test]
fn run_stage_add_success_prints_success() {
    let log = Log::default();
    let repo = repo_with_changes();
    assert!(run(&default_config(), &repo, &MockNix::default(), &ok_ui(&log)).is_ok());
    assert!(log.contains("Changes added to git stage successfully"));
}

#[test]
fn run_stage_decline_aborts_before_rebuild() {
    let log = Log::default();
    let ui = MockUi {
        confirm: Box::new(|_, msg| if msg.is_some() { Ok(false) } else { Ok(true) }),
        ..ok_ui(&log)
    };
    let repo = repo_with_changes();
    // If the rebuild ran, this mock would fail the whole workflow.
    let nix = MockNix {
        rebuild: Box::new(rebuild_errs),
        ..MockNix::default()
    };
    assert!(run(&default_config(), &repo, &nix, &ui).is_ok());
    assert!(log.contains("Changes not added to stage"));
}

#[test]
fn run_stage_add_failure_returns_git_add_failed() {
    let repo = MockRepo {
        changes: Box::new(|_| Ok(String::from(" M file\n"))),
        add_all: Box::new(|_| Err(Error::GitAddFailed)),
        ..MockRepo::default()
    };
    let err = run(
        &default_config(),
        &repo,
        &MockNix::default(),
        &ok_ui(&Log::default()),
    )
    .expect_err("add failure");
    assert_eq!(err.to_string(), "Failed to add changes to the stage");
}

#[test]
fn run_stage_add_spawn_failure_propagates_the_io_error() {
    let repo = MockRepo {
        changes: Box::new(|_| Ok(String::from(" M file\n"))),
        add_all: Box::new(|_| Err(Error::Io(std::io::Error::other("boom")))),
        ..MockRepo::default()
    };
    let err = run(
        &default_config(),
        &repo,
        &MockNix::default(),
        &ok_ui(&Log::default()),
    )
    .expect_err("add spawn failure");
    assert!(err.to_string().contains("IO error"));
}

// ------------------------------------------------------------------
// pull / repo errors
// ------------------------------------------------------------------

#[test]
fn run_git_pull_non_zero_returns_git_pull_failed() {
    let repo = MockRepo {
        pull: Box::new(pull_errs),
        ..MockRepo::default()
    };
    let err = run(
        &default_config(),
        &repo,
        &MockNix::default(),
        &ok_ui(&Log::default()),
    )
    .expect_err("pull failure");
    assert_eq!(err.to_string(), "Failed to pull changes");
}

#[test]
fn run_not_a_git_repo_returns_error() {
    let repo = MockRepo {
        is_repo: Box::new(|_| Ok(false)),
        ..MockRepo::default()
    };
    assert!(
        run(
            &default_config(),
            &repo,
            &MockNix::default(),
            &ok_ui(&Log::default())
        )
        .is_err()
    );
}

#[test]
fn run_is_repo_error_propagates() {
    let repo = MockRepo {
        is_repo: Box::new(|_| Err(Error::NotAGitRepo)),
        ..MockRepo::default()
    };
    assert!(
        run(
            &default_config(),
            &repo,
            &MockNix::default(),
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
    assert!(run_ok(&default_config(), &ui).is_err());
}

#[test]
fn run_config_error_propagates() {
    let ui = MockUi {
        config: Box::new(|_| Err(Error::GitPullFailed)),
        ..ok_ui(&Log::default())
    };
    assert!(run_ok(&default_config(), &ui).is_err());
}

#[test]
fn run_confirm_error_propagates() {
    let ui = MockUi {
        confirm: Box::new(|_, _| Err(Error::GitPullFailed)),
        ..ok_ui(&Log::default())
    };
    assert!(run_ok(&default_config(), &ui).is_err());
}

#[test]
fn run_invalid_config_returns_error() {
    let config = Config {
        repo: String::new(),
        ..default_config()
    };
    assert!(run_ok(&config, &ok_ui(&Log::default())).is_err());
}

// ------------------------------------------------------------------
// OSC 7501 state reports
// ------------------------------------------------------------------

#[test]
fn run_reports_working_rebuilding_and_done() {
    let log = Log::default();
    assert!(run_ok(&default_config(), &ok_ui(&log)).is_ok());
    assert!(log.contains("Working"));
    assert!(log.contains("Rebuilding"));
    assert!(log.contains("Done"));
}

#[test]
fn run_reports_blocked_permission_before_the_prompt() {
    let log = Log::default();
    assert!(run_ok(&default_config(), &ok_ui(&log)).is_ok());
    assert!(log.contains("BlockedPermission"));
}

#[test]
fn run_yes_skips_blocked_permission() {
    let log = Log::default();
    let config = Config {
        yes: true,
        ..default_config()
    };
    assert!(run_ok(&config, &ok_ui(&log)).is_ok());
    assert!(!log.contains("BlockedPermission"));
}

#[test]
fn run_reports_failed_when_a_step_fails() {
    let log = Log::default();
    let nix = MockNix {
        rebuild: Box::new(rebuild_errs),
        ..MockNix::default()
    };
    assert!(run(&default_config(), &MockRepo::default(), &nix, &ok_ui(&log)).is_err());
    assert!(log.contains("Failed"));
}

#[test]
fn run_start_progress_error_propagates() {
    let ui = MockUi {
        progress: Box::new(|_, _| Err(Error::GitPullFailed)),
        ..ok_ui(&Log::default())
    };
    assert!(run_ok(&default_config(), &ui).is_err());
}

#[test]
fn run_done_progress_error_propagates() {
    let ui = MockUi {
        progress: Box::new(|state, _| {
            if state == State::Done {
                Err(Error::GitPullFailed)
            } else {
                Ok(())
            }
        }),
        ..ok_ui(&Log::default())
    };
    assert!(run_ok(&default_config(), &ui).is_err());
}

#[test]
fn run_blocked_progress_error_propagates() {
    let ui = MockUi {
        progress: Box::new(|state, _| {
            if state == State::BlockedPermission {
                Err(Error::GitPullFailed)
            } else {
                Ok(())
            }
        }),
        ..ok_ui(&Log::default())
    };
    assert!(run_ok(&default_config(), &ui).is_err());
}

#[test]
fn run_rebuild_progress_error_propagates() {
    let ui = MockUi {
        progress: Box::new(|_, msg| {
            if msg.contains("Rebuilding") {
                Err(Error::RebuildFailed)
            } else {
                Ok(())
            }
        }),
        ..ok_ui(&Log::default())
    };
    assert!(run_ok(&default_config(), &ui).is_err());
}

#[test]
fn run_failed_progress_error_keeps_the_step_error() {
    let nix = MockNix {
        rebuild: Box::new(rebuild_errs),
        ..MockNix::default()
    };
    let ui = MockUi {
        progress: Box::new(|state, _| {
            if state == State::Failed {
                Err(Error::GitPullFailed)
            } else {
                Ok(())
            }
        }),
        ..ok_ui(&Log::default())
    };
    let err = run(&default_config(), &MockRepo::default(), &nix, &ui)
        .expect_err("rebuild failure must win");
    assert_eq!(err.to_string(), "NixOS rebuild failed");
}

#[test]
fn run_reports_blocked_auth_when_sudo_will_prompt() {
    let log = Log::default();
    let nix = MockNix {
        password_pending: Box::new(|| Ok(true)),
        ..MockNix::default()
    };
    assert!(run(&default_config(), &MockRepo::default(), &nix, &ok_ui(&log)).is_ok());
    assert!(log.contains("BlockedAuth"));
}

#[test]
fn run_skips_blocked_auth_when_credentials_are_cached() {
    let log = Log::default();
    assert!(run_ok(&default_config(), &ok_ui(&log)).is_ok());
    assert!(!log.contains("BlockedAuth"));
}

#[test]
fn run_clears_blocked_permission_after_the_prompt() {
    let log = Log::default();
    assert!(run_ok(&default_config(), &ok_ui(&log)).is_ok());
    let blocked = log.position("BlockedPermission").expect("blocked report");
    let cleared = log.last_position("Working").expect("working report");
    assert!(blocked < cleared);
}

#[test]
fn run_clears_blocked_auth_after_the_password() {
    let log = Log::default();
    let nix = MockNix {
        password_pending: Box::new(|| Ok(true)),
        ..MockNix::default()
    };
    assert!(run(&default_config(), &MockRepo::default(), &nix, &ok_ui(&log)).is_ok());
    let blocked = log.position("BlockedAuth").expect("blocked report");
    let cleared = log.last_position("Working").expect("working report");
    assert!(blocked < cleared);
}

#[test]
fn run_post_prompt_progress_error_propagates() {
    let calls = Rc::new(std::cell::Cell::new(0usize));
    let counter = Rc::clone(&calls);
    let ui = MockUi {
        progress: Box::new(move |_, _| {
            let n = counter.get() + 1;
            counter.set(n);
            if n == 3 {
                Err(Error::GitPullFailed)
            } else {
                Ok(())
            }
        }),
        ..ok_ui(&Log::default())
    };
    assert!(run_ok(&default_config(), &ui).is_err());
}

#[test]
fn run_stage_blocked_progress_error_propagates() {
    let repo = repo_with_changes();
    let ui = MockUi {
        progress: Box::new(|_, msg| {
            if msg.contains("staging") {
                Err(Error::GitPullFailed)
            } else {
                Ok(())
            }
        }),
        ..ok_ui(&Log::default())
    };
    assert!(run(&default_config(), &repo, &MockNix::default(), &ui).is_err());
}

#[test]
fn run_stage_working_progress_error_propagates() {
    let repo = repo_with_changes();
    let ui = MockUi {
        progress: Box::new(|_, msg| {
            if msg.contains("Staging") {
                Err(Error::GitPullFailed)
            } else {
                Ok(())
            }
        }),
        ..ok_ui(&Log::default())
    };
    assert!(run(&default_config(), &repo, &MockNix::default(), &ui).is_err());
}

#[test]
fn run_credentials_error_propagates() {
    let nix = MockNix {
        password_pending: Box::new(|| Ok(true)),
        refresh_credentials: Box::new(|| Err(Error::PasswordFailed)),
        ..MockNix::default()
    };
    let err = run(
        &default_config(),
        &MockRepo::default(),
        &nix,
        &ok_ui(&Log::default()),
    )
    .expect_err("refused password must abort");
    assert_eq!(err.to_string(), "Failed to obtain sudo credentials");
}

#[test]
fn run_dry_run_never_probes_or_refreshes_sudo() {
    let config = Config {
        dry_run: true,
        ..default_config()
    };
    let nix = MockNix {
        // Any probe or refresh would fail the run.
        password_pending: Box::new(|| Err(Error::PasswordFailed)),
        refresh_credentials: Box::new(|| Err(Error::PasswordFailed)),
        ..MockNix::default()
    };
    assert!(run(&config, &MockRepo::default(), &nix, &ok_ui(&Log::default())).is_ok());
}

#[test]
fn run_password_pending_error_propagates() {
    let nix = MockNix {
        password_pending: Box::new(|| Err(Error::RebuildFailed)),
        ..MockNix::default()
    };
    assert!(
        run(
            &default_config(),
            &MockRepo::default(),
            &nix,
            &ok_ui(&Log::default())
        )
        .is_err()
    );
}

#[test]
fn run_blocked_auth_progress_error_propagates() {
    let nix = MockNix {
        password_pending: Box::new(|| Ok(true)),
        ..MockNix::default()
    };
    let ui = MockUi {
        progress: Box::new(|state, _| {
            if state == State::BlockedAuth {
                Err(Error::GitPullFailed)
            } else {
                Ok(())
            }
        }),
        ..ok_ui(&Log::default())
    };
    assert!(run(&default_config(), &MockRepo::default(), &nix, &ui).is_err());
}

#[test]
fn run_password_clear_progress_error_propagates() {
    let nix = MockNix {
        password_pending: Box::new(|| Ok(true)),
        ..MockNix::default()
    };
    let ui = MockUi {
        progress: Box::new(|state, msg| {
            if state == State::Working && msg.contains("Rebuilding") {
                Err(Error::RebuildFailed)
            } else {
                Ok(())
            }
        }),
        ..ok_ui(&Log::default())
    };
    assert!(run(&default_config(), &MockRepo::default(), &nix, &ui).is_err());
}

#[test]
fn run_password_prompt_failure_keeps_the_step_error() {
    let nix = MockNix {
        password_pending: Box::new(|| Ok(true)),
        rebuild: Box::new(rebuild_errs),
        ..MockNix::default()
    };
    let err = run(
        &default_config(),
        &MockRepo::default(),
        &nix,
        &ok_ui(&Log::default()),
    )
    .expect_err("rebuild failure must win");
    assert_eq!(err.to_string(), "NixOS rebuild failed");
}

#[test]
fn run_stage_reports_blocked_permission_before_the_prompt() {
    let log = Log::default();
    let repo = repo_with_changes();
    assert!(run(&default_config(), &repo, &MockNix::default(), &ok_ui(&log)).is_ok());
    assert!(log.contains("BlockedPermission"));
}

// ------------------------------------------------------------------
// per-step error propagation
// ------------------------------------------------------------------

#[test]
fn run_pull_title_error_propagates() {
    assert!(run_ok(&default_config(), &title_fails_on("Git Pull")).is_err());
}

#[test]
fn run_flake_title_error_propagates() {
    let config = Config {
        pipeline: Pipeline::Default {
            update: true,
            diff: false,
        },
        ..default_config()
    };
    assert!(run_ok(&config, &title_fails_on("Nix Update")).is_err());
}

#[test]
fn run_flake_error_propagates() {
    let config = Config {
        pipeline: Pipeline::Default {
            update: true,
            diff: false,
        },
        ..default_config()
    };
    let nix = MockNix {
        update: Box::new(|_| Err(Error::FlakeUpdateFailed)),
        ..MockNix::default()
    };
    assert!(run(&config, &MockRepo::default(), &nix, &ok_ui(&Log::default())).is_err());
}

#[test]
fn run_stage_changes_error_propagates() {
    let repo = MockRepo {
        changes: Box::new(|_| Err(Error::GitStatusFailed)),
        ..MockRepo::default()
    };
    assert!(
        run(
            &default_config(),
            &repo,
            &MockNix::default(),
            &ok_ui(&Log::default())
        )
        .is_err()
    );
}

#[test]
fn run_stage_title_error_propagates() {
    let repo = repo_with_changes();
    assert!(
        run(
            &default_config(),
            &repo,
            &MockNix::default(),
            &title_fails_on("Git Changes")
        )
        .is_err()
    );
}

#[test]
fn run_stage_output_error_propagates() {
    let repo = repo_with_changes();
    let ui = MockUi {
        output: Box::new(|_| Err(Error::GitPullFailed)),
        ..ok_ui(&Log::default())
    };
    assert!(run(&default_config(), &repo, &MockNix::default(), &ui).is_err());
}

#[test]
fn run_stage_confirm_error_propagates() {
    let repo = repo_with_changes();
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
    assert!(run(&default_config(), &repo, &MockNix::default(), &ui).is_err());
}

#[test]
fn run_stage_decline_ignores_abort_message_write_error() {
    let calls = Rc::new(std::cell::Cell::new(0usize));
    let counter = Rc::clone(&calls);
    let ui = MockUi {
        confirm: Box::new(|_, msg| if msg.is_some() { Ok(false) } else { Ok(true) }),
        output: Box::new(move |_| {
            let n = counter.get() + 1;
            counter.set(n);
            if n == 1 {
                Ok(())
            } else {
                Err(Error::GitPullFailed)
            }
        }),
        ..ok_ui(&Log::default())
    };
    let repo = repo_with_changes();
    assert!(run(&default_config(), &repo, &MockNix::default(), &ui).is_ok());
}

#[test]
fn run_stage_add_success_output_error_propagates() {
    let repo = repo_with_changes();
    let ui = MockUi {
        success: Box::new(|_| Err(Error::GitPullFailed)),
        ..ok_ui(&Log::default())
    };
    assert!(run(&default_config(), &repo, &MockNix::default(), &ui).is_err());
}

#[test]
fn run_rebuild_title_error_propagates() {
    assert!(run_ok(&default_config(), &title_fails_on("NixOS Rebuild")).is_err());
}

#[test]
fn run_rebuild_error_propagates() {
    let nix = MockNix {
        rebuild: Box::new(rebuild_errs),
        ..MockNix::default()
    };
    assert!(
        run(
            &default_config(),
            &MockRepo::default(),
            &nix,
            &ok_ui(&Log::default())
        )
        .is_err()
    );
}

#[test]
fn run_only_diff_without_a_hostname_succeeds() {
    let config = Config {
        hostname: None,
        pipeline: Pipeline::Only(vec![Step::Diff]),
        ..default_config()
    };
    assert!(
        run(
            &config,
            &MockRepo::default(),
            &MockNix::default(),
            &ok_ui(&Log::default())
        )
        .is_ok()
    );
}

#[test]
fn run_only_cleanup_without_a_hostname_succeeds() {
    let config = Config {
        hostname: None,
        pipeline: Pipeline::Only(vec![Step::Pull, Step::Cleanup]),
        ..default_config()
    };
    assert!(
        run(
            &config,
            &MockRepo::default(),
            &MockNix::default(),
            &ok_ui(&Log::default())
        )
        .is_ok()
    );
}

#[test]
fn run_undetected_hostname_aborts_before_pull() {
    let config = Config {
        hostname: None,
        ..default_config()
    };
    let repo = MockRepo {
        pull: Box::new(pull_errs),
        ..MockRepo::default()
    };
    let err = run(&config, &repo, &MockNix::default(), &ok_ui(&Log::default()))
        .expect_err("unknown hostname must abort");
    assert_eq!(
        std::mem::discriminant(&err),
        std::mem::discriminant(&Error::HostnameUnknown)
    );
}

#[test]
fn run_cleanup_prune_error_propagates() {
    let nix = MockNix {
        prune: Box::new(|_| Err(Error::CleanupFailed)),
        ..MockNix::default()
    };
    assert!(
        run(
            &default_config(),
            &MockRepo::default(),
            &nix,
            &ok_ui(&Log::default())
        )
        .is_err()
    );
}

#[test]
fn run_cleanup_gc_error_propagates() {
    let nix = MockNix {
        gc: Box::new(|| Err(Error::CleanupFailed)),
        ..MockNix::default()
    };
    assert!(
        run(
            &default_config(),
            &MockRepo::default(),
            &nix,
            &ok_ui(&Log::default())
        )
        .is_err()
    );
}

#[test]
fn run_diff_title_error_propagates() {
    let config = Config {
        pipeline: Pipeline::Default {
            update: false,
            diff: true,
        },
        ..default_config()
    };
    assert!(run_ok(&config, &title_fails_on("Nix Diff")).is_err());
}

#[test]
fn run_diff_error_propagates() {
    let config = Config {
        pipeline: Pipeline::Default {
            update: false,
            diff: true,
        },
        ..default_config()
    };
    let nix = MockNix {
        diff: Box::new(|| Err(Error::Io(std::io::Error::other("x")))),
        ..MockNix::default()
    };
    assert!(run(&config, &MockRepo::default(), &nix, &ok_ui(&Log::default())).is_err());
}

#[test]
fn run_diff_output_error_propagates() {
    let config = Config {
        pipeline: Pipeline::Default {
            update: false,
            diff: true,
        },
        ..default_config()
    };
    let ui = MockUi {
        output: Box::new(|_| Err(Error::GitPullFailed)),
        ..ok_ui(&Log::default())
    };
    assert!(run_ok(&config, &ui).is_err());
}

#[test]
fn run_only_rebuild_skips_pull_and_stage() {
    let log = Log::default();
    let config = Config {
        pipeline: Pipeline::Only(vec![Step::Rebuild]),
        ..default_config()
    };
    assert!(
        run(
            &config,
            &MockRepo::default(),
            &MockNix::default(),
            &ok_ui(&log)
        )
        .is_ok()
    );
    assert!(log.contains("NixOS Rebuild"));
    assert!(!log.contains("Git Pull"));
    assert!(!log.contains("Git Changes"));
}

#[test]
fn run_only_cleanup_skips_rebuild() {
    let log = Log::default();
    let config = Config {
        pipeline: Pipeline::Only(vec![Step::Cleanup]),
        ..default_config()
    };
    assert!(
        run(
            &config,
            &MockRepo::default(),
            &MockNix::default(),
            &ok_ui(&log)
        )
        .is_ok()
    );
    assert!(!log.contains("NixOS Rebuild"));
}

#[test]
fn run_only_diff_runs_the_diff_alone() {
    let log = Log::default();
    let nix = MockNix {
        diff: Box::new(|| Ok(String::from("Version 1 -> 2\n"))),
        ..MockNix::default()
    };
    let config = Config {
        pipeline: Pipeline::Only(vec![Step::Diff]),

        ..default_config()
    };
    assert!(run(&config, &MockRepo::default(), &nix, &ok_ui(&log)).is_ok());
    assert!(log.contains("Version 1 -> 2"));
}
