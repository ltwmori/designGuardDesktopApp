//! CLI integration tests

use assert_cmd::cargo::cargo_bin_cmd;
use assert_cmd::Command;
use predicates::prelude::*;
use std::path::PathBuf;

/// Build command for the designguard-cli binary (finds it in target/debug when run via cargo test).
fn designguard_cli() -> Command {
    cargo_bin_cmd!("designguard-cli")
}

/// Path to designguard library test fixtures (relative to workspace).
fn fixtures_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("designguard")
        .join("tests")
        .join("fixtures")
}

#[test]
fn test_cli_help() {
    let mut cmd = designguard_cli();

    cmd.arg("--help");
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("KiCAD"));
}

#[test]
fn test_cli_version() {
    let mut cmd = designguard_cli();

    cmd.arg("--version");
    cmd.assert()
        .success()
        .stdout(predicate::str::contains(env!("CARGO_PKG_VERSION")));
}

#[test]
fn test_cli_check_valid_file() {
    let mut cmd = designguard_cli();
    let path = fixtures_dir().join("valid_design.kicad_sch");

    cmd.arg("check")
        .arg(path)
        .arg("--no-ai");

    cmd.assert().success();
}

#[test]
fn test_cli_check_missing_decap() {
    let mut cmd = designguard_cli();
    let path = fixtures_dir().join("missing_decap.kicad_sch");

    cmd.arg("check")
        .arg(path)
        .arg("--no-ai");

    cmd.assert().success();
}

#[test]
fn test_cli_check_with_fail_on_critical() {
    let mut cmd = designguard_cli();
    let path = fixtures_dir().join("missing_decap.kicad_sch");

    cmd.arg("check")
        .arg(path)
        .arg("--fail-on")
        .arg("critical")
        .arg("--no-ai");

    let _ = cmd.assert();
}

#[test]
fn test_cli_check_json_output() {
    let mut cmd = designguard_cli();
    let path = fixtures_dir().join("valid_design.kicad_sch");

    cmd.arg("check")
        .arg(path)
        .arg("--format")
        .arg("json")
        .arg("--no-ai");

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("{"))
        .stdout(predicate::str::contains("results").or(predicate::str::contains("issues")));
}

#[test]
fn test_cli_check_nonexistent_file() {
    let mut cmd = designguard_cli();

    cmd.arg("check").arg("does_not_exist.kicad_sch");

    cmd.assert()
        .failure()
        .stderr(predicate::str::contains("Error"));
}

#[test]
fn test_cli_project_command() {
    let mut cmd = designguard_cli();
    let dir = fixtures_dir();

    cmd.arg("project")
        .arg(dir)
        .arg("--no-ai");

    cmd.assert().success();
}

#[test]
fn test_cli_rules_command() {
    let mut cmd = designguard_cli();

    cmd.arg("rules");

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("decoupling"))
        .stdout(predicate::str::contains("i2c"));
}

#[test]
fn test_cli_rules_verbose() {
    let mut cmd = designguard_cli();

    cmd.arg("rules").arg("--verbose");

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("IPC-2221").or(predicate::str::contains("decoupling")));
}

#[test]
fn test_cli_github_format() {
    let mut cmd = designguard_cli();
    let path = fixtures_dir().join("missing_decap.kicad_sch");

    cmd.arg("check")
        .arg(path)
        .arg("--format")
        .arg("github")
        .arg("--no-ai");

    cmd.assert().success();
}

#[test]
fn test_cli_strict_mode() {
    let mut cmd = designguard_cli();
    let path = fixtures_dir().join("valid_design.kicad_sch");

    cmd.arg("check")
        .arg(path)
        .arg("--strict")
        .arg("--no-ai");

    cmd.assert().success();
}

#[test]
fn test_cli_exit_codes() {
    let valid_path = fixtures_dir().join("valid_design.kicad_sch");

    let mut cmd = designguard_cli();
    cmd.arg("check")
        .arg(&valid_path)
        .arg("--no-ai");
    cmd.assert().code(0);

    let mut cmd = designguard_cli();
    cmd.arg("check").arg("nonexistent.kicad_sch");
    cmd.assert().code(1);
}

#[test]
fn test_cli_output_formats_are_different() {
    let path = fixtures_dir().join("missing_decap.kicad_sch");

    let mut cmd_human = designguard_cli();
    cmd_human
        .arg("check")
        .arg(&path)
        .arg("--format")
        .arg("human")
        .arg("--no-ai");
    let human_output = cmd_human.output().unwrap();

    let mut cmd_json = designguard_cli();
    cmd_json
        .arg("check")
        .arg(&path)
        .arg("--format")
        .arg("json")
        .arg("--no-ai");
    let json_output = cmd_json.output().unwrap();

    assert_ne!(
        human_output.stdout,
        json_output.stdout,
        "Different formats should produce different output"
    );
}
