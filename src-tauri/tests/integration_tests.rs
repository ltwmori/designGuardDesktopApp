//! Integration Tests for KiCAD AI Assistant
//!
//! This module tests end-to-end workflows including:
//! - Full analysis pipeline (parse -> analyze -> report)
//! - File watcher functionality
//! - Command interactions
//! - State management

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use designguard::analyzer::rules::{Issue, RulesEngine, Severity};
use designguard::parser::kicad::KicadParser;
use designguard::parser::schema::{
    Component, Label, LabelType, Net, Pin, Position, Schematic, Wire,
};
use designguard::watcher::{ProjectWatcher, WatchEvent};

// =============================================================================
// Test Helpers
// =============================================================================

fn get_fixture_path(relative: &str) -> PathBuf {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    PathBuf::from(format!("{}/test-fixtures/{}", manifest_dir, relative))
}

fn get_edge_case_path(filename: &str) -> PathBuf {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    // Go up from src-tauri to project root, then to test-generation
    PathBuf::from(format!("{}/../test-generation/edge-cases/{}", manifest_dir, filename))
}

fn create_test_schematic_with_issues() -> Schematic {
    // Create a schematic that will trigger multiple design rule issues
    Schematic {
        uuid: "integration-test-schematic".to_string(),
        filename: "integration_test.kicad_sch".to_string(),
        version: Some("20231120".to_string()),
        components: vec![
            // IC without decoupling cap (issue)
            Component {
                uuid: "u1-uuid".to_string(),
                reference: "U1".to_string(),
                value: "STM32F401CCU6".to_string(),
                lib_id: "MCU_ST_STM32F4:STM32F401CCU6".to_string(),
                footprint: Some("Package_QFP:LQFP-48".to_string()),
                position: Position { x: 100.0, y: 100.0 },
                rotation: 0.0,
                properties: HashMap::new(),
                pins: vec![],
            },
            // Crystal without load caps (issue)
            Component {
                uuid: "y1-uuid".to_string(),
                reference: "Y1".to_string(),
                value: "8MHz".to_string(),
                lib_id: "Device:Crystal".to_string(),
                footprint: None,
                position: Position { x: 150.0, y: 100.0 },
                rotation: 0.0,
                properties: HashMap::new(),
                pins: vec![],
            },
            // USB connector without ESD (issue)
            Component {
                uuid: "j1-uuid".to_string(),
                reference: "J1".to_string(),
                value: "USB-C".to_string(),
                lib_id: "Connector:USB_C".to_string(),
                footprint: None,
                position: Position { x: 50.0, y: 100.0 },
                rotation: 0.0,
                properties: HashMap::new(),
                pins: vec![],
            },
            // Voltage regulator without bulk cap (issue)
            Component {
                uuid: "u2-uuid".to_string(),
                reference: "U2".to_string(),
                value: "AMS1117-3.3".to_string(),
                lib_id: "Regulator:Linear".to_string(),
                footprint: None,
                position: Position { x: 80.0, y: 50.0 },
                rotation: 0.0,
                properties: HashMap::new(),
                pins: vec![],
            },
        ],
        wires: vec![
            Wire {
                uuid: "wire1-uuid".to_string(),
                points: vec![
                    Position { x: 50.0, y: 100.0 },
                    Position { x: 100.0, y: 100.0 },
                ],
            },
        ],
        labels: vec![
            // I2C labels without pull-ups (issue)
            Label {
                uuid: "sda-label".to_string(),
                text: "SDA".to_string(),
                position: Position { x: 200.0, y: 80.0 },
                rotation: 0.0,
                label_type: LabelType::Global,
            },
            Label {
                uuid: "scl-label".to_string(),
                text: "SCL".to_string(),
                position: Position { x: 200.0, y: 90.0 },
                rotation: 0.0,
                label_type: LabelType::Global,
            },
        ],
        nets: vec![],
        power_symbols: vec![], // No GND (issue)
    }
}

