//! Entry point for `cargo xtask <task>`.

use std::process::ExitCode;

use xtask::{check_boundaries, find_workspace_root};

fn main() -> ExitCode {
    let mut args = std::env::args().skip(1);
    match args.next().as_deref() {
        Some("check-boundaries") => run_check_boundaries(),
        None | Some("help") | Some("--help") | Some("-h") => {
            print_help();
            ExitCode::SUCCESS
        }
        Some(other) => {
            eprintln!("xtask: unknown task `{other}`\n");
            print_help();
            ExitCode::from(2)
        }
    }
}

fn run_check_boundaries() -> ExitCode {
    let start = std::env::current_dir().unwrap_or_else(|_| ".".into());
    let root = match find_workspace_root(&start) {
        Ok(root) => root,
        Err(err) => {
            eprintln!("xtask: {err}");
            return ExitCode::FAILURE;
        }
    };

    match check_boundaries(&root) {
        Ok(violations) if violations.is_empty() => {
            println!("xtask: crate boundary check passed");
            ExitCode::SUCCESS
        }
        Ok(violations) => {
            for violation in &violations {
                eprintln!("xtask: error: {violation}");
            }
            ExitCode::FAILURE
        }
        Err(err) => {
            eprintln!("xtask: {err}");
            ExitCode::FAILURE
        }
    }
}

fn print_help() {
    println!(
        "xtask — NoteCraft repository tasks\n\
         \n\
         USAGE:\n    cargo xtask <task>\n\
         \n\
         TASKS:\n\
         \x20   check-boundaries    Verify crate dependency direction (CI gate)\n\
         \x20   help                Show this message"
    );
}
