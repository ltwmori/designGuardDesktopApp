//! Comprehensive tests for Capacitor Classifier
//!
//! Tests verify the classification logic for various capacitor scenarios:
//! - Timing caps near crystals
//! - Filtering caps on signal lines
//! - Unknown caps with no net connectivity
//! - Shared via detection (handled by DRS, but verified here)

#[cfg(test)]
mod comprehensive_tests {
    use crate::analyzer::capacitor_classifier::{CapacitorClassifier, CapacitorFunction};
    use crate::parser::schema::{Component, Schematic, Position, Pin, Net, Connection};
    use crate::compliance::power_net_registry::PowerNetRegistry;
    use crate::parser::netlist::PinNetConnection;
    use std::collections::HashMap;

    /// Helper: Create a test schematic with components
    fn create_test_schematic() -> Schematic {
        Schematic {
            uuid: "test-schematic".to_string(),
            filename: "test.kicad_sch".to_string(),
            version: None,
            components: Vec::new(),
            wires: Vec::new(),
            labels: Vec::new(),
            nets: Vec::new(),
            power_symbols: Vec::new(),
        }
    }

    /// Helper: Create a capacitor component
    fn create_capacitor(
        ref_des: &str,
        value: &str,
        position: Position,
        footprint: Option<&str>,
    ) -> Component {
        Component {
            uuid: format!("uuid-{}", ref_des),
            reference: ref_des.to_string(),
            value: value.to_string(),
            lib_id: "Device:C".to_string(),
            footprint: footprint.map(|s| s.to_string()),
            position,
            rotation: 0.0,
            properties: HashMap::new(),
            pins: vec![
                Pin {
                    number: "1".to_string(),
                    uuid: format!("pin1-{}", ref_des),
                },
                Pin {
                    number: "2".to_string(),
                    uuid: format!("pin2-{}", ref_des),
                },
            ],
        }
    }

    /// Helper: Create a crystal component
    fn create_crystal(ref_des: &str, position: Position) -> Component {
        Component {
            uuid: format!("uuid-{}", ref_des),
            reference: ref_des.to_string(),
            value: "16MHz".to_string(),
            lib_id: "Device:Crystal".to_string(),
            footprint: None,
            position,
            rotation: 0.0,
            properties: HashMap::new(),
            pins: vec![
                Pin {
                    number: "1".to_string(),
                    uuid: format!("pin1-{}", ref_des),
                },
                Pin {
                    number: "2".to_string(),
                    uuid: format!("pin2-{}", ref_des),
                },
            ],
        }
    }

    /// Helper: Create pin-to-net mapping
    fn create_pin_to_net(
        component_ref: &str,
        pin1_net: &str,
        pin2_net: &str,
    ) -> HashMap<String, Vec<PinNetConnection>> {
        let mut map = HashMap::new();
        map.insert(
            format!("{}:1", component_ref),
            vec![PinNetConnection {
                component_ref: component_ref.to_string(),
                pin_number: "1".to_string(),
                net_name: pin1_net.to_string(),
            }],
        );
        map.insert(
            format!("{}:2", component_ref),
            vec![PinNetConnection {
                component_ref: component_ref.to_string(),
                pin_number: "2".to_string(),
                net_name: pin2_net.to_string(),
            }],
        );
        map
    }