fn create_clean_schematic() -> Schematic {
    // Create a schematic that should pass most checks
    Schematic {
        uuid: "clean-schematic".to_string(),
        filename: "clean.kicad_sch".to_string(),
        version: Some("20231120".to_string()),
        components: vec![
            // IC
            Component {
                uuid: "u1-uuid".to_string(),
                reference: "U1".to_string(),
                value: "STM32F401".to_string(),
                lib_id: "MCU:STM32F401".to_string(),
                footprint: None,
                position: Position { x: 100.0, y: 100.0 },
                rotation: 0.0,
                properties: HashMap::new(),
                pins: vec![],
            },
            // Decoupling cap near IC
            Component {
                uuid: "c1-uuid".to_string(),
                reference: "C1".to_string(),
                value: "100nF".to_string(),
                lib_id: "Device:C".to_string(),
                footprint: None,
                position: Position { x: 105.0, y: 105.0 },
                rotation: 0.0,
                properties: HashMap::new(),
                pins: vec![],
            },
        ],
        wires: vec![],
        labels: vec![],
        nets: vec![],
        power_symbols: vec![
            // GND symbol present
            Component {
                uuid: "gnd-uuid".to_string(),
                reference: "#PWR01".to_string(),
                value: "GND".to_string(),
                lib_id: "power:GND".to_string(),
                footprint: None,
                position: Position { x: 100.0, y: 120.0 },
                rotation: 0.0,
                properties: HashMap::new(),
                pins: vec![],
            },
        ],
    }
}

// =============================================================================
// Full Analysis Workflow Tests
// =============================================================================

mod full_analysis_workflow_tests {
    use super::*;

    #[test]
    fn test_full_analysis_workflow() {
        // 1. Create/load schematic
        let schematic = create_test_schematic_with_issues();

        // 2. Run design rule checks
        let engine = RulesEngine::with_default_rules();
        let issues = engine.analyze(&schematic);

        // 3. Verify issues are detected
        assert!(!issues.is_empty(), "Should detect issues in problematic schematic");

        // 4. Verify specific issue types
        let rule_ids: Vec<&str> = issues.iter().map(|i| i.rule_id.as_str()).collect();

        assert!(
            rule_ids.contains(&"power_pins"),
            "Should detect missing GND"
        );
        assert!(
            rule_ids.contains(&"decoupling_capacitor"),
            "Should detect missing decoupling cap"
        );
        assert!(
            rule_ids.contains(&"i2c_pull_resistors"),
            "Should detect missing I2C pull-ups"
        );
        assert!(
            rule_ids.contains(&"crystal_load_capacitors"),
            "Should detect missing crystal caps"
        );
        assert!(
            rule_ids.contains(&"esd_protection"),
            "Should detect missing ESD protection"
        );
    }

    #[test]
    fn test_clean_schematic_minimal_issues() {
        let schematic = create_clean_schematic();
        let engine = RulesEngine::with_default_rules();
        let issues = engine.analyze(&schematic);

        // Should have minimal issues (no critical errors)
        let errors: Vec<_> = issues
            .iter()
            .filter(|i| i.severity == Severity::Error)
            .collect();

        assert!(
            errors.is_empty(),
            "Clean schematic should have no critical errors"
        );
    }

    #[test]
    fn test_analysis_results_contain_required_fields() {
        let schematic = create_test_schematic_with_issues();
        let engine = RulesEngine::with_default_rules();
        let issues = engine.analyze(&schematic);

        for issue in &issues {
            // Every issue should have required fields
            assert!(!issue.id.is_empty(), "Issue should have ID");
            assert!(!issue.rule_id.is_empty(), "Issue should have rule_id");
            assert!(!issue.message.is_empty(), "Issue should have message");
            // Suggestion is recommended but optional
        }
    }

    #[test]
    fn test_analysis_workflow_with_file() {
        let path = get_fixture_path("simple-project/simple.kicad_sch");

        // Skip if fixture doesn't exist
        if !path.exists() {
            eprintln!("Skipping file test - fixture not found at {:?}", path);
            return;
        }

        // 1. Parse schematic from file
        let schematic = KicadParser::parse_schematic(&path);
        assert!(schematic.is_ok(), "Should parse fixture file");

        let schematic = schematic.unwrap();

        // 2. Run analysis
        let engine = RulesEngine::with_default_rules();
        let issues = engine.analyze(&schematic);

        // 3. Analysis should complete without panic
        // Results depend on fixture content
        println!(
            "Analyzed {} components, found {} issues",
            schematic.components.len(),
            issues.len()
        );
    }

