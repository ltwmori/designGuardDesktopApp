//! Comprehensive test suite for KiCad legacy format parsing (versions 4-5)

use std::path::PathBuf;
use designguard::parser::kicad_legacy::LegacyParser;
use designguard::parser::format_detector::{detect_format, KicadVersion};
use designguard::parser::schema::Schematic;

#[test]
fn test_detect_legacy_schematic_v4() {
    let content = "EESchema Schematic File Version 4\nEELAYER 30 0";
    assert_eq!(detect_format(content), Some(KicadVersion::Legacy4));
}

#[test]
fn test_detect_legacy_schematic_v5() {
    let content = "EESchema Schematic File Version 5\nEELAYER 30 0";
    assert_eq!(detect_format(content), Some(KicadVersion::Legacy5));
}

#[test]
fn test_parse_legacy_component_basic() {
    let content = r#"EESchema Schematic File Version 4
EELAYER 30 0
EELAYER END
$Descr A3 16535 11693
encoding utf-8
$EndDescr
$Comp
L Device:R R1
U 1 1 561E4EB0
P 1200 8900
F 0 "R1" H 1200 8650 50  0001 C CNN
F 1 "10k" H 1200 8750 50  0000 C CNN
F 2 "R_0402" H 1200 8900 60  0000 C CNN
	1    1200 8900
$EndComp
$EndSCHEMATC"#;
    
    let result = LegacyParser::parse_legacy_schematic(content, "test.sch");
    assert!(result.is_ok());
    
    let schematic = result.unwrap();
    assert_eq!(schematic.components.len(), 1);
    assert_eq!(schematic.components[0].reference, "R1");
    assert_eq!(schematic.components[0].value, "10k");
    assert_eq!(schematic.components[0].footprint, Some("R_0402".to_string()));
}

#[test]
fn test_parse_legacy_wire() {
    let content = r#"EESchema Schematic File Version 4
EELAYER 30 0
EELAYER END
$Descr A3 16535 11693
$EndDescr
Wire Wire Line
	800  6800 2200 6800
$EndSCHEMATC"#;
    
    let result = LegacyParser::parse_legacy_schematic(content, "test.sch");
    assert!(result.is_ok());
    
    let schematic = result.unwrap();
    assert_eq!(schematic.wires.len(), 1);
    let wire = &schematic.wires[0];
    assert_eq!(wire.points.len(), 2);
    // Coordinates converted from internal units to mm (0.0001 mm per unit)
    assert!((wire.points[0].x - 0.08).abs() < 0.0001);
    assert!((wire.points[0].y - 0.68).abs() < 0.0001);
    assert!((wire.points[1].x - 0.22).abs() < 0.0001);
    assert!((wire.points[1].y - 0.68).abs() < 0.0001);
}

#[test]
fn test_parse_legacy_label() {
    let content = r#"EESchema Schematic File Version 4
EELAYER 30 0
EELAYER END
$Descr A3 16535 11693
$EndDescr
Text Label 5000 3300 0    60   ~ 0
VCC
$EndSCHEMATC"#;
    
    let result = LegacyParser::parse_legacy_schematic(content, "test.sch");
    assert!(result.is_ok());
    
    let schematic = result.unwrap();
    assert_eq!(schematic.labels.len(), 1);
    let label = &schematic.labels[0];
    assert_eq!(label.text, "VCC");
    // Coordinates converted to mm
    assert!((label.position.x - 0.5).abs() < 0.0001);
    assert!((label.position.y - 0.33).abs() < 0.0001);
}

#[test]
fn test_parse_legacy_power_symbol() {
    let content = r##"EESchema Schematic File Version 4
EELAYER 30 0
EELAYER END
$Descr A3 16535 11693
$EndDescr
$Comp
L power:GND #PWR01
U 1 1 561E4EB0
P 1200 8900
F 0 "#PWR01" H 1200 8650 50  0001 C CNN
F 1 "GND" H 1200 8750 50  0000 C CNN
	1    1200 8900
$EndComp
$EndSCHEMATC"##;
    
    let result = LegacyParser::parse_legacy_schematic(content, "test.sch");
    assert!(result.is_ok());
    
    let schematic = result.unwrap();
    assert_eq!(schematic.components.len(), 0); // Power symbols go to power_symbols
    assert_eq!(schematic.power_symbols.len(), 1);
    assert_eq!(schematic.power_symbols[0].reference, "#PWR01");
    assert_eq!(schematic.power_symbols[0].value, "GND");
}

#[test]
fn test_parse_legacy_complex_schematic() {
    let content = r#"EESchema Schematic File Version 4
EELAYER 30 0
EELAYER END
$Descr A3 16535 11693
encoding utf-8
Sheet 1 5
Title "Test Circuit"
$EndDescr
$Comp
L Device:R R1
U 1 1 561E4EB0
P 1200 8900
F 0 "R1" H 1200 8650 50  0001 C CNN
F 1 "10k" H 1200 8750 50  0000 C CNN
$EndComp
$Comp
L Device:C C1
U 1 1 561E4F97
P 2000 8900
F 0 "C1" H 2000 8650 50  0001 C CNN
F 1 "100nF" H 2000 8750 50  0000 C CNN
$EndComp
Wire Wire Line
	1200 8900 2000 8900
Text Label 1500 8700 0    60   ~ 0
NET1
$EndSCHEMATC"#;
    
    let result = LegacyParser::parse_legacy_schematic(content, "test.sch");
    assert!(result.is_ok());
    
    let schematic = result.unwrap();
    assert_eq!(schematic.components.len(), 2);
    assert_eq!(schematic.wires.len(), 1);
    assert_eq!(schematic.labels.len(), 1);
    assert!(!schematic.nets.is_empty());
}

#[test]
fn test_parse_legacy_empty_schematic() {
    let content = r#"EESchema Schematic File Version 4
EELAYER 30 0
EELAYER END
$Descr A3 16535 11693
$EndDescr
$EndSCHEMATC"#;
    
    let result = LegacyParser::parse_legacy_schematic(content, "test.sch");
    assert!(result.is_ok());
    
    let schematic = result.unwrap();
    assert_eq!(schematic.components.len(), 0);
    assert_eq!(schematic.wires.len(), 0);
    assert_eq!(schematic.labels.len(), 0);
}

#[test]
fn test_parse_legacy_pcb_basic() {
    let content = "PCBNEW\n$MODULE\n$EndMODULE";
    
    let result = LegacyParser::parse_legacy_pcb(content, "test.brd");
    assert!(result.is_ok());
    
    let pcb = result.unwrap();
    assert_eq!(pcb.version, Some("KiCad 4-5 (Legacy)".to_string()));
}

#[test]
fn test_parse_legacy_pcb_invalid_header() {
    let content = "NOT_PCBNEW\n$MODULE";
    
    let result = LegacyParser::parse_legacy_pcb(content, "test.brd");
    assert!(result.is_err());
}
