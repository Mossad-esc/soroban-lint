//! End-to-end CLI tests.

use assert_cmd::Command;
use predicates::prelude::*;

fn bin() -> Command {
    Command::cargo_bin("soroban-lint").expect("binary should build")
}

#[test]
fn help_lists_commands() {
    bin()
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("check"))
        .stdout(predicate::str::contains("rules"))
        .stdout(predicate::str::contains("explain"))
        .stdout(predicate::str::contains("init"));
}

#[test]
fn rules_command_lists_catalog() {
    bin()
        .arg("rules")
        .assert()
        .success()
        .stdout(predicate::str::contains("SOR001"))
        .stdout(predicate::str::contains("missing-require-auth"))
        .stdout(predicate::str::contains("planned"));
}

#[test]
fn explain_unknown_id_exits_two() {
    bin()
        .args(["explain", "SOR999"])
        .assert()
        .code(2)
        .stderr(predicate::str::contains("unknown rule"));
}

#[test]
fn unknown_select_id_exits_two() {
    bin()
        .args(["check", "--select", "SOR999"])
        .assert()
        .code(2)
        .stderr(predicate::str::contains("unknown rule id"));
}

#[test]
fn invalid_format_exits_two() {
    bin().args(["check", "--format", "xml"]).assert().code(2);
}

#[test]
fn check_on_empty_directory_exits_zero() {
    let dir = tempfile::tempdir().expect("temp dir");
    bin()
        .arg("check")
        .arg(dir.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("no rules implemented yet"));
}