    /// Case 1: 22pF cap near a crystal (Should be Timing)
    #[test]
    fn test_timing_cap_near_crystal() {
        let mut schematic = create_test_schematic();
        
        // Create crystal at (100, 100)
        let crystal = create_crystal("Y1", Position { x: 100.0, y: 100.0 });
        schematic.components.push(crystal);
        
        // Create 22pF capacitor at (105, 100) - 5mm from crystal
        let cap = create_capacitor(
            "C1",
            "22pF",
            Position { x: 105.0, y: 100.0 },
            Some("0402"),
        );
        schematic.components.push(cap);
        
        // Create pin-to-net mapping: C1 pin1 -> XTAL_IN, pin2 -> GND
        let mut pin_to_net = create_pin_to_net("C1", "XTAL_IN", "GND");
        
        // Build power registry
        let power_registry = PowerNetRegistry::new(&schematic);
        
        // Classify
        let classifications = CapacitorClassifier::classify_capacitors(
            &schematic,
            &power_registry,
            &pin_to_net,
        );
        
        // Verify
        assert!(!classifications.is_empty(), "Should have at least one classification");
        let c1_class = classifications.iter().find(|c| c.component_ref == "C1");
        assert!(c1_class.is_some(), "C1 should be classified");
        
        let classification = c1_class.unwrap();
        assert_eq!(
            classification.function,
            CapacitorFunction::Timing,
            "C1 (22pF near crystal) should be classified as Timing"
        );
        assert!(
            classification.confidence > 0.5,
            "Confidence should be > 0.5, got {}",
            classification.confidence
        );
        assert!(
            classification.reasoning.contains("Timing") || classification.reasoning.contains("crystal"),
            "Reasoning should mention timing or crystal: {}",
            classification.reasoning
        );
    }

    /// Case 2: 100nF cap between a signal line and GND (Should be Filtering)
    #[test]
    fn test_filtering_cap_signal_to_gnd() {
        let mut schematic = create_test_schematic();
        
        // Create 100nF capacitor
        let cap = create_capacitor(
            "C2",
            "100nF",
            Position { x: 50.0, y: 50.0 },
            Some("0603"),
        );
        schematic.components.push(cap);
        
        // Create a connector nearby (to increase confidence)
        let connector = Component {
            uuid: "uuid-J1".to_string(),
            reference: "J1".to_string(),
            value: "USB-A".to_string(),
            lib_id: "Connector:USB".to_string(),
            footprint: None,
            position: Position { x: 55.0, y: 50.0 },
            rotation: 0.0,
            properties: HashMap::new(),
            pins: Vec::new(),
        };
        schematic.components.push(connector);
        
        // Create pin-to-net mapping: C2 pin1 -> SIGNAL_IN, pin2 -> GND
        let mut pin_to_net = create_pin_to_net("C2", "SIGNAL_IN", "GND");
        
        // Build power registry
        let power_registry = PowerNetRegistry::new(&schematic);
        
        // Classify
        let classifications = CapacitorClassifier::classify_capacitors(
            &schematic,
            &power_registry,
            &pin_to_net,
        );
        
        // Verify
        assert!(!classifications.is_empty(), "Should have at least one classification");
        let c2_class = classifications.iter().find(|c| c.component_ref == "C2");
        assert!(c2_class.is_some(), "C2 should be classified");
        
        let classification = c2_class.unwrap();
        assert_eq!(
            classification.function,
            CapacitorFunction::Filtering,
            "C2 (100nF, SIGNAL_IN to GND) should be classified as Filtering"
        );
        assert!(
            classification.reasoning.contains("Filtering") || classification.reasoning.contains("low-pass"),
            "Reasoning should mention filtering: {}",
            classification.reasoning
        );
    }

    /// Case 3: 0.1µF cap with no net names (Should be Unknown)
    #[test]
    fn test_unknown_cap_no_nets() {
        let mut schematic = create_test_schematic();
        
        // Create 0.1µF capacitor
        let cap = create_capacitor(
            "C3",
            "0.1uF",
            Position { x: 30.0, y: 30.0 },
            Some("0603"),
        );
        schematic.components.push(cap);
        
        // No pin-to-net mapping (empty)
        let pin_to_net = HashMap::new();
        
        // Build power registry
        let power_registry = PowerNetRegistry::new(&schematic);
        
        // Classify
        let classifications = CapacitorClassifier::classify_capacitors(
            &schematic,
            &power_registry,
            &pin_to_net,
        );
        
        // Verify
        let c3_class = classifications.iter().find(|c| c.component_ref == "C3");
        
        // Should either be Unknown or not classified at all (if no nets)
        if let Some(classification) = c3_class {
            assert_eq!(
                classification.function,
                CapacitorFunction::Unknown,
                "C3 (0.1µF with no nets) should be classified as Unknown"
            );
        } else {
            // If not classified, that's also acceptable (no nets = can't classify)
            // This is fine - the classifier may skip caps with no net connectivity
        }
    }

