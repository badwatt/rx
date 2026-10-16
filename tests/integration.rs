use std::io::Write;
use std::process::{Command, Stdio};

/// Path to the compiled `rx` binary, provided by Cargo for integration tests.
const RX: &str = env!("CARGO_BIN_EXE_rx");

#[test]
fn help_flag_prints_rx_banner() {
    let output = Command::new(RX).arg("-h").output().expect("run rx");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("RX"));
}

#[test]
fn version_flag_prints_semver() {
    let output = Command::new(RX).arg("-v").output().expect("run rx");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("0.1.0"));
}

#[test]
fn unknown_flag_exits_nonzero() {
    let output = Command::new(RX).arg("-x").output().expect("run rx");
    assert!(!output.status.success());
    assert_eq!(output.status.code(), Some(2));
}

#[test]
fn piped_stdin_decline_does_not_panic() {
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
    let _ = child.wait();
}

#[test]
fn no_args_with_piped_stdin_runs_without_panic() {
    let _output = Command::new(RX)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .env("HOSTNAME", "testhost")
        .output()
        .expect("run rx");
}

#[test]
fn version_word_prints_version() {
    let output = Command::new(RX).arg("version").output().expect("run rx");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("RX version"));
}

#[test]
fn help_word_prints_help_and_exits_zero() {
    let output = Command::new(RX).arg("help").output().expect("run rx");
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("RX"));
}

#[test]
fn invalid_k_flag_exits_nonzero() {
    let output = Command::new(RX)
        .args(["-k", "abc"])
        .output()
        .expect("run rx");
    assert!(!output.status.success());
    assert_eq!(output.status.code(), Some(2));
}

#[test]
fn help_long_flag_prints_banner() {
    let output = Command::new(RX).arg("--help").output().expect("run rx");
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("RX"));
}

#[test]
fn version_long_flag_prints_semver() {
    let output = Command::new(RX).arg("--version").output().expect("run rx");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("0.1.0"));
}
