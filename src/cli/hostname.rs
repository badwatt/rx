/// Resolve the hostname: `/proc/sys/kernel/hostname` → `HOSTNAME` env →
/// `hostname` command → `None`.
///
/// `None` means "could not detect"; the caller decides how to report it. No
/// sentinel string is smuggled through the domain.
pub(crate) fn resolve() -> Option<String> {
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

/// Inner logic extracted for testability. Returns the raw output; the caller
/// normalizes it.
fn from_cmd_inner(output: Option<std::process::Output>) -> Option<String> {
    let out = output?;
    // A failing `hostname` is no answer, whatever it printed.
    if !out.status.success() {
        return None;
    }
    String::from_utf8(out.stdout).ok()
}

/// Trim a source's answer; a blank one is no answer, so the next source runs.
fn clean(value: Option<String>) -> Option<String> {
    value
        .map(|name| name.trim().to_owned())
        .filter(|name| !name.is_empty())
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
/// and all failing yields `None`.
pub(crate) fn resolve_from(
    proc_hostname: impl FnOnce() -> Option<String>,
    env_hostname: impl FnOnce() -> Option<String>,
    cmd_hostname: impl FnOnce() -> Option<String>,
) -> Option<String> {
    clean(proc_hostname())
        .or_else(|| clean(env_hostname()))
        .or_else(|| clean(cmd_hostname()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolve_from_proc_succeeds() {
        let result = resolve_from(|| Some(String::from("myhost\n")), || None, || None);
        assert_eq!(result, Some(String::from("myhost")));
    }

    #[test]
    fn resolve_from_trims_the_env_answer() {
        let result = resolve_from(|| None, || Some(String::from("envhost\n")), || None);
        assert_eq!(result, Some(String::from("envhost")));
    }

    #[test]
    fn resolve_from_trims_the_command_answer() {
        let result = resolve_from(|| None, || None, || Some(String::from("cmdhost\n")));
        assert_eq!(result, Some(String::from("cmdhost")));
    }

    #[test]
    fn resolve_from_skips_a_blank_source() {
        let result = resolve_from(
            || Some(String::from("  \n")),
            || Some(String::from("envhost")),
            || None,
        );
        assert_eq!(result, Some(String::from("envhost")));
    }

    #[test]
    fn resolve_from_falls_back_to_env() {
        let result = resolve_from(|| None, || Some(String::from("envhost")), || None);
        assert_eq!(result, Some(String::from("envhost")));
    }

    #[test]
    fn resolve_from_falls_back_to_cmd() {
        let result = resolve_from(|| None, || None, || Some(String::from("cmdhost")));
        assert_eq!(result, Some(String::from("cmdhost")));
    }

    #[test]
    fn resolve_from_yields_none_when_every_source_fails() {
        assert!(resolve_from(|| None, || None, || None).is_none());
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
        assert_eq!(from_cmd_inner(output), Some(String::from("myhost\n")));
    }

    #[test]
    fn from_cmd_inner_returns_none_on_a_failing_command() {
        use std::os::unix::process::ExitStatusExt;
        let output = std::process::Output {
            status: std::process::ExitStatus::from_raw(1 << 8),
            stdout: b"junk\n".to_vec(),
            stderr: Vec::new(),
        };
        assert!(from_cmd_inner(Some(output)).is_none());
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