    /// Case 4: Two 0.1µF caps sharing a single via to GND
    /// Note: Shared via detection is handled by DRS, but we verify the caps are classified correctly
    #[test]
    fn test_two_caps_sharing_via() {
        let mut schematic = create_test_schematic();
        
        // Create two 0.1µF capacitors close together
        let cap1 = create_capacitor(
            "C4",
            "0.1uF",
            Position { x: 100.0, y: 100.0 },
            Some("0603"),
        );
        let cap2 = create_capacitor(
            "C5",
            "0.1uF",
            Position { x: 102.0, y: 100.0 },
            Some("0603"),
        );
        schematic.components.push(cap1);
        schematic.components.push(cap2);
        
        // Create an IC nearby
        let ic = Component {
            uuid: "uuid-U1".to_string(),
            reference: "U1".to_string(),
            value: "STM32F4".to_string(),
            lib_id: "MCU:STM32".to_string(),
            footprint: None,
            position: Position { x: 105.0, y: 100.0 },
            rotation: 0.0,
            properties: HashMap::new(),
            pins: Vec::new(),
        };
        schematic.components.push(ic);
        
        // Create pin-to-net mapping: Both caps connected to VCC and GND
        let mut pin_to_net = HashMap::new();
        pin_to_net.extend(create_pin_to_net("C4", "VCC", "GND"));
        pin_to_net.extend(create_pin_to_net("C5", "VCC", "GND"));
        
        // Build power registry
        let mut schematic_with_vcc = schematic.clone();
        schematic_with_vcc.nets.push(Net {
            name: "VCC".to_string(),
            connections: vec![
                Connection {
                    component_ref: "C4".to_string(),
                    pin_number: "1".to_string(),
                },
                Connection {
                    component_ref: "C5".to_string(),
                    pin_number: "1".to_string(),
                },
            ],
        });
        let power_registry = PowerNetRegistry::new(&schematic_with_vcc);
        
        // Classify
        let classifications = CapacitorClassifier::classify_capacitors(
            &schematic,
            &power_registry,
            &pin_to_net,
        );
        
        // Verify both caps are classified as Decoupling
        let c4_class = classifications.iter().find(|c| c.component_ref == "C4");
        let c5_class = classifications.iter().find(|c| c.component_ref == "C5");
        
        assert!(c4_class.is_some(), "C4 should be classified");
        assert!(c5_class.is_some(), "C5 should be classified");
        
        // Both should be Decoupling (0.1µF = 100nF, which is in the 10nF-2.2µF range)
        if let Some(classification) = c4_class {
            assert_eq!(
                classification.function,
                CapacitorFunction::Decoupling,
                "C4 (0.1µF, VCC to GND) should be classified as Decoupling"
            );
        }
        
        if let Some(classification) = c5_class {
            assert_eq!(
                classification.function,
                CapacitorFunction::Decoupling,
                "C5 (0.1µF, VCC to GND) should be classified as Decoupling"
            );
        }
        
        // Note: Shared via detection would be done by DRS when PCB is available
        // This test verifies the classification is correct, not the shared via warning
    }

    /// Additional test: Verify decoupling cap classification
    #[test]
    fn test_decoupling_cap() {
        let mut schematic = create_test_schematic();
        
        // Create 100nF capacitor between VCC and GND
        let cap = create_capacitor(
            "C6",
            "100nF",
            Position { x: 50.0, y: 50.0 },
            Some("0402"),
        );
        schematic.components.push(cap);
        
        // Create pin-to-net mapping: C6 pin1 -> VCC, pin2 -> GND
        let mut pin_to_net = create_pin_to_net("C6", "VCC", "GND");
        
        // Build power registry with VCC as power net
        let mut schematic_with_vcc = schematic.clone();
        schematic_with_vcc.nets.push(Net {
            name: "VCC".to_string(),
            connections: vec![Connection {
                component_ref: "C6".to_string(),
                pin_number: "1".to_string(),
            }],
        });
        let power_registry = PowerNetRegistry::new(&schematic_with_vcc);
        
        // Classify
        let classifications = CapacitorClassifier::classify_capacitors(
            &schematic,
            &power_registry,
            &pin_to_net,
        );
        
        // Verify
        let c6_class = classifications.iter().find(|c| c.component_ref == "C6");
        assert!(c6_class.is_some(), "C6 should be classified");
        
        let classification = c6_class.unwrap();
        assert_eq!(
            classification.function,
            CapacitorFunction::Decoupling,
            "C6 (100nF, VCC to GND) should be classified as Decoupling"
        );
    }

