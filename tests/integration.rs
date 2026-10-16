use std::io::Write;
use std::process::{Command, Output, Stdio};

/// Path to the compiled `rx` binary, provided by Cargo for integration tests.
const RX: &str = env!("CARGO_BIN_EXE_rx");

/// Run `rx` with `args` (stdin closed), capturing stdout/stderr and status.
fn run(args: &[&str]) -> Output {
    Command::new(RX).args(args).output().expect("run rx")
}

fn stdout_of(output: &Output) -> String {
    String::from_utf8_lossy(&output.stdout).into_owned()
}

#[test]
fn help_flag_prints_rx_banner() {
    let output = run(&["-h"]);
    assert!(stdout_of(&output).contains("RX"));
}

#[test]
fn help_long_flag_prints_banner() {
    let output = run(&["--help"]);
    assert!(output.status.success());
    assert!(stdout_of(&output).contains("RX"));
}

#[test]
fn help_word_prints_help_and_exits_zero() {
    let output = run(&["help"]);
    assert!(output.status.success());
    assert!(stdout_of(&output).contains("RX"));
}

#[test]
fn version_flag_prints_semver() {
    assert!(stdout_of(&run(&["-v"])).contains("0.1.0"));
}

#[test]
fn version_long_flag_prints_semver() {
    assert!(stdout_of(&run(&["--version"])).contains("0.1.0"));
}

#[test]
fn version_word_prints_version() {
    assert!(stdout_of(&run(&["version"])).contains("RX version"));
}

#[test]
fn unknown_flag_exits_nonzero() {
    let output = run(&["-x"]);
    assert!(!output.status.success());
    assert_eq!(output.status.code(), Some(2));
}

#[test]
fn invalid_k_flag_exits_nonzero() {
    let output = run(&["-k", "abc"]);
    assert!(!output.status.success());
    assert_eq!(output.status.code(), Some(2));
}

#[test]
fn zero_keep_flag_exits_nonzero() {
    let output = run(&["-k", "0"]);
    assert!(!output.status.success());
    assert_eq!(output.status.code(), Some(2));
}

#[test]
fn non_git_repo_exits_nonzero() {
    let output = run(&["--repo", "/nonexistent_rx_repo", "--hostname", "testhost"]);
    assert!(!output.status.success());
}

#[test]
fn dry_run_prints_commands_without_executing() {
    let output = run(&["--dry-run", "-y", "--hostname", "testhost"]);
    assert!(output.status.success());
    let stdout = stdout_of(&output);
    assert!(stdout.contains("$ "));
    assert!(stdout.contains("nixos-rebuild"));
}

#[test]
fn piped_stdin_decline_exits_zero() {
    let mut child = Command::new(RX)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn rx");
    if let Some(mut stdin) = child.stdin.take() {
        let _ = stdin.write_all(b"n\n");
        drop(stdin);
    }
    let status = child.wait().expect("wait rx");
    assert!(status.success());
}