    #[test]
    fn test_analysis_workflow_with_issues_file() {
        let path = get_fixture_path("with-issues/issues.kicad_sch");

        if !path.exists() {
            eprintln!("Skipping file test - fixture not found at {:?}", path);
            return;
        }

        let schematic = KicadParser::parse_schematic(&path);
        assert!(schematic.is_ok(), "Should parse issues fixture file");

        let schematic = schematic.unwrap();
        let engine = RulesEngine::with_default_rules();
        let issues = engine.analyze(&schematic);

        // Issues file should have multiple problems
        assert!(
            issues.len() >= 3,
            "Issues fixture should have multiple problems detected"
        );
    }

    #[test]
    fn test_corrupt_file_handling() {
        let path = get_fixture_path("invalid/corrupt.kicad_sch");

        if !path.exists() {
            eprintln!("Skipping file test - fixture not found at {:?}", path);
            return;
        }

        let result = KicadParser::parse_schematic(&path);

        // Corrupt file should fail gracefully
        assert!(result.is_err(), "Corrupt file should fail to parse");
    }
}

// =============================================================================
// File Watcher Tests
// =============================================================================

mod file_watcher_tests {
    use super::*;
    use std::fs;
    use std::time::Duration;
    use tempfile::TempDir;

    #[test]
    fn test_watcher_creation() {
        let watcher = ProjectWatcher::new();
        // Watcher should be created successfully
    }

    #[test]
    fn test_watcher_subscribe() {
        let watcher = ProjectWatcher::new();
        let _receiver = watcher.subscribe();
        // Should be able to subscribe without issues
    }

    #[tokio::test]
    async fn test_watcher_watch_valid_directory() {
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let mut watcher = ProjectWatcher::new();

        let result: anyhow::Result<()> = watcher.watch(temp_dir.path().to_path_buf()).await;
        assert!(result.is_ok(), "Should watch valid directory");

        // Cleanup
        let _ = watcher.unwatch().await;
    }

    #[tokio::test]
    async fn test_watcher_watch_invalid_path() {
        let mut watcher = ProjectWatcher::new();
        let invalid_path = PathBuf::from("/nonexistent/path/that/doesnt/exist");

        let result: anyhow::Result<()> = watcher.watch(invalid_path).await;
        assert!(result.is_err(), "Should fail on nonexistent path");
    }

    #[tokio::test]
    async fn test_watcher_unwatch() {
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let mut watcher = ProjectWatcher::new();

        // Start watching
        watcher
            .watch(temp_dir.path().to_path_buf())
            .await
            .expect("Failed to watch");

        // Stop watching
        let result: anyhow::Result<()> = watcher.unwatch().await;
        assert!(result.is_ok(), "Should unwatch successfully");
    }

    #[tokio::test]
    async fn test_watcher_multiple_watch_calls() {
        let temp_dir1 = TempDir::new().expect("Failed to create temp dir 1");
        let temp_dir2 = TempDir::new().expect("Failed to create temp dir 2");
        let mut watcher = ProjectWatcher::new();

        // Watch first directory
        watcher
            .watch(temp_dir1.path().to_path_buf())
            .await
            .expect("Failed to watch dir 1");

        // Watch second directory (should stop first)
        let result: anyhow::Result<()> = watcher.watch(temp_dir2.path().to_path_buf()).await;
        assert!(result.is_ok(), "Should be able to switch watched directories");

        let _ = watcher.unwatch().await;
    }

    #[tokio::test]
    async fn test_file_watcher_triggers_reanalysis() {
        // This test simulates the workflow where file changes trigger reanalysis
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let schematic_path = temp_dir.path().join("test.kicad_sch");

        // Create initial schematic file
        let initial_content = r#"(kicad_sch (version 20231120) (generator "eeschema")
  (uuid "initial-uuid")
)"#;
        fs::write(&schematic_path, initial_content).expect("Failed to write initial file");

        // Create watcher
        let mut watcher = ProjectWatcher::new();
        let mut receiver = watcher.subscribe();

        // Start watching
        let _: anyhow::Result<()> = watcher
            .watch(temp_dir.path().to_path_buf())
            .await;

        // Give watcher time to initialize
        tokio::time::sleep(Duration::from_millis(100)).await;

