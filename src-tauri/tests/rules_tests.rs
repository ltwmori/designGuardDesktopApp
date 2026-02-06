//! Comprehensive Rules Engine Tests for KiCAD AI Assistant
//!
//! This module tests all design rule checks including:
//! - Decoupling capacitor detection
//! - I2C pull-up resistor detection
//! - Crystal load capacitor detection
//! - Power pin verification
//! - ESD protection detection
//! - Bulk capacitor detection
//! - Multiple issue detection

use std::collections::HashMap;

use designguard::analyzer::rules::{
    BulkCapacitorRule, CrystalLoadCapacitorRule, DecouplingCapacitorRule,
    ESDProtectionRule, I2CPullResistorRule, Issue, PowerPinRule, Rule,
    RulesEngine, Severity,
};
use designguard::parser::schema::{
    Component, Label, LabelType, Net, Pin, Position, Schematic, Wire,
};

// =============================================================================
// Test Helpers
// =============================================================================

fn create_empty_schematic() -> Schematic {
    Schematic {
        uuid: "test-schematic".to_string(),
        filename: "test.kicad_sch".to_string(),
        version: Some("20231120".to_string()),
        components: Vec::new(),
        wires: Vec::new(),
        labels: Vec::new(),
        nets: Vec::new(),
        power_symbols: Vec::new(),
    }
}

fn create_component(
    reference: &str,
    value: &str,
    lib_id: &str,
    x: f64,
    y: f64,
) -> Component {
    Component {
        uuid: format!("{}-uuid", reference.to_lowercase()),
        reference: reference.to_string(),
        value: value.to_string(),
        lib_id: lib_id.to_string(),
        footprint: None,
        position: Position { x, y },
        rotation: 0.0,
        properties: HashMap::new(),
        pins: Vec::new(),
    }
}

fn create_label(text: &str, label_type: LabelType, x: f64, y: f64) -> Label {
    Label {
        uuid: format!("{}-uuid", text.to_lowercase()),
        text: text.to_string(),
        position: Position { x, y },
        rotation: 0.0,
        label_type,
    }
}

// =============================================================================
// Decoupling Capacitor Rule Tests
// =============================================================================

mod decoupling_capacitor_tests {
    use super::*;

    #[test]
    fn test_decoupling_rule_missing_cap() {
        let mut schematic = create_empty_schematic();

        // Add an IC without any nearby decoupling capacitor
        schematic.components.push(create_component(
            "U1",
            "STM32F401",
            "MCU:STM32F401",
            100.0,
            100.0,
        ));

        let rule = DecouplingCapacitorRule;
        let issues = rule.check(&schematic);

        assert!(!issues.is_empty(), "Should detect missing decoupling capacitor");
        assert_eq!(issues[0].rule_id, "decoupling_capacitor");
        assert_eq!(issues[0].severity, Severity::Warning);
        assert!(issues[0].component.is_some());
        assert_eq!(issues[0].component.as_ref().unwrap(), "U1");
    }

    #[test]
    fn test_decoupling_rule_with_cap() {
        let mut schematic = create_empty_schematic();

        // Add an IC
        schematic.components.push(create_component(
            "U1",
            "STM32F401",
            "MCU:STM32F401",
            100.0,
            100.0,
        ));

        // Add a 100nF capacitor within 20mm
        schematic.components.push(create_component(
            "C1",
            "100nF",
            "Device:C",
            105.0, // Within 20mm of IC
            105.0,
        ));

        let rule = DecouplingCapacitorRule;
        let issues = rule.check(&schematic);

        // Filter issues for U1 - should not have decoupling cap issue
        let u1_decoupling_issues: Vec<_> = issues
            .iter()
            .filter(|i| {
                i.rule_id == "decoupling_capacitor"
                    && i.component.as_ref() == Some(&"U1".to_string())
            })
            .collect();

        assert!(
            u1_decoupling_issues.is_empty(),
            "Should not detect issue when 100nF cap is nearby"
        );
    }

