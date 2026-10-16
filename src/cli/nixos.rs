//! NixOS release detection.
//!
//! `nixos-rebuild-ng` became the default in NixOS 25.11 and supports the
//! `--elevate=sudo` and `--no-reexec` flags. Older (classic) releases do not,
//! so we must pick the command form accordingly.

/// Return `true` when the running system is NixOS >= 25.11 (ng `nixos-rebuild`).
pub fn supports_elevate() -> bool {
    supports_elevate_from(read_os_release().as_deref())
}

/// Pure decision over the optional `/etc/os-release` contents.
fn supports_elevate_from(contents: Option<&str>) -> bool {
    contents
        .and_then(version_id)
        .is_some_and(|version| version_at_least(&version, 25, 11))
}

/// Extract the `VERSION_ID` value from `/etc/os-release` contents.
fn version_id(contents: &str) -> Option<String> {
    contents.lines().find_map(|line| {
        let value = line.strip_prefix("VERSION_ID=")?;
        Some(value.trim().trim_matches('"').to_owned())
    })
}

/// Return `true` when `version` (`"25.11"`, `"26.05"`, …) is `>= major.minor`.
fn version_at_least(version: &str, major: u32, minor: u32) -> bool {
    let mut parts = version.split('.');
    let parsed_major = parts.next().and_then(|s| s.parse().ok()).unwrap_or(0);
    let parsed_minor = parts.next().and_then(|s| s.parse().ok()).unwrap_or(0);
    (parsed_major, parsed_minor) >= (major, minor)
}

#[cfg(not(test))]
fn os_release_path() -> &'static str {
    "/etc/os-release"
}

#[cfg(test)]
fn os_release_path() -> &'static str {
    "/nonexistent_os_release"
}

/// Read `/etc/os-release`, returning `None` when unavailable.
fn read_os_release() -> Option<String> {
    std::fs::read_to_string(os_release_path()).ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn version_id_parses_quoted_value() {
        let contents = "NAME=\"NixOS\"\nVERSION_ID=\"25.11\"\nID=nixos\n";
        assert_eq!(version_id(contents), Some(String::from("25.11")));
    }

    #[test]
    fn version_id_parses_unquoted_value() {
        assert_eq!(
            version_id("VERSION_ID=26.05\n"),
            Some(String::from("26.05"))
        );
    }

    #[test]
    fn version_id_missing_returns_none() {
        assert!(version_id("ID=ubuntu\n").is_none());
    }

    #[test]
    fn version_at_least_at_threshold_is_true() {
        assert!(version_at_least("25.11", 25, 11));
    }

    #[test]
    fn version_at_least_above_threshold_is_true() {
        assert!(version_at_least("26.05", 25, 11));
    }

    #[test]
    fn version_at_least_below_threshold_is_false() {
        assert!(!version_at_least("25.05", 25, 11));
        assert!(!version_at_least("24.11", 25, 11));
    }

    #[test]
    fn version_at_least_major_only_is_false() {
        assert!(!version_at_least("25", 25, 11));
    }

    #[test]
    fn version_at_least_non_numeric_is_false() {
        assert!(!version_at_least("unstable", 25, 11));
    }

    #[test]
    fn supports_elevate_from_modern_is_true() {
        assert!(supports_elevate_from(Some("VERSION_ID=\"25.11\"\n")));
    }

    #[test]
    fn supports_elevate_from_old_is_false() {
        assert!(!supports_elevate_from(Some("VERSION_ID=\"25.05\"\n")));
    }

    #[test]
    fn supports_elevate_from_missing_id_is_false() {
        assert!(!supports_elevate_from(Some("ID=ubuntu\n")));
    }

    #[test]
    fn supports_elevate_from_none_is_false() {
        assert!(!supports_elevate_from(None));
    }

    #[test]
    fn supports_elevate_runs_without_panic() {
        let _ = supports_elevate();
    }
}