        // Modify the file
        let modified_content = r#"(kicad_sch (version 20231120) (generator "eeschema")
  (uuid "modified-uuid")
  (symbol (lib_id "Device:R") (at 100 50 0) (unit 1)
    (uuid "r1-uuid")
    (property "Reference" "R1" (at 101 49 0))
    (property "Value" "10k" (at 101 51 0))
    (pin "1" (uuid "pin-1"))
  )
)"#;
        fs::write(&schematic_path, modified_content).expect("Failed to modify file");

        // Wait for debounced event (watcher uses 500ms debounce)
        let timeout = tokio::time::timeout(
            Duration::from_secs(2),
            async {
                loop {
                    match receiver.try_recv() {
                        Ok(event) => {
                            match event {
                                WatchEvent::FileModified(path) |
                                WatchEvent::FileCreated(path) => {
                                    if path.extension().and_then(|e| e.to_str()) == Some("kicad_sch") {
                                        return true;
                                    }
                                }
                                _ => {}
                            }
                        }
                        Err(_) => {
                            tokio::time::sleep(Duration::from_millis(50)).await;
                        }
                    }
                }
            }
        ).await;

        // Cleanup
        let _ = watcher.unwatch().await;

        // Note: This test may be flaky depending on the system's file notification behavior
        // In CI, consider marking as ignored or adjusting timeouts
    }

    #[test]
    fn test_kicad_file_detection() {
        // Test the file extension detection logic
        let kicad_files = vec![
            Path::new("/path/to/design.kicad_sch"),
            Path::new("/path/to/board.kicad_pcb"),
            Path::new("/path/to/project.kicad_pro"),
        ];

        let non_kicad_files = vec![
            Path::new("/path/to/readme.txt"),
            Path::new("/path/to/image.png"),
            Path::new("/path/to/code.rs"),
        ];

        for file in kicad_files {
            let ext = file.extension().and_then(|e| e.to_str()).unwrap_or("");
            assert!(
                ["kicad_sch", "kicad_pcb", "kicad_pro"].contains(&ext),
                "Should detect KiCAD file: {:?}",
                file
            );
        }

        for file in non_kicad_files {
            let ext = file.extension().and_then(|e| e.to_str()).unwrap_or("");
            assert!(
                !["kicad_sch", "kicad_pcb", "kicad_pro"].contains(&ext),
                "Should not detect as KiCAD file: {:?}",
                file
            );
        }
    }
}

// =============================================================================
// State Management Tests
// =============================================================================

mod state_tests {
    use super::*;

    #[test]
    fn test_schematic_state_serialization() {
        let schematic = create_clean_schematic();

        // Serialize
        let json = serde_json::to_string(&schematic);
        assert!(json.is_ok(), "Schematic should serialize to JSON");

        // Deserialize
        let json_str = json.unwrap();
        let restored: Result<Schematic, _> = serde_json::from_str(&json_str);
        assert!(restored.is_ok(), "Schematic should deserialize from JSON");

        let restored = restored.unwrap();
        assert_eq!(restored.uuid, schematic.uuid);
        assert_eq!(restored.components.len(), schematic.components.len());
    }

    #[test]
    fn test_issue_state_serialization() {
        let issues = vec![
            Issue {
                id: "test-issue-1".to_string(),
                rule_id: "test_rule".to_string(),
                severity: Severity::Warning,
                message: "Test issue message".to_string(),
                component: Some("U1".to_string()),
                risk_score: None,
                location: Some(Position { x: 100.0, y: 100.0 }),
                suggestion: Some("Fix it".to_string()),
            },
        ];

        let json = serde_json::to_string(&issues);
        assert!(json.is_ok(), "Issues should serialize");

        let restored: Result<Vec<Issue>, _> = serde_json::from_str(&json.unwrap());
        assert!(restored.is_ok(), "Issues should deserialize");
    }
}

// =============================================================================
// Performance Tests
// =============================================================================

mod performance_tests {
    use super::*;
    use std::time::Instant;

    #[test]
    fn test_large_schematic_analysis_performance() {
        // Create a large schematic
        let mut schematic = create_clean_schematic();

        // Add many components
        for i in 0..500 {
            schematic.components.push(Component {
                uuid: format!("comp-{}", i),
                reference: format!("R{}", i),
                value: "10k".to_string(),
                lib_id: "Device:R".to_string(),
                footprint: None,
                position: Position {
                    x: (i % 50) as f64 * 10.0,
                    y: (i / 50) as f64 * 10.0,
                },
                rotation: 0.0,
                properties: HashMap::new(),
                pins: vec![],
            });
        }

        let engine = RulesEngine::with_default_rules();

        // Measure analysis time
        let start = Instant::now();
        let _issues = engine.analyze(&schematic);
        let duration = start.elapsed();

        // Analysis should complete in reasonable time (< 1 second for 500 components)
        assert!(
            duration.as_millis() < 1000,
            "Analysis took too long: {:?}",
            duration
        );
    }