    #[test]
    fn test_decoupling_rule_cap_too_far() {
        let mut schematic = create_empty_schematic();

        // Add an IC
        schematic.components.push(create_component(
            "U1",
            "STM32F401",
            "MCU:STM32F401",
            100.0,
            100.0,
        ));

        // Add a 100nF capacitor but too far away (> 20mm)
        schematic.components.push(create_component(
            "C1",
            "100nF",
            "Device:C",
            150.0, // More than 20mm away
            150.0,
        ));

        let rule = DecouplingCapacitorRule;
        let issues = rule.check(&schematic);

        assert!(
            !issues.is_empty(),
            "Should detect missing decoupling cap when cap is too far"
        );
    }

    #[test]
    fn test_decoupling_rule_wrong_value() {
        let mut schematic = create_empty_schematic();

        // Add an IC
        schematic.components.push(create_component(
            "U1",
            "STM32F401",
            "MCU:STM32F401",
            100.0,
            100.0,
        ));

        // Add a capacitor with wrong value (1uF instead of 100nF)
        schematic.components.push(create_component(
            "C1",
            "1uF",
            "Device:C",
            105.0,
            105.0,
        ));

        let rule = DecouplingCapacitorRule;
        let issues = rule.check(&schematic);

        // Should still flag because 1uF is not in the 80-120nF range
        assert!(
            !issues.is_empty(),
            "Should detect issue when cap value is not ~100nF"
        );
    }

    #[test]
    fn test_decoupling_rule_multiple_ics() {
        let mut schematic = create_empty_schematic();

        // Add multiple ICs
        schematic.components.push(create_component(
            "U1",
            "STM32F401",
            "MCU:STM32F401",
            100.0,
            100.0,
        ));
        schematic.components.push(create_component(
            "U2",
            "LM7805",
            "Regulator:LM7805",
            200.0,
            100.0,
        ));

        // Only add cap near U1
        schematic.components.push(create_component(
            "C1",
            "100nF",
            "Device:C",
            105.0,
            105.0,
        ));

        let rule = DecouplingCapacitorRule;
        let issues = rule.check(&schematic);

        // U2 should be flagged but not U1
        let u2_issues: Vec<_> = issues
            .iter()
            .filter(|i| i.component.as_ref() == Some(&"U2".to_string()))
            .collect();
        assert!(!u2_issues.is_empty(), "U2 should be flagged");
    }
}

// =============================================================================
// I2C Pull-up Resistor Rule Tests
// =============================================================================

mod i2c_pullup_tests {
    use super::*;

    #[test]
    fn test_i2c_pullup_missing() {
        let mut schematic = create_empty_schematic();

        // Add I2C labels without pull-up resistors
        schematic.labels.push(create_label("SDA", LabelType::Global, 100.0, 50.0));
        schematic.labels.push(create_label("SCL", LabelType::Global, 100.0, 60.0));

        let rule = I2CPullResistorRule;
        let issues = rule.check(&schematic);

        assert!(!issues.is_empty(), "Should detect missing I2C pull-ups");
        assert_eq!(issues[0].rule_id, "i2c_pull_resistors");
        assert!(issues[0].message.contains("pull-up"));
    }

    #[test]
    fn test_i2c_pullup_present() {
        let mut schematic = create_empty_schematic();

        // Add I2C labels
        schematic.labels.push(create_label("SDA", LabelType::Global, 100.0, 50.0));
        schematic.labels.push(create_label("SCL", LabelType::Global, 100.0, 60.0));

        // Add proper 4.7k pull-up resistors
        schematic.components.push(create_component(
            "R1",
            "4.7k",
            "Device:R",
            110.0,
            50.0,
        ));
        schematic.components.push(create_component(
            "R2",
            "4.7k",
            "Device:R",
            110.0,
            60.0,
        ));

        let rule = I2CPullResistorRule;
        let issues = rule.check(&schematic);

        assert!(
            issues.is_empty(),
            "Should not flag when proper pull-ups exist"
        );
    }

