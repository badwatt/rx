//! Command vocabulary: argv specs for the external tools rx drives (git, nix).
//!
//! Kept at the crate root on purpose. The `update` use-case needs to name the
//! commands it runs, and moving these specs into `adapters/` would invert the
//! hexagonal dependency direction (domain -> adapter). They are pure data
//! (no I/O); execution goes through the `CommandRunner` port.

/// Build a `git pull` command argv for `repo`.
pub(crate) fn git_pull(repo: &str) -> Vec<String> {
    vec!["git".into(), "-C".into(), repo.into(), "pull".into()]
}

/// Build a `git status --porcelain` command argv for `repo`.
pub(crate) fn git_status(repo: &str) -> Vec<String> {
    vec![
        "git".into(),
        "-C".into(),
        repo.into(),
        "status".into(),
        "--porcelain".into(),
    ]
}

/// Build a `git rev-parse --is-inside-work-tree` command argv for `repo`.
pub(crate) fn git_rev_parse(repo: &str) -> Vec<String> {
    vec![
        "git".into(),
        "-C".into(),
        repo.into(),
        "rev-parse".into(),
        "--is-inside-work-tree".into(),
    ]
}

/// Build a `git add .` command argv for `repo`.
pub(crate) fn git_add(repo: &str) -> Vec<String> {
    vec![
        "git".into(),
        "-C".into(),
        repo.into(),
        "add".into(),
        ".".into(),
    ]
}

/// Build a `nix flake update` command argv for `repo`.
pub(crate) fn nix_update(repo: &str) -> Vec<String> {
    vec![
        "nix".into(),
        "flake".into(),
        "update".into(),
        "--flake".into(),
        repo.into(),
    ]
}

/// Which `nixos-rebuild` implementation is installed.
///
/// The classic shell script elevates the whole command and skips the
/// self-rebuild re-exec with `--fast`; the `-ng` rewrite elevates only the
/// activation with `--elevate=sudo` and spells the skip `--no-reexec`.
#[derive(Debug, PartialEq, Clone, Copy)]
pub(crate) enum RebuildFlavor {
    Classic,
    Ng,
}

/// Capability probe for the installed `nixos-rebuild`.
///
/// `--elevate` only exists in the `-ng` rewrite, and `list-generations` is
/// read-only, so exit code 0 means `-ng`.
pub(crate) fn rebuild_probe() -> Vec<String> {
    vec![
        "nixos-rebuild".into(),
        "--elevate=none".into(),
        "list-generations".into(),
    ]
}

/// Build a `nixos-rebuild switch` command argv for `repo` and `hostname`.
///
/// The self-rebuild re-exec is always skipped, with the flavor-appropriate
/// flag, so the redundant pre-build evaluation is not paid.
pub(crate) fn nix_rebuild(repo: &str, hostname: &str, flavor: RebuildFlavor) -> Vec<String> {
    let flake = format!("{repo}#{hostname}");
    match flavor {
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
    }
}

/// Build a `nix-env --delete-generations` command argv keeping `generations`.
pub(crate) fn nix_keep(generations: u8) -> Vec<String> {
    vec![
        "sudo".into(),
        "nix-env".into(),
        "--profile".into(),
        "/nix/var/nix/profiles/system".into(),
        "--delete-generations".into(),
        format!("+{generations}"),
    ]
}

/// Build a `nix-collect-garbage` command argv to free unreferenced store paths.
pub(crate) fn nix_collect_garbage() -> Vec<String> {
    vec!["sudo".into(), "nix-collect-garbage".into()]
}

/// Build a `nix profile diff-closures` command argv for the system profile.
pub(crate) fn nix_diff_closures() -> Vec<String> {
    vec![
        "nix".into(),
        "profile".into(),
        "diff-closures".into(),
        "--profile".into(),
        "/nix/var/nix/profiles/system".into(),
    ]
}

/// Return the tail of `output`, starting at the last line containing `Version`.
///
/// Mirrors the former `tac | awk '/Version/{print; exit} 1' | tac` pipeline:
/// when no `Version` line exists, the whole output is returned unchanged.
pub(crate) fn tail_from_last_version(output: &str) -> String {
    let lines: Vec<&str> = output.lines().collect();
    match lines.iter().rposition(|line| line.contains("Version")) {
        Some(start) => {
            let mut result = lines[start..].join("\n");
            result.push('\n');
            result
        }
        None => output.to_owned(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn git_pull_argv() {
        assert_eq!(git_pull("/repo"), vec!["git", "-C", "/repo", "pull"]);
    }

    #[test]
    fn git_status_argv() {
        assert_eq!(
            git_status("/repo"),
            vec!["git", "-C", "/repo", "status", "--porcelain"]
        );
    }

    #[test]
    fn git_rev_parse_argv() {
        assert_eq!(
            git_rev_parse("/repo"),
            vec!["git", "-C", "/repo", "rev-parse", "--is-inside-work-tree"]
        );
    }

    #[test]
    fn git_add_argv() {
        assert_eq!(git_add("/repo"), vec!["git", "-C", "/repo", "add", "."]);
    }

    #[test]
    fn nix_update_argv() {
        assert_eq!(
            nix_update("/repo"),
            vec!["nix", "flake", "update", "--flake", "/repo"]
        );
    }

    #[test]
    fn rebuild_probe_argv() {
        assert_eq!(
            rebuild_probe(),
            vec!["nixos-rebuild", "--elevate=none", "list-generations"]
        );
    }

    #[test]
    fn nix_rebuild_argv_classic_uses_sudo_and_fast() {
        assert_eq!(
            nix_rebuild("/repo", "host", RebuildFlavor::Classic),
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
    fn nix_rebuild_argv_ng_uses_elevate_and_no_reexec() {
        assert_eq!(
            nix_rebuild("/repo", "host", RebuildFlavor::Ng),
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
    fn nix_keep_argv() {
        assert_eq!(
            nix_keep(5),
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
    fn nix_collect_garbage_argv() {
        assert_eq!(nix_collect_garbage(), vec!["sudo", "nix-collect-garbage"]);
    }

    #[test]
    fn nix_diff_closures_argv() {
        assert_eq!(
            nix_diff_closures(),
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
    fn tail_from_last_version_keeps_from_last_match() {
        let output = "old Version 1\nnoise\nVersion 2\nline a\nline b\n";
        assert_eq!(
            tail_from_last_version(output),
            "Version 2\nline a\nline b\n"
        );
    }

    #[test]
    fn tail_from_last_version_without_match_returns_all() {
        let output = "line a\nline b\n";
        assert_eq!(tail_from_last_version(output), "line a\nline b\n");
    }

    #[test]
    fn tail_from_last_version_empty_returns_empty() {
        assert_eq!(tail_from_last_version(""), "");
    }

    #[test]
    fn tail_from_last_version_match_on_last_line() {
        assert_eq!(tail_from_last_version("a\nVersion 3\n"), "Version 3\n");
    }
}