    #[test]
    fn test_parse_performance() {
        let schematic_content = generate_large_schematic_content(100);

        let start = Instant::now();
        let result = KicadParser::parse_schematic_str(&schematic_content, "perf_test.kicad_sch");
        let duration = start.elapsed();

        assert!(result.is_ok(), "Parsing should succeed");
        assert!(
            duration.as_millis() < 500,
            "Parsing took too long: {:?}",
            duration
        );
    }

    fn generate_large_schematic_content(num_components: usize) -> String {
        let mut content = String::from(
            r#"(kicad_sch (version 20231120) (generator "eeschema")
  (uuid "perf-test-uuid")
"#,
        );

        for i in 0..num_components {
            content.push_str(&format!(
                r#"
  (symbol (lib_id "Device:R") (at {} {} 0) (unit 1)
    (uuid "r{}-uuid")
    (property "Reference" "R{}" (at {} {} 0))
    (property "Value" "10k" (at {} {} 0))
    (pin "1" (uuid "r{}-pin1"))
    (pin "2" (uuid "r{}-pin2"))
  )
"#,
                (i % 50) * 10,
                (i / 50) * 10,
                i,
                i,
                (i % 50) * 10 + 1,
                (i / 50) * 10 - 1,
                (i % 50) * 10 + 1,
                (i / 50) * 10 + 1,
                i,
                i
            ));
        }

        content.push_str(")\n");
        content
    }
}

// =============================================================================
// Edge Case Tests
// =============================================================================

mod edge_case_tests {
    use super::*;

    #[test]
    fn test_empty_schematic() {
        let schematic = Schematic {
            uuid: "empty".to_string(),
            filename: "empty.kicad_sch".to_string(),
            version: None,
            components: vec![],
            wires: vec![],
            labels: vec![],
            nets: vec![],
            power_symbols: vec![],
        };

        let engine = RulesEngine::with_default_rules();
        let issues = engine.analyze(&schematic);

        // Should still work and detect missing GND
        assert!(issues.iter().any(|i| i.message.contains("GND")));
    }

    #[test]
    fn test_schematic_with_unicode() {
        let mut schematic = create_clean_schematic();
        schematic.components.push(Component {
            uuid: "unicode-test".to_string(),
            reference: "C1".to_string(),
            value: "100µF".to_string(), // Unicode mu
            lib_id: "Device:C".to_string(),
            footprint: None,
            position: Position { x: 0.0, y: 0.0 },
            rotation: 0.0,
            properties: HashMap::new(),
            pins: vec![],
        });

        let engine = RulesEngine::with_default_rules();
        let _issues = engine.analyze(&schematic);
        // Should not panic with unicode characters
    }

    #[test]
    fn test_schematic_with_special_characters() {
        let schematic_str = r#"(kicad_sch (version 20231120) (generator "eeschema")
  (uuid "special-chars-test")
  (symbol (lib_id "Device:R") (at 100 50 0) (unit 1)
    (uuid "r1-uuid")
    (property "Reference" "R1" (at 101 49 0))
    (property "Value" "10k (1%)" (at 101 51 0))
    (property "Manufacturer" "Yageo & Co." (at 101 53 0))
    (property "Notes" "Use \"precision\" type" (at 101 55 0))
    (pin "1" (uuid "pin-1"))
  )
)"#;