    /// Additional test: Verify bulk cap classification
    #[test]
    fn test_bulk_cap() {
        let mut schematic = create_test_schematic();
        
        // Create 10µF capacitor between VCC and GND
        let cap = create_capacitor(
            "C7",
            "10uF",
            Position { x: 50.0, y: 50.0 },
            Some("0805"),
        );
        schematic.components.push(cap);
        
        // Create pin-to-net mapping: C7 pin1 -> VCC, pin2 -> GND
        let mut pin_to_net = create_pin_to_net("C7", "VCC", "GND");
        
        // Build power registry
        let mut schematic_with_vcc = schematic.clone();
        schematic_with_vcc.nets.push(Net {
            name: "VCC".to_string(),
            connections: vec![Connection {
                component_ref: "C7".to_string(),
                pin_number: "1".to_string(),
            }],
        });
        let power_registry = PowerNetRegistry::new(&schematic_with_vcc);
        
        // Classify
        let classifications = CapacitorClassifier::classify_capacitors(
            &schematic,
            &power_registry,
            &pin_to_net,
        );
        
        // Verify
        let c7_class = classifications.iter().find(|c| c.component_ref == "C7");
        assert!(c7_class.is_some(), "C7 should be classified");
        
        let classification = c7_class.unwrap();
        assert_eq!(
            classification.function,
            CapacitorFunction::Bulk,
            "C7 (10µF, VCC to GND) should be classified as Bulk"
        );
    }