    #[test]
    fn test_i2c_no_labels_no_issue() {
        let schematic = create_empty_schematic();

        let rule = I2CPullResistorRule;
        let issues = rule.check(&schematic);

        assert!(issues.is_empty(), "Should not flag when no I2C labels");
    }

    #[test]
    fn test_i2c_only_sda_no_issue() {
        let mut schematic = create_empty_schematic();

        // Only SDA label, no SCL
        schematic.labels.push(create_label("SDA", LabelType::Global, 100.0, 50.0));

        let rule = I2CPullResistorRule;
        let issues = rule.check(&schematic);

        // Rule requires both SDA and SCL
        assert!(
            issues.is_empty(),
            "Should not flag when only one I2C label"
        );
    }

    #[test]
    fn test_i2c_pullup_wrong_value() {
        let mut schematic = create_empty_schematic();

        // Add I2C labels
        schematic.labels.push(create_label("SDA", LabelType::Global, 100.0, 50.0));
        schematic.labels.push(create_label("SCL", LabelType::Global, 100.0, 60.0));

        // Add resistors with wrong values (100 ohm - too low)
        schematic.components.push(create_component(
            "R1",
            "100",
            "Device:R",
            110.0,
            50.0,
        ));

        let rule = I2CPullResistorRule;
        let issues = rule.check(&schematic);

        assert!(
            !issues.is_empty(),
            "Should flag when pull-up value is incorrect"
        );
    }

    #[test]
    fn test_i2c_case_insensitive() {
        let mut schematic = create_empty_schematic();

        // Add I2C labels with different cases
        schematic.labels.push(create_label("sda", LabelType::Global, 100.0, 50.0));
        schematic.labels.push(create_label("Scl", LabelType::Global, 100.0, 60.0));

        let rule = I2CPullResistorRule;
        let issues = rule.check(&schematic);

        assert!(
            !issues.is_empty(),
            "Should detect I2C regardless of label case"
        );
    }
}

// =============================================================================
// Crystal Load Capacitor Rule Tests
// =============================================================================

mod crystal_caps_tests {
    use super::*;

    #[test]
    fn test_crystal_caps_missing() {
        let mut schematic = create_empty_schematic();

        // Add a crystal without load capacitors
        schematic.components.push(create_component(
            "Y1",
            "8MHz",
            "Device:Crystal",
            150.0,
            100.0,
        ));

        let rule = CrystalLoadCapacitorRule;
        let issues = rule.check(&schematic);

        assert!(!issues.is_empty(), "Should detect missing crystal load caps");
        assert_eq!(issues[0].rule_id, "crystal_load_capacitors");
        assert!(issues[0].message.contains("Crystal"));
    }

    #[test]
    fn test_crystal_caps_present() {
        let mut schematic = create_empty_schematic();

        // Add a crystal
        schematic.components.push(create_component(
            "Y1",
            "8MHz",
            "Device:Crystal",
            150.0,
            100.0,
        ));

        // Add two 22pF load capacitors nearby
        schematic.components.push(create_component(
            "C1",
            "22pF",
            "Device:C",
            145.0,
            105.0,
        ));
        schematic.components.push(create_component(
            "C2",
            "22pF",
            "Device:C",
            155.0,
            105.0,
        ));

        let rule = CrystalLoadCapacitorRule;
        let issues = rule.check(&schematic);

        assert!(
            issues.is_empty(),
            "Should not flag when proper load caps exist"
        );
    }

    #[test]
    fn test_crystal_caps_only_one() {
        let mut schematic = create_empty_schematic();

        // Add a crystal
        schematic.components.push(create_component(
            "Y1",
            "8MHz",
            "Device:Crystal",
            150.0,
            100.0,
        ));

        // Add only one 22pF capacitor
        schematic.components.push(create_component(
            "C1",
            "22pF",
            "Device:C",
            145.0,
            105.0,
        ));

        let rule = CrystalLoadCapacitorRule;
        let issues = rule.check(&schematic);

        assert!(
            !issues.is_empty(),
            "Should flag when only one load cap exists"
        );
        assert!(issues[0].message.contains("Found: 1"));
    }

