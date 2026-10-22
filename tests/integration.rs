use std::io::Write;
use std::process::{Command, Output, Stdio};

/// Path to the compiled `rx` binary, provided by Cargo for integration tests.
const RX: &str = env!("CARGO_BIN_EXE_rx");

/// Project version, sourced from the repository-root `VERSION` file.
fn version() -> &'static str {
    include_str!("../VERSION").trim()
}

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
    assert!(stdout_of(&run(&["-v"])).contains(version()));
}

#[test]
fn version_long_flag_prints_semver() {
    assert!(stdout_of(&run(&["--version"])).contains(version()));
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
    let output = run(&[
        "-y",
        "--repo",
        "/nonexistent_rx_repo",
        "--hostname",
        "testhost",
    ]);
    assert!(!output.status.success());
}

#[test]
fn dry_run_does_not_echo_the_capability_probe() {
    let output = run(&[
        "--dry-run",
        "-y",
        "--hostname",
        "testhost",
        "--repo",
        "/tmp",
    ]);
    assert!(output.status.success());
    assert!(!stdout_of(&output).contains("list-generations"));
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
fn closed_stdin_aborts_without_rebuilding() {
    let mut child = Command::new(RX)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn rx");
    drop(child.stdin.take());
    let output = child.wait_with_output().expect("wait rx");
    assert!(output.status.success());
    assert!(String::from_utf8_lossy(&output.stdout).contains("Aborted."));
}

#[test]
fn piped_stdin_decline_exits_zero() {
    let mut child = Command::new(RX)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn rx");
    let mut stdin = child.stdin.take().expect("child stdin");
    let _ = stdin.write_all(b"n\n");
    drop(stdin);
    let status = child.wait().expect("wait rx");
    assert!(status.success());
}

#[test]
fn piped_output_has_no_ansi() {
    let output = run(&[
        "--dry-run",
        "-y",
        "--hostname",
        "testhost",
        "--repo",
        "/tmp",
    ]);
    assert!(!stdout_of(&output).contains('\u{1b}'));
}

#[test]
fn parse_error_goes_to_stderr_only() {
    let output = run(&["-x"]);
    assert!(!output.status.success());
    assert_eq!(stdout_of(&output), "");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("Unknown flag"));
}

#[test]
fn invalid_jobs_exits_nonzero() {
    let output = run(&["-j", "abc"]);
    assert!(!output.status.success());
    assert_eq!(output.status.code(), Some(2));
}

#[test]
fn dry_run_summary_shows_dry_run() {
    let output = run(&[
        "--dry-run",
        "-y",
        "--hostname",
        "testhost",
        "--repo",
        "/tmp",
    ]);
    assert!(output.status.success());
    assert!(stdout_of(&output).contains("dry-run"));
}

#[test]
fn no_color_version_has_no_ansi() {
    let output = run(&["--no-color", "--version"]);
    assert!(output.status.success());
    assert!(!stdout_of(&output).contains('\u{1b}'));
}

#[test]
fn help_mentions_only() {
    let output = run(&["--help"]);
    assert!(stdout_of(&output).contains("--only"));
}

#[test]
fn help_mentions_no_color() {
    let output = run(&["--help"]);
    assert!(stdout_of(&output).contains("--no-color"));
}

#[test]
fn only_diff_skips_pull_and_rebuild() {
    let output = run(&[
        "--only",
        "diff",
        "--dry-run",
        "-y",
        "--hostname",
        "testhost",
        "--repo",
        "/tmp",
    ]);
    assert!(output.status.success());
    let stdout = stdout_of(&output);
    assert!(stdout.contains("Nix Diff"));
    assert!(!stdout.contains("Git Pull"));
    assert!(!stdout.contains("NixOS Rebuild"));
}

#[test]
fn unknown_only_step_exits_two() {
    let output = run(&["--only", "bogus"]);
    assert_eq!(output.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&output.stderr).contains("Unknown step"));
}
