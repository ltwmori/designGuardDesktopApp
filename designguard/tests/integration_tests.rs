//! Integration tests for DesignGuard library

use designguard::prelude::*;
use std::path::PathBuf;

fn fixture_path(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join(name)
}

#[test]
fn test_validate_valid_design() {
    let options = ValidationOptions {
        enable_ai: false,
        offline_mode: true,
        strict_mode: false,
        rules: vec![],
    };

    let result = DesignGuardCore::validate_schematic(
        &fixture_path("valid_design.kicad_sch"),
        options,
    );

    assert!(result.is_ok(), "Valid design should parse successfully");

    let result = result.unwrap();
    assert_eq!(
        result.stats.critical, 0,
        "Valid design should have no critical issues"
    );
    let high_issues: Vec<_> = result
        .issues
        .iter()
        .filter(|i| matches!(i.severity, Severity::Warning))
        .collect();
    assert_eq!(
        result.stats.high, 0,
        "Valid design should have no high issues. High (Warning) issues: {:?}",
        high_issues
            .iter()
            .map(|i| &i.message)
            .collect::<Vec<_>>()
    );
}

#[test]
fn test_validate_missing_decap() {
    let options = ValidationOptions {
        enable_ai: false,
        offline_mode: true,
        strict_mode: false,
        rules: vec![],
    };

    let result = DesignGuardCore::validate_schematic(
        &fixture_path("missing_decap.kicad_sch"),
        options,
    )
    .expect("Should parse successfully");

    // Should detect missing decoupling capacitor
    assert!(
        result.stats.critical > 0 || result.stats.high > 0,
        "Should flag missing decoupling capacitor"
    );

    // Check that at least one issue mentions decoupling
    let has_decap_issue = result.issues.iter().any(|issue| {
        issue.message.to_lowercase().contains("decoupling")
            || issue.message.to_lowercase().contains("bypass")
            || issue.message.to_lowercase().contains("capacitor")
    });

    assert!(
        has_decap_issue,
        "Should have issue about decoupling capacitor"
    );
}

#[test]
fn test_validate_missing_i2c_pullups() {
    let options = ValidationOptions {
        enable_ai: false,
        offline_mode: true,
        strict_mode: false,
        rules: vec![],
    };

    let result = DesignGuardCore::validate_schematic(
        &fixture_path("missing_i2c_pullups.kicad_sch"),
        options,
    )
    .expect("Should parse successfully");

    // Should detect missing pull-ups
    let has_pullup_issue = result.issues.iter().any(|issue| {
        issue.message.to_lowercase().contains("pull")
            && (issue.message.contains("SDA") || issue.message.contains("SCL"))
    });

    assert!(has_pullup_issue, "Should flag missing I2C pull-ups");
}

#[test]
fn test_validate_nonexistent_file() {
    let options = ValidationOptions::default();

    let result = DesignGuardCore::validate_schematic(
        &PathBuf::from("does_not_exist.kicad_sch"),
        options,
    );

    assert!(result.is_err(), "Should return error for nonexistent file");
}

#[test]
fn test_validation_options() {
    // Test with AI disabled
    let options = ValidationOptions {
        enable_ai: false,
        offline_mode: true,
        strict_mode: false,
        rules: vec![],
    };

    let result = DesignGuardCore::validate_schematic(
        &fixture_path("valid_design.kicad_sch"),
        options,
    );

    assert!(result.is_ok());

    // Test with strict mode
    let strict_options = ValidationOptions {
        enable_ai: false,
        offline_mode: true,
        strict_mode: true,
        rules: vec![],
    };

    let strict_result = DesignGuardCore::validate_schematic(
        &fixture_path("valid_design.kicad_sch"),
        strict_options,
    );

    assert!(strict_result.is_ok());
}

#[test]
fn test_validation_result_stats() {
    let options = ValidationOptions::default();

    let result = DesignGuardCore::validate_schematic(
        &fixture_path("missing_decap.kicad_sch"),
        options,
    )
    .expect("Should parse");

    // Test helper methods
    let total = result.total_issues();
    let expected_total = result.stats.critical
        + result.stats.high
        + result.stats.medium
        + result.stats.low
        + result.stats.info;

    assert_eq!(total, expected_total, "total_issues() should match sum");

    if result.stats.critical > 0 {
        assert!(result.has_critical(), "has_critical() should return true");
        assert!(
            result.has_high_or_critical(),
            "has_high_or_critical() should return true"
        );
    }
}

#[test]
fn test_validate_project_directory() {
    let options = ValidationOptions {
        enable_ai: false,
        offline_mode: true,
        strict_mode: false,
        rules: vec![],
    };

    let fixtures_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures");

    let results = DesignGuardCore::validate_project(&fixtures_dir, options)
        .expect("Should validate project directory");

    // Should find multiple files
    assert!(!results.is_empty(), "Should find at least one KiCad file");

    // Each result should have a file path
    for result in &results {
        assert!(result.file.exists(), "Result file should exist");
    }
}

#[test]
fn test_discover_kicad_files() {
    let fixtures_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures");

    let files = designguard::discover_kicad_files(&fixtures_dir)
        .expect("Should discover files");

    assert!(!files.is_empty(), "Should find KiCad files");

    // All files should be .kicad_sch or .kicad_pcb
    for file in &files {
        let ext = file.extension().and_then(|s| s.to_str()).unwrap();
        assert!(
            ext == "kicad_sch" || ext == "kicad_pcb" || ext == "sch" || ext == "brd",
            "File should be KiCad format: {:?}",
            file
        );
    }
}
