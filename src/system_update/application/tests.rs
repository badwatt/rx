use super::*;
use crate::domain::config::BuildOptions;
use std::cell::RefCell;
use std::rc::Rc;

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
type PruneFn = Box<dyn Fn(u8) -> Result<(), Error>>;
type GcFn = Box<dyn Fn() -> Result<(), Error>>;
type DiffFn = Box<dyn Fn() -> Result<String, Error>>;

struct MockNix {
    update: StrFn,
    rebuild: RebuildFn,
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
type ConfirmFn = Box<dyn Fn(bool, Option<&str>) -> Result<bool, Error>>;
type MsgFn = Box<dyn Fn(&str) -> Result<(), Error>>;
type ConfigFn = Box<dyn Fn(&Config) -> Result<(), Error>>;

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
        jobs: None,
        cores: None,
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
        update: true,
        diff: true,
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
fn run_stage_add_failure_prints_failure_message() {
    let log = Log::default();
    let repo = MockRepo {
        changes: Box::new(|_| Ok(String::from(" M file\n"))),
        add_all: Box::new(|_| Err(Error::GitAddFailed)),
        ..MockRepo::default()
    };
    assert!(run(&default_config(), &repo, &MockNix::default(), &ok_ui(&log)).is_err());
    assert!(log.contains("Failed to add changes to the stage"));
}

#[test]
fn run_stage_add_spawn_failure_prints_detail() {
    let log = Log::default();
    let repo = MockRepo {
        changes: Box::new(|_| Ok(String::from(" M file\n"))),
        add_all: Box::new(|_| Err(Error::Io(std::io::Error::other("boom")))),
        ..MockRepo::default()
    };
    assert!(run(&default_config(), &repo, &MockNix::default(), &ok_ui(&log)).is_err());
    assert!(log.contains("Failed to add changes to the stage: "));
}

// ------------------------------------------------------------------
// pull / repo errors
// ------------------------------------------------------------------

#[test]
fn run_git_pull_non_zero_returns_error() {
    let log = Log::default();
    let repo = MockRepo {
        pull: Box::new(pull_errs),
        ..MockRepo::default()
    };
    assert!(run(&default_config(), &repo, &MockNix::default(), &ok_ui(&log)).is_err());
    assert!(log.contains("Failed to pull changes"));
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

#[test]
fn run_pull_error_output_error_propagates() {
    let repo = MockRepo {
        pull: Box::new(pull_errs),
        ..MockRepo::default()
    };
    let ui = MockUi {
        error: Box::new(|_| Err(Error::GitPullFailed)),
        ..ok_ui(&Log::default())
    };
    assert!(run(&default_config(), &repo, &MockNix::default(), &ui).is_err());
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
// per-step error propagation
// ------------------------------------------------------------------

#[test]
fn run_pull_title_error_propagates() {
    assert!(run_ok(&default_config(), &title_fails_on("Git Pull")).is_err());
}

#[test]
fn run_flake_title_error_propagates() {
    let config = Config {
        update: true,
        ..default_config()
    };
    assert!(run_ok(&config, &title_fails_on("Nix Update")).is_err());
}

#[test]
fn run_flake_error_propagates() {
    let config = Config {
        update: true,
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
fn run_stage_add_failure_output_error_propagates() {
    let repo = MockRepo {
        changes: Box::new(|_| Ok(String::from(" M file\n"))),
        add_all: Box::new(|_| Err(Error::GitAddFailed)),
        ..MockRepo::default()
    };
    let ui = MockUi {
        error: Box::new(|_| Err(Error::GitPullFailed)),
        ..ok_ui(&Log::default())
    };
    assert!(run(&default_config(), &repo, &MockNix::default(), &ui).is_err());
}

#[test]
fn run_stage_add_spawn_failure_output_error_propagates() {
    let repo = MockRepo {
        changes: Box::new(|_| Ok(String::from(" M file\n"))),
        add_all: Box::new(|_| Err(Error::Io(std::io::Error::other("boom")))),
        ..MockRepo::default()
    };
    let ui = MockUi {
        error: Box::new(|_| Err(Error::GitPullFailed)),
        ..ok_ui(&Log::default())
    };
    assert!(run(&default_config(), &repo, &MockNix::default(), &ui).is_err());
}

#[test]
fn run_rebuild_title_error_propagates() {
    assert!(run_ok(&default_config(), &title_fails_on("Nixos Rebuild")).is_err());
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
fn run_unknown_hostname_aborts_before_pull() {
    let config = Config {
        hostname: String::from("unknown"),
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
        diff: true,
        ..default_config()
    };
    assert!(run_ok(&config, &title_fails_on("Nix Diff")).is_err());
}

#[test]
fn run_diff_error_propagates() {
    let config = Config {
        diff: true,
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
        diff: true,
        ..default_config()
    };
    let ui = MockUi {
        output: Box::new(|_| Err(Error::GitPullFailed)),
        ..ok_ui(&Log::default())
    };
    assert!(run_ok(&config, &ui).is_err());
}
