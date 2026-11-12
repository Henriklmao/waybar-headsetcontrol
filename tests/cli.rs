//! Integration tests for the command line surface of the `headset-tui` binary.
//!
//! Only argument handling is exercised here: `--help` and the error paths.
//! Commands that need a headset or a terminal (`--waybar-status`,
//! `--toggle-sidetone`, `--config`, the TUI) are covered by unit tests in
//! `src/cli.rs` and never run as a real process, so these tests need neither a
//! headset nor `headsetcontrol`.

use std::process::{Command, Output};

const BIN: &str = env!("CARGO_BIN_EXE_headset-tui");

fn run(args: &[&str]) -> Output {
    Command::new(BIN)
        .args(args)
        .output()
        .unwrap_or_else(|error| panic!("failed to run {BIN}: {error}"))
}

fn stdout(output: &Output) -> String {
    String::from_utf8(output.stdout.clone()).expect("stdout is not valid UTF-8")
}

fn stderr(output: &Output) -> String {
    String::from_utf8(output.stderr.clone()).expect("stderr is not valid UTF-8")
}

fn exit_code(output: &Output) -> i32 {
    output
        .status
        .code()
        .expect("process was killed by a signal")
}

// --- --help ---------------------------------------------------------------

#[test]
fn help_exits_successfully() {
    let output = run(&["--help"]);
    assert_eq!(exit_code(&output), 0);
    assert!(
        stderr(&output).is_empty(),
        "unexpected stderr: {}",
        stderr(&output)
    );
}

#[test]
fn help_prints_the_usage_header() {
    let output = run(&["--help"]);
    let text = stdout(&output);
    assert!(text.starts_with("headset-tui - HeadsetControl integration for Waybar"));
    assert!(text.contains("USAGE:"));
    assert!(text.contains("headset-tui [OPTIONS]"));
}

#[test]
fn help_lists_every_command_flag() {
    let text = stdout(&run(&["--help"]));
    for flag in [
        "--waybar-status",
        "--toggle-sidetone",
        "--config",
        "-h, --help",
    ] {
        assert!(text.contains(flag), "help output does not mention {flag}");
    }
}

#[test]
fn help_documents_the_test_modifier() {
    let text = stdout(&run(&["--help"]));
    assert!(text.contains("--test"));
    assert!(text.contains("--test-device"));
    assert!(text.contains("simulated"));
}

#[test]
fn help_does_not_use_the_old_binary_name() {
    assert!(!stdout(&run(&["--help"])).contains("wb-headset"));
}

#[test]
fn short_help_exits_successfully() {
    let output = run(&["-h"]);
    assert_eq!(exit_code(&output), 0);
    assert!(stderr(&output).is_empty());
}

#[test]
fn short_help_matches_long_help() {
    let long = run(&["--help"]);
    let short = run(&["-h"]);
    assert_eq!(long.stdout, short.stdout);
}

#[test]
fn help_works_together_with_the_test_modifier() {
    let output = run(&["--test", "--help"]);
    assert_eq!(exit_code(&output), 0);
    assert!(stdout(&output).contains("USAGE:"));
}

// --- Unknown arguments ----------------------------------------------------

#[test]
fn unknown_argument_exits_with_code_two() {
    assert_eq!(exit_code(&run(&["--bogus"])), 2);
}

#[test]
fn unknown_argument_is_reported_on_stderr() {
    let output = run(&["--bogus"]);
    assert!(stderr(&output).contains("unknown argument"));
    assert!(stderr(&output).contains("--bogus"));
}

#[test]
fn unknown_argument_prints_nothing_on_stdout() {
    assert!(stdout(&run(&["--bogus"])).is_empty());
}

#[test]
fn unknown_argument_suggests_the_help_command() {
    let text = stderr(&run(&["--bogus"]));
    assert!(text.contains("Run 'headset-tui --help' for usage."));
}

#[test]
fn unknown_short_argument_exits_with_code_two() {
    let output = run(&["-z"]);
    assert_eq!(exit_code(&output), 2);
    assert!(stderr(&output).contains("unknown argument '-z'"));
}

#[test]
fn unknown_positional_argument_exits_with_code_two() {
    let output = run(&["status"]);
    assert_eq!(exit_code(&output), 2);
    assert!(stderr(&output).contains("unknown argument 'status'"));
}

#[test]
fn test_device_is_not_a_program_argument() {
    let output = run(&["--test-device"]);
    assert_eq!(exit_code(&output), 2);
    assert!(stderr(&output).contains("unknown argument '--test-device'"));
}

// --- Conflicting commands -------------------------------------------------

#[test]
fn conflicting_commands_exit_with_code_two() {
    assert_eq!(exit_code(&run(&["--config", "--waybar-status"])), 2);
}

#[test]
fn conflicting_commands_are_reported_on_stderr() {
    let text = stderr(&run(&["--config", "--waybar-status"]));
    assert!(text.contains("conflicting commands"));
    assert!(text.contains("--config"));
    assert!(text.contains("--waybar-status"));
}

#[test]
fn conflicting_commands_print_nothing_on_stdout() {
    assert!(stdout(&run(&["--config", "--waybar-status"])).is_empty());
}

#[test]
fn conflicting_commands_suggest_the_help_command() {
    let text = stderr(&run(&["--config", "--waybar-status"]));
    assert!(text.contains("Run 'headset-tui --help' for usage."));
}

#[test]
fn conflicting_commands_do_not_mention_the_test_modifier() {
    let text = stderr(&run(&["--test", "--config", "--toggle-sidetone"]));
    assert!(text.contains("conflicting commands: --config, --toggle-sidetone"));
}

#[test]
fn help_and_config_conflict() {
    assert_eq!(exit_code(&run(&["--help", "--config"])), 2);
    assert!(stderr(&run(&["--help", "--config"])).contains("conflicting commands"));
}
