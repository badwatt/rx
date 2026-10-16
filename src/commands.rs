/// Build a `git pull` command argv for `repo`.
pub fn git_pull(repo: &str) -> Vec<String> {
    vec!["git".into(), "-C".into(), repo.into(), "pull".into()]
}

/// Build a `git status --porcelain` command argv for `repo`.
pub fn git_status(repo: &str) -> Vec<String> {
    vec![
        "git".into(),
        "-C".into(),
        repo.into(),
        "status".into(),
        "--porcelain".into(),
    ]
}

/// Build a `git add .` command argv for `repo`.
pub fn git_add(repo: &str) -> Vec<String> {
    vec![
        "git".into(),
        "-C".into(),
        repo.into(),
        "add".into(),
        ".".into(),
    ]
}

/// Build a `nix flake update` command argv for `repo`.
pub fn nix_update(repo: &str) -> Vec<String> {
    vec![
        "nix".into(),
        "flake".into(),
        "update".into(),
        "--flake".into(),
        repo.into(),
    ]
}

/// Build a `nixos-rebuild switch` command argv for `repo` and `hostname`.
pub fn nix_rebuild(repo: &str, hostname: &str) -> Vec<String> {
    vec![
        "sudo".into(),
        "nixos-rebuild".into(),
        "switch".into(),
        "--flake".into(),
        format!("{repo}#{hostname}"),
        "--show-trace".into(),
    ]
}

/// Build a `nix-env --delete-generations` command argv keeping `generations`.
pub fn nix_keep(generations: u8) -> Vec<String> {
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
pub fn nix_collect_garbage() -> Vec<String> {
    vec!["sudo".into(), "nix-collect-garbage".into()]
}

/// Build a `nix profile diff-closures` command argv for the system profile.
pub fn nix_diff_closures() -> Vec<String> {
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
pub fn tail_from_last_version(output: &str) -> String {
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
    fn nix_rebuild_argv() {
        assert_eq!(
            nix_rebuild("/repo", "host"),
            vec![
                "sudo",
                "nixos-rebuild",
                "switch",
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