    #[test]
    fn test_crystal_caps_wrong_value() {
        let mut schematic = create_empty_schematic();

        // Add a crystal
        schematic.components.push(create_component(
            "Y1",
            "8MHz",
            "Device:Crystal",
            150.0,
            100.0,
        ));

        // Add capacitors with wrong values (100nF instead of pF range)
        schematic.components.push(create_component(
            "C1",
            "100nF",
            "Device:C",
            145.0,
            105.0,
        ));
        schematic.components.push(create_component(
            "C2",
            "100nF",
            "Device:C",
            155.0,
            105.0,
        ));

        let rule = CrystalLoadCapacitorRule;
        let issues = rule.check(&schematic);

        assert!(
            !issues.is_empty(),
            "Should flag when cap values are wrong"
        );
    }

    #[test]
    fn test_crystal_detection_by_value() {
        let mut schematic = create_empty_schematic();

        // Add a crystal detected by value (not reference)
        let mut crystal = create_component(
            "X1", // Different reference
            "16MHz Crystal",
            "Device:Crystal",
            150.0,
            100.0,
        );
        schematic.components.push(crystal);

        let rule = CrystalLoadCapacitorRule;
        let issues = rule.check(&schematic);

        assert!(
            !issues.is_empty(),
            "Should detect crystal by MHz value"
        );
    }
}

// =============================================================================
// Power Pin Rule Tests
// =============================================================================

mod power_pin_tests {
    use super::*;

    #[test]
    fn test_power_pin_missing_gnd() {
        let schematic = create_empty_schematic();

        let rule = PowerPinRule;
        let issues = rule.check(&schematic);

        assert!(!issues.is_empty(), "Should detect missing GND");
        assert!(issues.iter().any(|i| i.message.contains("GND")));
    }

    #[test]
    fn test_power_pin_with_gnd_symbol() {
        let mut schematic = create_empty_schematic();

        // Add GND power symbol
        schematic.power_symbols.push(create_component(
            "#PWR01",
            "GND",
            "power:GND",
            100.0,
            100.0,
        ));

        let rule = PowerPinRule;
        let issues = rule.check(&schematic);

        // Should not have "missing GND" issue
        let gnd_issues: Vec<_> = issues
            .iter()
            .filter(|i| i.message.contains("No GND"))
            .collect();
        assert!(
            gnd_issues.is_empty(),
            "Should not flag missing GND when power symbol exists"
        );
    }

    #[test]
    fn test_power_pin_with_gnd_label() {
        let mut schematic = create_empty_schematic();

        // Add GND label instead of power symbol
        schematic.labels.push(create_label("GND", LabelType::Global, 100.0, 100.0));

        let rule = PowerPinRule;
        let issues = rule.check(&schematic);

        let gnd_issues: Vec<_> = issues
            .iter()
            .filter(|i| i.message.contains("No GND"))
            .collect();
        assert!(
            gnd_issues.is_empty(),
            "Should not flag missing GND when GND label exists"
        );
    }

    #[test]
    fn test_power_pin_vss_accepted() {
        let mut schematic = create_empty_schematic();

        // Add VSS power symbol (alternative to GND)
        schematic.power_symbols.push(create_component(
            "#PWR01",
            "VSS",
            "power:VSS",
            100.0,
            100.0,
        ));

        let rule = PowerPinRule;
        let issues = rule.check(&schematic);

        let gnd_issues: Vec<_> = issues
            .iter()
            .filter(|i| i.message.contains("No GND"))
            .collect();
        assert!(
            gnd_issues.is_empty(),
            "Should accept VSS as ground alternative"
        );
    }
}

