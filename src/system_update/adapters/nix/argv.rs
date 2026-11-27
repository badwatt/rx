//! Argv builders for the nix commands; pure, so they are tested without
//! spawning anything.

use super::{BuildOptions, RebuildFlavor};

/// Capability probe for the installed `nixos-rebuild`.
///
/// `--elevate` only exists in the `-ng` rewrite, and `list-generations` is
/// read-only, so exit code 0 means `-ng`.
pub(super) fn rebuild_probe() -> Vec<String> {
    vec![
        "nixos-rebuild".into(),
        "--elevate=none".into(),
        "list-generations".into(),
    ]
}

/// `sudo -n true`: succeeds only when sudo would not ask for a password.
pub(super) fn sudo_probe() -> Vec<String> {
    vec!["sudo".into(), "-n".into(), "true".into()]
}

/// `sudo -v`: validate (or prompt for) the sudo credentials up front.
pub(super) fn sudo_validate() -> Vec<String> {
    vec!["sudo".into(), "-v".into()]
}

/// `nom`: the monitor the rebuild's log is piped through.
pub(super) fn cmd_nom() -> Vec<String> {
    vec!["nom".into()]
}

/// `nom --version`: succeeds only when `nom` is installed.
pub(super) fn cmd_nom_probe() -> Vec<String> {
    vec!["nom".into(), "--version".into()]
}

/// `sudo -n -v`: renew the cached credentials only when no prompt is needed.
pub(super) fn sudo_keep_alive() -> Vec<String> {
    vec!["sudo".into(), "-n".into(), "-v".into()]
}

/// `nix flake update --flake <repo>`
pub(super) fn cmd_update(repo: &str) -> Vec<String> {
    vec![
        "nix".into(),
        "flake".into(),
        "update".into(),
        "--flake".into(),
        repo.into(),
    ]
}

/// `nixos-rebuild switch <flavor flags> --flake <repo>#<hostname> --show-trace`
/// plus any parallelism caps.
pub(super) fn cmd_rebuild(
    repo: &str,
    hostname: &str,
    flavor: RebuildFlavor,
    options: BuildOptions,
) -> Vec<String> {
    let flake = format!("{repo}#{hostname}");
    let mut cmd = match flavor {
        RebuildFlavor::Ng => vec![
            "nixos-rebuild".into(),
            "switch".into(),
            "--elevate=sudo".into(),
            "--no-reexec".into(),
            "--flake".into(),
            flake,
            "--show-trace".into(),
        ],
        RebuildFlavor::Classic => vec![
            "sudo".into(),
            "nixos-rebuild".into(),
            "switch".into(),
            "--fast".into(),
            "--flake".into(),
            flake,
            "--show-trace".into(),
        ],
    };
    if let Some(jobs) = options.jobs {
        cmd.push("--max-jobs".into());
        cmd.push(jobs.to_string());
    }
    if let Some(cores) = options.cores {
        cmd.push("--cores".into());
        cmd.push(cores.to_string());
    }
    cmd
}

/// `nix-env --profile /nix/var/nix/profiles/system --delete-generations +<keep>`
pub(super) fn cmd_prune(keep: u8) -> Vec<String> {
    vec![
        "sudo".into(),
        "nix-env".into(),
        "--profile".into(),
        "/nix/var/nix/profiles/system".into(),
        "--delete-generations".into(),
        format!("+{keep}"),
    ]
}

/// `nix store gc` (modern replacement for `nix-collect-garbage`).
pub(super) fn cmd_collect_garbage() -> Vec<String> {
    vec!["sudo".into(), "nix".into(), "store".into(), "gc".into()]
}