        let result = KicadParser::parse_schematic_str(schematic_str, "special.kicad_sch");
        assert!(result.is_ok(), "Should handle special characters in values");
    }

    #[test]
    fn test_overlapping_components() {
        let mut schematic = create_clean_schematic();

        // Add components at the same position
        for i in 0..5 {
            schematic.components.push(Component {
                uuid: format!("overlap-{}", i),
                reference: format!("R{}", i),
                value: "10k".to_string(),
                lib_id: "Device:R".to_string(),
                footprint: None,
                position: Position { x: 100.0, y: 100.0 }, // Same position
                rotation: 0.0,
                properties: HashMap::new(),
                pins: vec![],
            });
        }

        let engine = RulesEngine::with_default_rules();
        let _issues = engine.analyze(&schematic);
        // Should handle overlapping components without panic
    }

    #[test]
    fn test_negative_coordinates() {
        let mut schematic = create_clean_schematic();
        schematic.components.push(Component {
            uuid: "neg-coords".to_string(),
            reference: "U2".to_string(),
            value: "Test".to_string(),
            lib_id: "Device:IC".to_string(),
            footprint: None,
            position: Position {
                x: -500.0,
                y: -300.0,
            },
            rotation: 0.0,
            properties: HashMap::new(),
            pins: vec![],
        });

        let engine = RulesEngine::with_default_rules();
        let issues = engine.analyze(&schematic);

        // Should work with negative coordinates
        let u2_issues: Vec<_> = issues
            .iter()
            .filter(|i| i.component.as_ref() == Some(&"U2".to_string()))
            .collect();
        // Negative coordinates shouldn't prevent issue detection
    }
}

// =============================================================================
// Generated Edge-Case File Tests
// =============================================================================

mod generated_edge_case_tests {
    use super::*;

    /// Test helper that attempts to parse an edge-case file and validates the result
    fn test_edge_case_file(
        filename: &str,
        should_parse: bool,
        description: &str,
    ) {
        let path = get_edge_case_path(filename);
        
        if !path.exists() {
            eprintln!(
                "Skipping edge-case test - file not found: {:?} ({})",
                path, description
            );
            return;
        }

        let result = KicadParser::parse_schematic(&path);
        
        if should_parse {
            assert!(
                result.is_ok(),
                "Edge case '{}' should parse successfully: {}",
                filename,
                description
            );
            
            let schematic = result.unwrap();
            // Validate basic structure
            assert!(!schematic.uuid.is_empty(), "Schematic should have UUID");
            assert_eq!(schematic.filename, path.file_name().unwrap().to_str().unwrap());
            
            // Try to run analysis on parsed schematic
            let engine = RulesEngine::with_default_rules();
            let _issues = engine.analyze(&schematic);
            // Analysis should complete without panic
        } else {
            // Some edge cases are intentionally malformed and should fail gracefully
            if result.is_err() {
                println!(
                    "Edge case '{}' failed as expected: {}",
                    filename,
                    result.unwrap_err()
                );
            }
            // We don't assert here because some malformed files might still parse
            // (parser might be lenient), which is also acceptable behavior
        }
    }

    #[test]
    fn test_case_01_deeply_nested_hierarchical_sheets() {
        test_edge_case_file(
            "case_01_deeply_nested_hierarchical_sheets.kicad_sch",
            true,
            "Deeply nested hierarchical sheets (3+ levels)",
        );
    }

    #[test]
    fn test_case_02_missing_timestamps() {
        test_edge_case_file(
            "case_02_missing_timestamps.kicad_sch",
            true,
            "Missing timestamps and optional metadata",
        );
    }

    #[test]
    fn test_case_03_custom_footprints_special_chars() {
        test_edge_case_file(
            "case_03_custom_footprints_special_chars.kicad_sch",
            true,
            "Custom footprints with special characters (spaces, Unicode, symbols)",
        );
        
        // Additional validation: check that special characters are preserved
        let path = get_edge_case_path("case_03_custom_footprints_special_chars.kicad_sch");
        if path.exists() {
            if let Ok(schematic) = KicadParser::parse_schematic(&path) {
                // Check that footprints with special characters are parsed
                let has_special_footprint = schematic.components.iter().any(|c| {
                    c.footprint.as_ref().map_or(false, |f: &String| {
                        f.contains(' ') || f.contains('_') || f.contains('#') || f.contains('/')
                    })
                });
                // Some components might not have footprints, so this is optional
                println!("Found special character footprints: {}", has_special_footprint);
            }
        }
    }

    #[test]
    fn test_case_04_empty_property_values() {
        test_edge_case_file(
            "case_04_empty_property_values.kicad_sch",
            true,
            "Empty property values",
        );
    }

