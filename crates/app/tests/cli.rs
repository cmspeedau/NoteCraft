//! End-to-end checks that drive the built binary the way a shell would. These
//! exercise the real process boundary, not just the library.
//!
//! A bare launch now opens a window and runs the event loop, so it cannot be
//! exercised in a host-side test; the window itself is covered by the shell's
//! own tests. These tests cover the paths that return before the window opens.

use std::process::Command;

/// Path to the binary under test, provided by Cargo for integration tests.
const BIN: &str = env!("CARGO_BIN_EXE_notepad");

#[test]
fn version_flag_prints_name_and_version() {
    let out = Command::new(BIN).arg("--version").output().unwrap();
    assert!(out.status.success(), "exit: {:?}", out.status);
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("nc-app"), "unexpected stdout: {stdout:?}");
}

#[test]
fn help_flag_prints_usage() {
    let out = Command::new(BIN).arg("--help").output().unwrap();
    assert!(out.status.success());
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("Usage:"));
}

#[test]
fn unknown_option_is_a_usage_error() {
    let out = Command::new(BIN)
        .arg("--definitely-not-an-option")
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(2));
}