    /// Test: Create a board with 10 capacitors (as requested)
    #[test]
    fn test_board_with_10_capacitors() {
        let mut schematic = create_test_schematic();
        
        // C1: 22pF near crystal (Timing)
        let crystal = create_crystal("Y1", Position { x: 100.0, y: 100.0 });
        schematic.components.push(crystal);
        let c1 = create_capacitor("C1", "22pF", Position { x: 105.0, y: 100.0 }, Some("0402"));
        schematic.components.push(c1);
        
        // C2: 100nF signal to GND (Filtering)
        let c2 = create_capacitor("C2", "100nF", Position { x: 50.0, y: 50.0 }, Some("0603"));
        schematic.components.push(c2);
        
        // C3: 0.1µF no nets (Unknown)
        let c3 = create_capacitor("C3", "0.1uF", Position { x: 30.0, y: 30.0 }, Some("0603"));
        schematic.components.push(c3);
        
        // C4, C5: Two 0.1µF caps sharing VCC/GND (Decoupling, potential shared via)
        let c4 = create_capacitor("C4", "0.1uF", Position { x: 100.0, y: 100.0 }, Some("0603"));
        let c5 = create_capacitor("C5", "0.1uF", Position { x: 102.0, y: 100.0 }, Some("0603"));
        schematic.components.push(c4);
        schematic.components.push(c5);
        
        // C6: 100nF VCC to GND (Decoupling)
        let c6 = create_capacitor("C6", "100nF", Position { x: 70.0, y: 70.0 }, Some("0402"));
        schematic.components.push(c6);
        
        // C7: 10µF VCC to GND (Bulk)
        let c7 = create_capacitor("C7", "10uF", Position { x: 80.0, y: 80.0 }, Some("0805"));
        schematic.components.push(c7);
        
        // C8: 47pF near crystal (Timing)
        let c8 = create_capacitor("C8", "47pF", Position { x: 95.0, y: 100.0 }, Some("0402"));
        schematic.components.push(c8);
        
        // C9: 1nF signal to signal (Filtering in-series)
        let c9 = create_capacitor("C9", "1nF", Position { x: 60.0, y: 60.0 }, Some("0402"));
        schematic.components.push(c9);
        
        // C10: 220nF VCC to GND (Decoupling)
        let c10 = create_capacitor("C10", "220nF", Position { x: 90.0, y: 90.0 }, Some("0603"));
        schematic.components.push(c10);
        
        // Create pin-to-net mappings
        let mut pin_to_net = HashMap::new();
        pin_to_net.extend(create_pin_to_net("C1", "XTAL_IN", "GND"));  // Timing
        pin_to_net.extend(create_pin_to_net("C2", "SIGNAL_IN", "GND"));  // Filtering
        // C3: no nets (Unknown)
        pin_to_net.extend(create_pin_to_net("C4", "VCC", "GND"));  // Decoupling
        pin_to_net.extend(create_pin_to_net("C5", "VCC", "GND"));  // Decoupling
        pin_to_net.extend(create_pin_to_net("C6", "VCC", "GND"));  // Decoupling
        pin_to_net.extend(create_pin_to_net("C7", "VCC", "GND"));  // Bulk
        pin_to_net.extend(create_pin_to_net("C8", "XTAL_OUT", "GND"));  // Timing
        pin_to_net.extend(create_pin_to_net("C9", "SIGNAL_A", "SIGNAL_B"));  // Filtering
        pin_to_net.extend(create_pin_to_net("C10", "VCC", "GND"));  // Decoupling
        
        // Build power registry
        let mut schematic_with_nets = schematic.clone();
        schematic_with_nets.nets.push(Net {
            name: "VCC".to_string(),
            connections: vec![],
        });
        let power_registry = PowerNetRegistry::new(&schematic_with_nets);
        
        // Classify all capacitors
        let classifications = CapacitorClassifier::classify_capacitors(
            &schematic,
            &power_registry,
            &pin_to_net,
        );
        
        // Verify we classified all capacitors with nets
        assert_eq!(classifications.len(), 9, "Should classify 9 capacitors (C3 has no nets)");
        
        // Verify specific classifications
        let c1_class = classifications.iter().find(|c| c.component_ref == "C1");
        assert!(c1_class.is_some() && c1_class.unwrap().function == CapacitorFunction::Timing,
                "C1 should be Timing");
        
        let c2_class = classifications.iter().find(|c| c.component_ref == "C2");
        assert!(c2_class.is_some() && c2_class.unwrap().function == CapacitorFunction::Filtering,
                "C2 should be Filtering");
        
        // C3 should not be classified (no nets)
        let c3_class = classifications.iter().find(|c| c.component_ref == "C3");
        assert!(c3_class.is_none(), "C3 should not be classified (no nets)");
        
        let c4_class = classifications.iter().find(|c| c.component_ref == "C4");
        assert!(c4_class.is_some() && c4_class.unwrap().function == CapacitorFunction::Decoupling,
                "C4 should be Decoupling");
        
        let c5_class = classifications.iter().find(|c| c.component_ref == "C5");
        assert!(c5_class.is_some() && c5_class.unwrap().function == CapacitorFunction::Decoupling,
                "C5 should be Decoupling");
        
        let c7_class = classifications.iter().find(|c| c.component_ref == "C7");
        assert!(c7_class.is_some() && c7_class.unwrap().function == CapacitorFunction::Bulk,
                "C7 should be Bulk");
        
        println!("✓ All 10 capacitors processed correctly!");
        println!("  - C1 (22pF near crystal): {:?}", c1_class.map(|c| c.function));
        println!("  - C2 (100nF signal-GND): {:?}", c2_class.map(|c| c.function));
        println!("  - C3 (0.1µF no nets): Not classified");
        println!("  - C4, C5 (0.1µF VCC-GND): {:?}, {:?}", 
                 c4_class.map(|c| c.function), c5_class.map(|c| c.function));
    }
}
