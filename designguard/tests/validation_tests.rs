//! Tests for specific validation rules

use designguard::prelude::*;
use designguard::analyzer::rules::RulesEngine;
use designguard::parse_schematic;
use std::path::PathBuf;

fn fixture_path(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join(name)
}

#[test]
fn test_decoupling_rule() {
    let schematic = parse_schematic(&fixture_path("missing_decap.kicad_sch"))
        .expect("Should parse schematic");

    let rules = RulesEngine::with_default_rules();
    let issues = rules.analyze(&schematic);

    // Should have decoupling-related issues
    let decap_issues: Vec<_> = issues
        .iter()
        .filter(|i| {
            i.rule_id.contains("decoupling")
                || i.message.to_lowercase().contains("decoupling")
        })
        .collect();

    assert!(!decap_issues.is_empty(), "Should detect decoupling issues");
}

#[test]
fn test_i2c_pullup_rule() {
    let schematic = parse_schematic(&fixture_path("missing_i2c_pullups.kicad_sch"))
        .expect("Should parse schematic");

    let rules = RulesEngine::with_default_rules();
    let issues = rules.analyze(&schematic);

    // Should have pull-up related issues
    let pullup_issues: Vec<_> = issues
        .iter()
        .filter(|i| {
            i.message.to_lowercase().contains("pull")
                && (i.message.contains("SDA") || i.message.contains("SCL"))
        })
        .collect();

    assert!(
        !pullup_issues.is_empty(),
        "Should detect missing I2C pull-ups"
    );
}

#[test]
fn test_severity_levels() {
    let options = ValidationOptions::default();

    let result = DesignGuardCore::validate_schematic(
        &fixture_path("missing_decap.kicad_sch"),
        options,
    )
    .expect("Should parse");

    // Test that severity enum works
    for issue in &result.issues {
        match issue.severity {
            Severity::Error | Severity::Warning | Severity::Suggestion | Severity::Info => {
                // Valid severity
            }
        }
    }
}

#[test]
fn test_issue_structure() {
    let options = ValidationOptions::default();

    let result = DesignGuardCore::validate_schematic(
        &fixture_path("missing_decap.kicad_sch"),
        options,
    )
    .expect("Should parse");

    for issue in &result.issues {
        // Every issue should have a message
        assert!(!issue.message.is_empty(), "Issue should have message");

        // Every issue should have a rule_id
        assert!(!issue.rule_id.is_empty(), "Issue should have rule_id");

        // Suggestion is optional but should be meaningful if present
        if let Some(ref suggestion) = issue.suggestion {
            assert!(!suggestion.is_empty(), "Suggestion should not be empty");
        }
    }
}
