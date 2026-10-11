//! End-to-end checks that drive the built `notepad` binary the way a shell
//! would. These exercise the real process boundary, not just the library.

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

/// A bare launch must return cleanly on a host without a native window.
/// On Windows this opens a real window and is covered by the UI suite instead.
#[cfg(not(windows))]
#[test]
fn headless_launch_exits_cleanly() {
    let out = Command::new(BIN).output().unwrap();
    assert!(out.status.success(), "exit: {:?}", out.status);
}