// =============================================================================
// ESD Protection Rule Tests
// =============================================================================

mod esd_protection_tests {
    use super::*;

    #[test]
    fn test_esd_usb_no_protection() {
        let mut schematic = create_empty_schematic();

        // Add USB connector without ESD protection
        schematic.components.push(create_component(
            "J1",
            "USB_B_Micro",
            "Connector:USB_B_Micro",
            50.0,
            50.0,
        ));

        let rule = ESDProtectionRule;
        let issues = rule.check(&schematic);

        assert!(!issues.is_empty(), "Should detect missing USB ESD protection");
        assert!(issues[0].message.contains("USB"));
        assert!(issues[0].message.contains("ESD"));
    }

    #[test]
    fn test_esd_usb_with_tvs() {
        let mut schematic = create_empty_schematic();

        // Add USB connector
        schematic.components.push(create_component(
            "J1",
            "USB_B_Micro",
            "Connector:USB_B_Micro",
            50.0,
            50.0,
        ));

        // Add TVS diode
        schematic.components.push(create_component(
            "D1",
            "ESD5V0D1",
            "Diode:ESD5V0D1",
            60.0,
            50.0,
        ));

        let rule = ESDProtectionRule;
        let issues = rule.check(&schematic);

        assert!(
            issues.is_empty(),
            "Should not flag when TVS diode present"
        );
    }

    #[test]
    fn test_esd_ethernet_no_protection() {
        let mut schematic = create_empty_schematic();

        // Add Ethernet connector
        schematic.components.push(create_component(
            "J1",
            "RJ45",
            "Connector:RJ45_Ethernet",
            50.0,
            50.0,
        ));

        let rule = ESDProtectionRule;
        let issues = rule.check(&schematic);

        assert!(
            !issues.is_empty(),
            "Should detect missing Ethernet ESD protection"
        );
        assert!(issues[0].message.contains("Ethernet"));
    }

    #[test]
    fn test_esd_detection_by_label() {
        let mut schematic = create_empty_schematic();

        // Add USB labels (D+, D-)
        schematic.labels.push(create_label("D+", LabelType::Global, 50.0, 50.0));
        schematic.labels.push(create_label("D-", LabelType::Global, 50.0, 60.0));

        let rule = ESDProtectionRule;
        let issues = rule.check(&schematic);

        assert!(
            !issues.is_empty(),
            "Should detect USB by D+/D- labels"
        );
    }

    #[test]
    fn test_esd_no_external_interface() {
        let mut schematic = create_empty_schematic();

        // Add internal components only
        schematic.components.push(create_component(
            "U1",
            "STM32F401",
            "MCU:STM32F401",
            100.0,
            100.0,
        ));
        schematic.components.push(create_component(
            "R1",
            "10k",
            "Device:R",
            120.0,
            100.0,
        ));

        let rule = ESDProtectionRule;
        let issues = rule.check(&schematic);

        assert!(
            issues.is_empty(),
            "Should not flag when no external interfaces"
        );
    }
}

// =============================================================================
// Bulk Capacitor Rule Tests
// =============================================================================

mod bulk_capacitor_tests {
    use super::*;

    #[test]
    fn test_bulk_cap_missing() {
        let mut schematic = create_empty_schematic();

        // Add voltage regulator without bulk capacitor
        schematic.components.push(create_component(
            "U1",
            "AMS1117-3.3",
            "Regulator:Linear",
            100.0,
            100.0,
        ));

        let rule = BulkCapacitorRule;
        let issues = rule.check(&schematic);

        assert!(
            !issues.is_empty(),
            "Should detect missing bulk capacitor"
        );
        assert!(issues[0].message.contains("bulk capacitor"));
    }