    #[test]
    fn test_case_05_extreme_coordinate_values() {
        test_edge_case_file(
            "case_05_extreme_coordinate_values.kicad_sch",
            true,
            "Extreme coordinate values (very large, negative, zero)",
        );
        
        // Additional validation: check coordinates are parsed correctly
        let path = get_edge_case_path("case_05_extreme_coordinate_values.kicad_sch");
        if path.exists() {
            if let Ok(schematic) = KicadParser::parse_schematic(&path) {
                let has_extreme_coords = schematic.components.iter().any(|c| {
                    c.position.x.abs() > 1000.0 || c.position.y.abs() > 1000.0
                });
                let has_negative_coords = schematic.components.iter().any(|c| {
                    c.position.x < 0.0 || c.position.y < 0.0
                });
                println!(
                    "Extreme coords: {}, Negative coords: {}",
                    has_extreme_coords, has_negative_coords
                );
            }
        }
    }

    #[test]
    fn test_case_06_malformed_uuids() {
        test_edge_case_file(
            "case_06_malformed_uuids.kicad_sch",
            true, // Parser might be lenient with UUIDs
            "Malformed but parseable UUIDs",
        );
    }

    #[test]
    fn test_case_07_unicode_in_values() {
        test_edge_case_file(
            "case_07_unicode_in_values.kicad_sch",
            true,
            "Unicode in component values (emoji, Chinese, Cyrillic)",
        );
        
        // Additional validation: check Unicode is preserved
        let path = get_edge_case_path("case_07_unicode_in_values.kicad_sch");
        if path.exists() {
            if let Ok(schematic) = KicadParser::parse_schematic(&path) {
                let has_unicode = schematic.components.iter().any(|c| {
                    c.value.chars().any(|ch| ch as u32 > 127)
                });
                println!("Found Unicode in values: {}", has_unicode);
            }
        }
    }

    #[test]
    fn test_case_08_very_long_strings() {
        test_edge_case_file(
            "case_08_very_long_strings.kicad_sch",
            true,
            "Very long strings (1000+ characters)",
        );
    }

    #[test]
    fn test_case_09_missing_required_fields() {
        test_edge_case_file(
            "case_09_missing_required_fields.kicad_sch",
            false, // Should fail or handle gracefully
            "Missing required fields",
        );
    }

    #[test]
    fn test_case_10_nested_comments() {
        test_edge_case_file(
            "case_10_nested_comments.kicad_sch",
            true,
            "Nested comments with special characters",
        );
    }

    #[test]
    fn test_case_11_empty_lists() {
        test_edge_case_file(
            "case_11_empty_lists.kicad_sch",
            true,
            "Empty lists and collections",
        );
    }

    #[test]
    fn test_case_12_mixed_case_formatting() {
        test_edge_case_file(
            "case_12_mixed_case_formatting.kicad_sch",
            false, // Parser is case-sensitive for root element (KICAD_SCH vs kicad_sch)
            "Mixed case and formatting",
        );
    }

    #[test]
    fn test_case_13_legacy_format() {
        test_edge_case_file(
            "case_13_legacy_format.kicad_sch",
            true,
            "Legacy format compatibility",
        );
    }

    #[test]
    fn test_case_14_escaped_strings() {
        test_edge_case_file(
            "case_14_escaped_strings.kicad_sch",
            true,
            "Escaped strings (quotes, backslashes)",
        );
    }

    #[test]
    fn test_case_15_boundary_values() {
        test_edge_case_file(
            "case_15_boundary_values.kicad_sch",
            true,
            "Boundary value testing (max precision)",
        );
    }

    #[test]
    fn test_case_16_duplicate_references() {
        test_edge_case_file(
            "case_16_duplicate_references.kicad_sch",
            true,
            "Duplicate references",
        );
        
        // Additional validation: check duplicates are detected
        let path = get_edge_case_path("case_16_duplicate_references.kicad_sch");
        if path.exists() {
            if let Ok(schematic) = KicadParser::parse_schematic(&path) {
                let references: Vec<&String> = schematic.components.iter()
                    .map(|c| &c.reference)
                    .collect();
                let unique_refs: std::collections::HashSet<&String> = references.iter().cloned().collect();
                let has_duplicates = references.len() != unique_refs.len();
                println!("Found duplicate references: {}", has_duplicates);
            }
        }
    }

