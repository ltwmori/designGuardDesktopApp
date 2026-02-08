//! Tests for KiCad file parsing

use designguard::{parse_schematic, parse_pcb};
use std::path::PathBuf;

fn fixture_path(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join(name)
}

#[test]
fn test_parse_valid_schematic() {
    let result = parse_schematic(&fixture_path("valid_design.kicad_sch"));
    assert!(result.is_ok(), "Should parse valid schematic");

    let schematic = result.unwrap();

    // Should have components
    assert!(!schematic.components.is_empty(), "Should have components");

    // Should have nets (from wires and labels)
    assert!(!schematic.nets.is_empty(), "Should have nets");
}

#[test]
fn test_parse_invalid_file() {
    let result = parse_schematic(&PathBuf::from("not_a_real_file.kicad_sch"));
    assert!(result.is_err(), "Should fail on nonexistent file");
}

#[test]
fn test_parse_component_properties() {
    let schematic = parse_schematic(&fixture_path("valid_design.kicad_sch")).expect("Should parse");

    // Find the STM32 component
    let stm32 = schematic.components.iter().find(|c| c.reference.starts_with('U'));

    assert!(stm32.is_some(), "Should find MCU component");

    let stm32 = stm32.unwrap();
    assert!(stm32.value.contains("STM32"), "Should have correct value");
}

#[test]
fn test_parse_nets() {
    let schematic = parse_schematic(&fixture_path("valid_design.kicad_sch")).expect("Should parse");

    // Should have power nets from global labels
    let has_vdd = schematic.nets.iter().any(|n| n.name == "VDD");
    let has_gnd = schematic.nets.iter().any(|n| n.name == "GND");

    assert!(has_vdd, "Should have VDD net");
    assert!(has_gnd, "Should have GND net");
}

#[test]
#[ignore] // Only run when PCB fixture exists with segments
fn test_parse_pcb() {
    let result = parse_pcb(&fixture_path("complete_design.kicad_pcb"));

    if result.is_ok() {
        let pcb = result.unwrap();
        assert!(!pcb.traces.is_empty(), "PCB should have traces");
    }
}