/// `nix profile diff-closures --profile /nix/var/nix/profiles/system`
pub(super) fn cmd_diff_closures() -> Vec<String> {
    vec![
        "nix".into(),
        "profile".into(),
        "diff-closures".into(),
        "--profile".into(),
        "/nix/var/nix/profiles/system".into(),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::error::Error;
    use crate::system_update::adapters::nix::{NixCli, refresh_sudo_ticket, spawn_keep_alive};
    use crate::system_update::ports::nix::Nix as _;
    use crate::test_support::StubRunner;
    use std::cell::RefCell;
    use std::rc::Rc;

    #[test]
    fn rebuild_probe_argv() {
        assert_eq!(
            rebuild_probe(),
            vec!["nixos-rebuild", "--elevate=none", "list-generations"]
        );
    }

    #[test]
    fn sudo_probe_argv() {
        assert_eq!(sudo_probe(), vec!["sudo", "-n", "true"]);
    }

    #[test]
    fn cmd_nom_argv() {
        assert_eq!(cmd_nom(), vec!["nom"]);
        assert_eq!(cmd_nom_probe(), vec!["nom", "--version"]);
    }

    #[test]
    fn sudo_keep_alive_argv() {
        assert_eq!(sudo_keep_alive(), vec!["sudo", "-n", "-v"]);
    }

    #[test]
    fn sudo_validate_argv() {
        assert_eq!(sudo_validate(), vec!["sudo", "-v"]);
    }

    #[test]
    fn cmd_update_argv() {
        assert_eq!(
            cmd_update("/repo"),
            vec!["nix", "flake", "update", "--flake", "/repo"]
        );
    }

    #[test]
    fn cmd_rebuild_ng_argv() {
        assert_eq!(
            cmd_rebuild("/repo", "host", RebuildFlavor::Ng, BuildOptions::default()),
            vec![
                "nixos-rebuild",
                "switch",
                "--elevate=sudo",
                "--no-reexec",
                "--flake",
                "/repo#host",
                "--show-trace"
            ]
        );
    }

    #[test]
    fn cmd_rebuild_classic_argv() {
        assert_eq!(
            cmd_rebuild(
                "/repo",
                "host",
                RebuildFlavor::Classic,
                BuildOptions::default()
            ),
            vec![
                "sudo",
                "nixos-rebuild",
                "switch",
                "--fast",
                "--flake",
                "/repo#host",
                "--show-trace"
            ]
        );
    }

    #[test]
    fn cmd_rebuild_appends_parallelism_caps() {
        let options = BuildOptions {
            jobs: Some(2),
            cores: Some(4),
        };
        let cmd = cmd_rebuild("/repo", "host", RebuildFlavor::Ng, options);
        assert_eq!(
            cmd,
            vec![
                "nixos-rebuild",
                "switch",
                "--elevate=sudo",
                "--no-reexec",
                "--flake",
                "/repo#host",
                "--show-trace",
                "--max-jobs",
                "2",
                "--cores",
                "4"
            ]
        );
    }

    #[test]
    fn cmd_prune_argv() {
        assert_eq!(
            cmd_prune(5),
            vec![
                "sudo",
                "nix-env",
                "--profile",
                "/nix/var/nix/profiles/system",
                "--delete-generations",
                "+5"
            ]
        );
    }

    #[test]
    fn cmd_collect_garbage_argv() {
        assert_eq!(cmd_collect_garbage(), vec!["sudo", "nix", "store", "gc"]);
    }

    #[test]
    fn cmd_diff_closures_argv() {
        assert_eq!(
            cmd_diff_closures(),
            vec![
                "nix",
                "profile",
                "diff-closures",
                "--profile",
                "/nix/var/nix/profiles/system"
            ]
        );
    }

    #[test]
    fn password_pending_true_when_sudo_would_prompt() {
        let runner = StubRunner::new().with_probe(|_| Ok(1));
        assert!(NixCli::new(&runner).password_pending().expect("sudo probe"));
    }

    #[test]
    fn password_pending_false_when_credentials_are_cached() {
        let runner = StubRunner::new().with_probe(|_| Ok(0));
        assert!(!NixCli::new(&runner).password_pending().expect("sudo probe"));
    }

    #[test]
    fn password_pending_true_when_the_probe_errors() {
        let runner = StubRunner::new().with_probe(|_| Err(Error::Io(std::io::Error::other("x"))));
        assert!(NixCli::new(&runner).password_pending().expect("sudo probe"));
    }

    #[test]
    fn keep_alive_refreshes_until_the_probe_fails() {
        use std::sync::atomic::{AtomicUsize, Ordering};
        let calls = std::sync::Arc::new(AtomicUsize::new(0));
        let counter = std::sync::Arc::clone(&calls);
        let handle = spawn_keep_alive(
            std::time::Duration::ZERO,
            Box::new(move || counter.fetch_add(1, Ordering::SeqCst) < 2),
        );
        handle.join().expect("keep-alive thread");
        assert_eq!(calls.load(Ordering::SeqCst), 3);
    }

    #[test]
    fn refresh_sudo_ticket_reports_the_probe_outcome() {
        let _ = refresh_sudo_ticket();
    }

    #[test]
    fn refresh_credentials_ok_on_zero() {
        let runner = StubRunner::new();
        assert!(NixCli::new(&runner).refresh_credentials().is_ok());
    }

    #[test]
    fn refresh_credentials_errs_on_nonzero() {
        let runner = StubRunner::new().with_run(|_| Ok(1));
        let err = NixCli::new(&runner)
            .refresh_credentials()
            .expect_err("refused password");
        assert_eq!(err.to_string(), "Failed to obtain sudo credentials");
    }

    #[test]
    fn refresh_credentials_propagates_run_error() {
        let runner = StubRunner::new().with_run(|_| Err(Error::Io(std::io::Error::other("x"))));
        let err = NixCli::new(&runner)
            .refresh_credentials()
            .expect_err("run error");
        assert!(err.to_string().contains("IO error"));
    }

    #[test]
    fn update_ok_on_zero() {
        let runner = StubRunner::new();
        assert!(NixCli::new(&runner).update("/repo", None).is_ok());
    }

    #[test]
    fn update_runs_the_configured_command_at_the_repo_root() {
        let seen = Rc::new(RefCell::new(Vec::new()));
        let recorder = Rc::clone(&seen);
        let runner = StubRunner::new().with_run_in(move |dir, cmd| {
            recorder.borrow_mut().push(dir.to_owned());
            recorder.borrow_mut().extend_from_slice(cmd);
            Ok(0)
        });
        let command = vec![String::from("tack"), String::from("update")];
        assert!(NixCli::new(&runner).update("/repo", Some(&command)).is_ok());
        assert_eq!(seen.borrow().join(" "), "/repo tack update");
    }

    #[test]
    fn update_errs_when_the_configured_command_fails() {
        let runner = StubRunner::new().with_run_in(|_, _| Ok(1));
        let command = vec![String::from("tack")];
        let err = NixCli::new(&runner)
            .update("/repo", Some(&command))
            .expect_err("custom command failure");
        assert_eq!(err.to_string(), "Failed to update flake inputs");
    }

    #[test]
    fn update_propagates_a_configured_command_run_error() {
        let runner =
            StubRunner::new().with_run_in(|_, _| Err(Error::Io(std::io::Error::other("x"))));
        let command = vec![String::from("tack")];
        let err = NixCli::new(&runner)
            .update("/repo", Some(&command))
            .expect_err("run error");
        assert!(err.to_string().contains("IO error"));
    }

    #[test]
    fn update_errs_on_nonzero() {
        let runner = StubRunner::new().with_run(|_| Ok(1));
        let err = NixCli::new(&runner)
            .update("/repo", None)
            .expect_err("update failure");
        assert_eq!(err.to_string(), "Failed to update flake inputs");
    }

    #[test]
    fn update_propagates_run_error() {
        let runner = StubRunner::new().with_run(|_| Err(Error::Io(std::io::Error::other("x"))));
        let err = NixCli::new(&runner)
            .update("/repo", None)
            .expect_err("run error");
        assert!(err.to_string().contains("IO error"));
    }

    #[test]
    fn rebuild_ok_and_probes_ng_by_default() {
        let runner = StubRunner::new();
        assert!(
            NixCli::new(&runner)
                .rebuild("/repo", "host", BuildOptions::default(), Some(false))
                .is_ok()
        );
    }

    #[test]
    fn rebuild_ng_uses_elevate_and_no_reexec() {
        let seen = Rc::new(RefCell::new(Vec::new()));
        let recorder = Rc::clone(&seen);
        let runner = StubRunner::new().with_run(move |cmd| {
            recorder.borrow_mut().extend_from_slice(cmd);
            Ok(0)
        });
        assert!(
            NixCli::new(&runner)
                .rebuild("/repo", "host", BuildOptions::default(), Some(false))
                .is_ok()
        );
        let joined = seen.borrow().join(" ");
        assert!(joined.contains("--elevate=sudo"));
        assert!(joined.contains("--no-reexec"));
    }

    #[test]
    fn rebuild_classic_uses_sudo_and_fast() {
        let seen = Rc::new(RefCell::new(Vec::new()));
        let recorder = Rc::clone(&seen);
        let runner = StubRunner::new()
            .with_probe(|_| Ok(1))
            .with_run(move |cmd| {
                recorder.borrow_mut().extend_from_slice(cmd);
                Ok(0)
            });
        assert!(
            NixCli::new(&runner)
                .rebuild("/repo", "host", BuildOptions::default(), Some(false))
                .is_ok()
        );
        let joined = seen.borrow().join(" ");
        assert!(joined.contains("sudo"));
        assert!(joined.contains("--fast"));
    }

    #[test]
    fn rebuild_classic_when_probe_errors() {
        let runner = StubRunner::new().with_probe(|_| Err(Error::Io(std::io::Error::other("x"))));
        assert!(
            NixCli::new(&runner)
                .rebuild("/repo", "host", BuildOptions::default(), Some(false))
                .is_ok()
        );
    }

    #[test]
    fn rebuild_pipes_through_nom_when_asked() {
        let seen = Rc::new(RefCell::new(vec![String::new()]));
        let recorder = Rc::clone(&seen);
        let runner = StubRunner::new().with_run_through(move |cmd, consumer| {
            recorder.borrow_mut().extend_from_slice(cmd);
            recorder.borrow_mut().push("|".to_owned());
            recorder.borrow_mut().extend_from_slice(consumer);
            Ok(0)
        });
        assert!(
            NixCli::new(&runner)
                .rebuild("/repo", "host", BuildOptions::default(), Some(true))
                .is_ok()
        );
        let joined = seen.borrow().join(" ");
        assert!(joined.contains("nixos-rebuild"));
        assert!(joined.ends_with("| nom"));
    }

    #[test]
    fn rebuild_nom_errs_on_nonzero() {
        let runner = StubRunner::new().with_run_through(|_, _| Ok(1));
        let err = NixCli::new(&runner)
            .rebuild("/repo", "host", BuildOptions::default(), Some(true))
            .expect_err("rebuild failure");
        assert_eq!(err.to_string(), "NixOS rebuild failed");
    }

    #[test]
    fn rebuild_forced_nom_missing_returns_the_hint() {
        let runner = StubRunner::new().with_probe(|_| {
            Err(Error::Io(std::io::Error::from(
                std::io::ErrorKind::NotFound,
            )))
        });
        let err = NixCli::new(&runner)
            .rebuild("/repo", "host", BuildOptions::default(), Some(true))
            .expect_err("missing monitor");
        assert!(err.to_string().contains("--nom=false"));
    }

    #[test]
    fn rebuild_uses_nom_when_it_is_installed() {
        let seen = Rc::new(std::cell::Cell::new(false));
        let recorder = Rc::clone(&seen);
        let runner = StubRunner::new().with_run_through(move |_, _| {
            recorder.set(true);
            Ok(0)
        });
        assert!(
            NixCli::new(&runner)
                .rebuild("/repo", "host", BuildOptions::default(), None)
                .is_ok()
        );
        assert!(seen.get());
    }

    #[test]
    fn rebuild_skips_nom_when_it_is_missing() {
        let seen = Rc::new(std::cell::Cell::new(false));
        let recorder = Rc::clone(&seen);
        let runner = StubRunner::new()
            .with_probe(|_| Err(Error::Io(std::io::Error::other("missing"))))
            .with_run(move |_| {
                recorder.set(true);
                Ok(0)
            });
        assert!(
            NixCli::new(&runner)
                .rebuild("/repo", "host", BuildOptions::default(), None)
                .is_ok()
        );
        assert!(seen.get());
    }

    #[test]
    fn rebuild_nom_propagates_the_run_error() {
        let runner =
            StubRunner::new().with_run_through(|_, _| Err(Error::Io(std::io::Error::other("x"))));
        let err = NixCli::new(&runner)
            .rebuild("/repo", "host", BuildOptions::default(), Some(true))
            .expect_err("run error");
        assert!(err.to_string().contains("IO error"));
    }

    #[test]
    fn rebuild_errs_on_nonzero() {
        let runner = StubRunner::new().with_run(|_| Ok(1));
        let err = NixCli::new(&runner)
            .rebuild("/repo", "host", BuildOptions::default(), Some(false))
            .expect_err("rebuild failure");
        assert_eq!(err.to_string(), "NixOS rebuild failed");
    }

    #[test]
    fn rebuild_propagates_run_error() {
        let runner = StubRunner::new().with_run(|_| Err(Error::Io(std::io::Error::other("x"))));
        let err = NixCli::new(&runner)
            .rebuild("/repo", "host", BuildOptions::default(), Some(false))
            .expect_err("run error");
        assert!(err.to_string().contains("IO error"));
    }

    #[test]
    fn prune_ok_on_zero() {
        let runner = StubRunner::new();
        assert!(NixCli::new(&runner).prune(5).is_ok());
    }

    #[test]
    fn prune_errs_on_nonzero() {
        let runner = StubRunner::new().with_run(|_| Ok(1));
        let err = NixCli::new(&runner).prune(5).expect_err("prune failure");
        assert_eq!(err.to_string(), "Cleanup failed");
    }

    #[test]
    fn prune_propagates_run_error() {
        let runner = StubRunner::new().with_run(|_| Err(Error::Io(std::io::Error::other("x"))));
        let err = NixCli::new(&runner).prune(5).expect_err("run error");
        assert!(err.to_string().contains("IO error"));
    }

    #[test]
    fn collect_garbage_ok_on_zero() {
        let runner = StubRunner::new();
        assert!(NixCli::new(&runner).collect_garbage().is_ok());
    }

    #[test]
    fn collect_garbage_errs_on_nonzero() {
        let runner = StubRunner::new().with_run(|_| Ok(1));
        let err = NixCli::new(&runner)
            .collect_garbage()
            .expect_err("gc failure");
        assert_eq!(err.to_string(), "Cleanup failed");
    }

    #[test]
    fn collect_garbage_propagates_run_error() {
        let runner = StubRunner::new().with_run(|_| Err(Error::Io(std::io::Error::other("x"))));
        let err = NixCli::new(&runner)
            .collect_garbage()
            .expect_err("run error");
        assert!(err.to_string().contains("IO error"));
    }

    #[test]
    fn diff_closures_returns_output() {
        let runner = StubRunner::new().with_capture(|_| Ok((0, String::from("Version 1 -> 2\n"))));
        assert_eq!(
            NixCli::new(&runner).diff_closures().expect("diff"),
            "Version 1 -> 2\n"
        );
    }

    #[test]
    fn diff_closures_errs_on_nonzero() {
        let runner = StubRunner::new().with_capture(|_| Ok((1, String::from("junk"))));
        let err = NixCli::new(&runner)
            .diff_closures()
            .expect_err("diff failure");
        assert_eq!(err.to_string(), "Failed to compute closure diff");
    }

    #[test]
    fn diff_closures_propagates_capture_error() {
        let runner = StubRunner::new().with_capture(|_| Err(Error::Io(std::io::Error::other("x"))));
        let err = NixCli::new(&runner)
            .diff_closures()
            .expect_err("capture error");
        assert!(err.to_string().contains("IO error"));
    }
}