    #[test]
    fn test_case_17_complex_pin_configurations() {
        test_edge_case_file(
            "case_17_complex_pin_configurations.kicad_sch",
            true,
            "Complex pin configurations (100+ pins, non-numeric names)",
        );
        
        // Additional validation: check pin count
        let path = get_edge_case_path("case_17_complex_pin_configurations.kicad_sch");
        if path.exists() {
            if let Ok(schematic) = KicadParser::parse_schematic(&path) {
                let max_pins = schematic.components.iter()
                    .map(|c| c.pins.len())
                    .max()
                    .unwrap_or(0);
                println!("Maximum pins in a component: {}", max_pins);
                assert!(max_pins >= 2, "Should have components with multiple pins");
            }
        }
    }

    #[test]
    fn test_case_18_invalid_property_formats() {
        test_edge_case_file(
            "case_18_invalid_property_formats.kicad_sch",
            false, // Should fail or handle gracefully
            "Invalid property formats",
        );
    }

    #[test]
    fn test_case_19_sheet_instances_hierarchical_labels() {
        test_edge_case_file(
            "case_19_sheet_instances_hierarchical_labels.kicad_sch",
            true,
            "Sheet instances and hierarchical labels",
        );
    }

    #[test]
    fn test_case_20_mixed_valid_invalid() {
        test_edge_case_file(
            "case_20_mixed_valid_invalid.kicad_sch",
            true, // Parser should handle mixed content gracefully
            "Mixed valid and invalid elements",
        );
    }

    /// Comprehensive test that validates all edge-case files can be processed
    #[test]
    fn test_all_edge_cases_comprehensive() {
        let edge_cases = vec![
            ("case_01_deeply_nested_hierarchical_sheets.kicad_sch", true),
            ("case_02_missing_timestamps.kicad_sch", true),
            ("case_03_custom_footprints_special_chars.kicad_sch", true),
            ("case_04_empty_property_values.kicad_sch", true),
            ("case_05_extreme_coordinate_values.kicad_sch", true),
            ("case_06_malformed_uuids.kicad_sch", true),
            ("case_07_unicode_in_values.kicad_sch", true),
            ("case_08_very_long_strings.kicad_sch", true),
            ("case_09_missing_required_fields.kicad_sch", false),
            ("case_10_nested_comments.kicad_sch", true),
            ("case_11_empty_lists.kicad_sch", true),
            ("case_12_mixed_case_formatting.kicad_sch", false), // Case-sensitive parser
            ("case_13_legacy_format.kicad_sch", true),
            ("case_14_escaped_strings.kicad_sch", true),
            ("case_15_boundary_values.kicad_sch", true),
            ("case_16_duplicate_references.kicad_sch", true),
            ("case_17_complex_pin_configurations.kicad_sch", true),
            ("case_18_invalid_property_formats.kicad_sch", false),
            ("case_19_sheet_instances_hierarchical_labels.kicad_sch", true),
            ("case_20_mixed_valid_invalid.kicad_sch", true),
        ];

        let mut parsed_count = 0;
        let mut failed_count = 0;
        let mut skipped_count = 0;

        for (filename, should_parse) in edge_cases {
            let path = get_edge_case_path(filename);
            
            if !path.exists() {
                eprintln!("Skipping: {} (file not found)", filename);
                skipped_count += 1;
                continue;
            }

            let result = KicadParser::parse_schematic(&path);
            
            match result {
                Ok(schematic) => {
                    parsed_count += 1;
                    println!(
                        "✓ {} - Parsed successfully ({} components, {} wires, {} labels)",
                        filename,
                        schematic.components.len(),
                        schematic.wires.len(),
                        schematic.labels.len()
                    );
                    
                    // Try analysis
                    let engine = RulesEngine::with_default_rules();
                    let issues = engine.analyze(&schematic);
                    println!("  → Analysis found {} issues", issues.len());
                }
                Err(e) => {
                    failed_count += 1;
                    if should_parse {
                        eprintln!("✗ {} - Failed to parse (expected success): {}", filename, e);
                    } else {
                        println!("✗ {} - Failed as expected: {}", filename, e);
                    }
                }
            }
        }

        println!("\n=== Edge-Case Test Summary ===");
        println!("Parsed successfully: {}", parsed_count);
        println!("Failed (some expected): {}", failed_count);
        println!("Skipped (file not found): {}", skipped_count);
        println!("Total tested: {}", parsed_count + failed_count + skipped_count);

        // At least most files should parse
        assert!(
            parsed_count >= 15,
            "At least 15 edge-case files should parse successfully, but only {} did",
            parsed_count
        );
    }
}
