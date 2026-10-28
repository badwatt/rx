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
    use crate::shared::test_helpers::StubRunner;
    use crate::system_update::adapters::nix::NixCli;
    use crate::system_update::ports::nix::Nix as _;
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
    fn update_ok_on_zero() {
        let runner = StubRunner::new();
        assert!(NixCli::new(&runner).update("/repo").is_ok());
    }

    #[test]
    fn update_errs_on_nonzero() {
        let runner = StubRunner::new().with_run(|_| Ok(1));
        let err = NixCli::new(&runner)
            .update("/repo")
            .expect_err("update failure");
        assert_eq!(err.to_string(), "Failed to update flake inputs");
    }

    #[test]
    fn update_propagates_run_error() {
        let runner = StubRunner::new().with_run(|_| Err(Error::Io(std::io::Error::other("x"))));
        let err = NixCli::new(&runner).update("/repo").expect_err("run error");
        assert!(err.to_string().contains("IO error"));
    }

    #[test]
    fn rebuild_ok_and_probes_ng_by_default() {
        let runner = StubRunner::new();
        assert!(
            NixCli::new(&runner)
                .rebuild("/repo", "host", BuildOptions::default())
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
                .rebuild("/repo", "host", BuildOptions::default())
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
                .rebuild("/repo", "host", BuildOptions::default())
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
                .rebuild("/repo", "host", BuildOptions::default())
                .is_ok()
        );
    }

    #[test]
    fn rebuild_errs_on_nonzero() {
        let runner = StubRunner::new().with_run(|_| Ok(1));
        let err = NixCli::new(&runner)
            .rebuild("/repo", "host", BuildOptions::default())
            .expect_err("rebuild failure");
        assert_eq!(err.to_string(), "NixOS rebuild failed");
    }

    #[test]
    fn rebuild_propagates_run_error() {
        let runner = StubRunner::new().with_run(|_| Err(Error::Io(std::io::Error::other("x"))));
        let err = NixCli::new(&runner)
            .rebuild("/repo", "host", BuildOptions::default())
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