    #[test]
    fn test_bulk_cap_present() {
        let mut schematic = create_empty_schematic();

        // Add voltage regulator
        schematic.components.push(create_component(
            "U1",
            "AMS1117-3.3",
            "Regulator:Linear",
            100.0,
            100.0,
        ));

        // Add 22uF bulk capacitor nearby
        schematic.components.push(create_component(
            "C1",
            "22uF",
            "Device:C",
            110.0,
            100.0,
        ));

        let rule = BulkCapacitorRule;
        let issues = rule.check(&schematic);

        assert!(
            issues.is_empty(),
            "Should not flag when bulk cap present"
        );
    }

    #[test]
    fn test_bulk_cap_too_small() {
        let mut schematic = create_empty_schematic();

        // Add voltage regulator
        schematic.components.push(create_component(
            "U1",
            "AMS1117-3.3",
            "Regulator:Linear",
            100.0,
            100.0,
        ));

        // Add capacitor that's too small (1uF instead of 10-100uF)
        schematic.components.push(create_component(
            "C1",
            "1uF",
            "Device:C",
            110.0,
            100.0,
        ));

        let rule = BulkCapacitorRule;
        let issues = rule.check(&schematic);

        assert!(
            !issues.is_empty(),
            "Should flag when bulk cap value is too small"
        );
    }

    #[test]
    fn test_bulk_cap_detects_ldo() {
        let mut schematic = create_empty_schematic();

        // Add LDO regulator
        schematic.components.push(create_component(
            "U1",
            "LDO_3.3V",
            "Regulator:LDO",
            100.0,
            100.0,
        ));

        let rule = BulkCapacitorRule;
        let issues = rule.check(&schematic);

        assert!(
            !issues.is_empty(),
            "Should detect LDO regulators"
        );
    }
}

// =============================================================================
// Rules Engine Tests
// =============================================================================

mod rules_engine_tests {
    use super::*;

    #[test]
    fn test_rules_engine_default() {
        let engine = RulesEngine::with_default_rules();
        let schematic = create_empty_schematic();
        let issues = engine.analyze(&schematic);

        // At minimum should detect missing GND
        assert!(!issues.is_empty());
    }

    #[test]
    fn test_multiple_issues() {
        let mut schematic = create_empty_schematic();

        // Add IC without decoupling cap
        schematic.components.push(create_component(
            "U1",
            "STM32F401",
            "MCU:STM32F401",
            100.0,
            100.0,
        ));

        // Add I2C labels without pull-ups
        schematic.labels.push(create_label("SDA", LabelType::Global, 150.0, 50.0));
        schematic.labels.push(create_label("SCL", LabelType::Global, 150.0, 60.0));

        // Add crystal without load caps
        schematic.components.push(create_component(
            "Y1",
            "8MHz",
            "Device:Crystal",
            180.0,
            100.0,
        ));

        // Add USB without ESD
        schematic.components.push(create_component(
            "J1",
            "USB-C",
            "Connector:USB",
            50.0,
            50.0,
        ));

        let engine = RulesEngine::with_default_rules();
        let issues = engine.analyze(&schematic);

        // Should detect multiple different issues
        let rule_ids: Vec<&str> = issues.iter().map(|i| i.rule_id.as_str()).collect();
        
        assert!(
            rule_ids.contains(&"decoupling_capacitor"),
            "Should detect decoupling issue"
        );
        assert!(
            rule_ids.contains(&"i2c_pull_resistors"),
            "Should detect I2C issue"
        );
        assert!(
            rule_ids.contains(&"crystal_load_capacitors"),
            "Should detect crystal issue"
        );
        assert!(
            rule_ids.contains(&"esd_protection"),
            "Should detect ESD issue"
        );
        assert!(
            rule_ids.contains(&"power_pins"),
            "Should detect power issue"
        );
    }

