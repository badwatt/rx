/// Resolve the hostname: `/proc/sys/kernel/hostname` → `HOSTNAME` env →
/// `hostname` command → `"unknown"`.
pub(crate) fn resolve() -> String {
    resolve_from(
        || std::fs::read_to_string(resolve_path()).ok(),
        || std::env::var("HOSTNAME").ok(),
        from_cmd,
    )
}

/// Read hostname from the `hostname` command output.
fn from_cmd() -> Option<String> {
    from_cmd_inner(std::process::Command::new("hostname").output().ok())
}

/// Inner logic extracted for testability.
fn from_cmd_inner(output: Option<std::process::Output>) -> Option<String> {
    let out = output?;
    String::from_utf8(out.stdout).ok()
}

#[cfg(not(test))]
fn resolve_path() -> &'static str {
    "/proc/sys/kernel/hostname"
}

#[cfg(test)]
fn resolve_path() -> &'static str {
    "/nonexistent_hostname_path"
}

/// Parameterized hostname resolution for testability.
///
/// `proc_hostname` tries to read `/proc/sys/kernel/hostname`,
/// `env_hostname` falls back to the `HOSTNAME` env var,
/// `cmd_hostname` falls back to the `hostname` command output,
/// and all failing yields `"unknown"`.
pub(crate) fn resolve_from(
    proc_hostname: impl FnOnce() -> Option<String>,
    env_hostname: impl FnOnce() -> Option<String>,
    cmd_hostname: impl FnOnce() -> Option<String>,
) -> String {
    proc_hostname()
        .map(|s| s.trim().to_owned())
        .or_else(env_hostname)
        .or_else(cmd_hostname)
        .unwrap_or_else(|| String::from("unknown"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolve_from_proc_succeeds() {
        let result = resolve_from(|| Some(String::from("myhost\n")), || None, || None);
        assert_eq!(result, "myhost");
    }

    #[test]
    fn resolve_from_falls_back_to_env() {
        let result = resolve_from(|| None, || Some(String::from("envhost")), || None);
        assert_eq!(result, "envhost");
    }

    #[test]
    fn resolve_from_falls_back_to_cmd() {
        let result = resolve_from(|| None, || None, || Some(String::from("cmdhost")));
        assert_eq!(result, "cmdhost");
    }

    #[test]
    fn resolve_from_falls_back_to_unknown() {
        let result = resolve_from(|| None, || None, || None);
        assert_eq!(result, "unknown");
    }

    #[test]
    fn resolve_runs_without_panic() {
        let _ = resolve();
    }

    #[test]
    fn from_cmd_inner_returns_some_on_valid_output() {
        use std::os::unix::process::ExitStatusExt;
        let output = Some(std::process::Output {
            status: std::process::ExitStatus::from_raw(0),
            stdout: b"myhost\n".to_vec(),
            stderr: Vec::new(),
        });
        let result = from_cmd_inner(output);
        assert!(result.is_some());
        assert!(!result.expect("hostname").trim().is_empty());
    }

    #[test]
    fn from_cmd_inner_returns_none_on_bad_utf8() {
        use std::os::unix::process::ExitStatusExt;
        let output = std::process::Output {
            status: std::process::ExitStatus::from_raw(0),
            stdout: vec![0xff, 0xfe], // invalid UTF-8
            stderr: Vec::new(),
        };
        assert!(from_cmd_inner(Some(output)).is_none());
    }

    #[test]
    fn from_cmd_inner_returns_none_on_none() {
        assert!(from_cmd_inner(None).is_none());
    }
}
