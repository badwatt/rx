/// Build a `git pull` command argv for `repo`.
pub fn git_pull(repo: &str) -> Vec<String> {
    vec!["git".into(), "-C".into(), repo.into(), "pull".into()]
}

/// Build a `git diff --exit-code` command argv for `repo`.
pub fn git_diff(repo: &str) -> Vec<String> {
    vec![
        "git".into(),
        "-C".into(),
        repo.into(),
        "diff".into(),
        "--exit-code".into(),
    ]
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

/// Return the `nix profile diff-closures` pipeline string.
/// This requires shell execution (uses pipes).
pub fn nix_diff() -> &'static str {
    "nix profile diff-closures --profile /nix/var/nix/profiles/system | tac | awk '/Version/{print; exit} 1' | tac"
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn git_pull_argv() {
        assert_eq!(git_pull("/repo"), vec!["git", "-C", "/repo", "pull"]);
    }

    #[test]
    fn git_diff_argv() {
        assert_eq!(
            git_diff("/repo"),
            vec!["git", "-C", "/repo", "diff", "--exit-code"]
        );
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
    fn nix_diff_contains_diff_closures() {
        assert!(nix_diff().contains("diff-closures"));
    }
}