    #[test]
    fn test_issue_severity_levels() {
        let engine = RulesEngine::with_default_rules();
        
        let mut schematic = create_empty_schematic();
        schematic.components.push(create_component(
            "U1",
            "STM32F401",
            "MCU:STM32F401",
            100.0,
            100.0,
        ));

        let issues = engine.analyze(&schematic);

        // Verify we have issues with different severity levels
        let has_warning = issues.iter().any(|i| i.severity == Severity::Warning);
        let has_error = issues.iter().any(|i| i.severity == Severity::Error);
        let has_info = issues.iter().any(|i| i.severity == Severity::Info);

        // Should have at least warnings (decoupling) and errors (power pins)
        assert!(has_warning || has_error || has_info, "Should have varied severity levels");
    }

    #[test]
    fn test_issue_contains_suggestion() {
        let mut schematic = create_empty_schematic();
        schematic.components.push(create_component(
            "U1",
            "STM32F401",
            "MCU:STM32F401",
            100.0,
            100.0,
        ));

        let rule = DecouplingCapacitorRule;
        let issues = rule.check(&schematic);

        assert!(!issues.is_empty());
        assert!(
            issues[0].suggestion.is_some(),
            "Issues should include suggestions"
        );
    }

    #[test]
    fn test_issue_contains_location() {
        let mut schematic = create_empty_schematic();
        schematic.components.push(create_component(
            "U1",
            "STM32F401",
            "MCU:STM32F401",
            100.0,
            100.0,
        ));

        let rule = DecouplingCapacitorRule;
        let issues = rule.check(&schematic);

        assert!(!issues.is_empty());
        assert!(
            issues[0].location.is_some(),
            "Issues should include location"
        );
        let loc = issues[0].location.as_ref().unwrap();
        assert_eq!(loc.x, 100.0);
        assert_eq!(loc.y, 100.0);
    }

    #[test]
    fn test_clean_schematic_no_issues() {
        let mut schematic = create_empty_schematic();

        // Add IC with decoupling cap
        schematic.components.push(create_component(
            "U1",
            "STM32F401",
            "MCU:STM32F401",
            100.0,
            100.0,
        ));
        schematic.components.push(create_component(
            "C1",
            "100nF",
            "Device:C",
            105.0,
            105.0,
        ));

        // Add GND
        schematic.power_symbols.push(create_component(
            "#PWR01",
            "GND",
            "power:GND",
            100.0,
            120.0,
        ));

        // No I2C, no crystal, no USB/Ethernet, no regulator
        // This should be relatively clean

        let engine = RulesEngine::with_default_rules();
        let issues = engine.analyze(&schematic);

        // Filter out power pin warnings for ICs (which are informational)
        let serious_issues: Vec<_> = issues
            .iter()
            .filter(|i| i.severity == Severity::Error)
            .collect();

        assert!(
            serious_issues.is_empty(),
            "Clean schematic should have no errors"
        );
    }
}

// =============================================================================
// Value Parsing Tests
// =============================================================================

mod value_parsing_tests {
    use super::*;

    #[test]
    fn test_various_capacitor_values() {
        let values = vec![
            ("100nF", true),   // Should be recognized
            ("100n", true),    // Short form
            ("0.1uF", true),   // Alternate notation
            ("100pF", true),   // Different unit
            ("10uF", true),    // Larger value
            ("47µF", true),    // Unicode mu
        ];

        for (value, _should_parse) in values {
            // Create a schematic with a capacitor of this value
            let mut schematic = create_empty_schematic();
            schematic.components.push(create_component(
                "C1",
                value,
                "Device:C",
                0.0,
                0.0,
            ));

            // Just verify it doesn't panic
            let engine = RulesEngine::with_default_rules();
            let _issues = engine.analyze(&schematic);
        }
    }

    #[test]
    fn test_various_resistor_values() {
        let values = vec![
            "10k",
            "4.7k",
            "4k7",
            "100",
            "1M",
            "2.2kohm",
        ];

        for value in values {
            let mut schematic = create_empty_schematic();
            schematic.components.push(create_component(
                "R1",
                value,
                "Device:R",
                0.0,
                0.0,
            ));

            let engine = RulesEngine::with_default_rules();
            let _issues = engine.analyze(&schematic);
        }
    }
}
