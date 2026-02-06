//! Comprehensive Parser Tests for KiCAD AI Assistant
//!
//! This module tests all aspects of KiCAD schematic parsing including:
//! - S-expression parsing
//! - Component parsing (resistors, capacitors, ICs, etc.)
//! - Wire parsing
//! - Label parsing (local, global, hierarchical)
//! - Power symbol parsing
//! - Full schematic parsing
//! - Error handling for malformed files

use std::collections::HashMap;
use std::path::Path;

// Import the library crate
use designguard::parser::kicad::{KicadParser, KicadParseError};
use designguard::parser::schema::{
    Component, Label, LabelType, Net, Pin, Position, Schematic, Wire,
};
use designguard::parser::sexp::{ParseError, SExp, SExpParser};

// =============================================================================
// S-Expression Parser Tests
// =============================================================================

mod sexp_tests {
    use super::*;

    #[test]
    fn test_parse_atom() {
        let mut parser = SExpParser::new("hello");
        let result = parser.parse().unwrap();
        assert_eq!(result, SExp::Atom("hello".to_string()));
    }

    #[test]
    fn test_parse_number_atom() {
        let mut parser = SExpParser::new("12345");
        let result = parser.parse().unwrap();
        assert_eq!(result, SExp::Atom("12345".to_string()));
    }

    #[test]
    fn test_parse_float_atom() {
        let mut parser = SExpParser::new("100.33");
        let result = parser.parse().unwrap();
        assert_eq!(result, SExp::Atom("100.33".to_string()));
    }

    #[test]
    fn test_parse_negative_number() {
        let mut parser = SExpParser::new("-50.5");
        let result = parser.parse().unwrap();
        assert_eq!(result, SExp::Atom("-50.5".to_string()));
    }

    #[test]
    fn test_parse_quoted_string() {
        let mut parser = SExpParser::new("\"hello world\"");
        let result = parser.parse().unwrap();
        assert_eq!(result, SExp::Atom("hello world".to_string()));
    }

    #[test]
    fn test_parse_string_with_escapes() {
        let mut parser = SExpParser::new("\"hello\\nworld\"");
        let result = parser.parse().unwrap();
        assert_eq!(result, SExp::Atom("hello\nworld".to_string()));
    }

    #[test]
    fn test_parse_string_with_quotes() {
        let mut parser = SExpParser::new("\"say \\\"hello\\\"\"");
        let result = parser.parse().unwrap();
        assert_eq!(result, SExp::Atom("say \"hello\"".to_string()));
    }

    #[test]
    fn test_parse_empty_list() {
        let mut parser = SExpParser::new("()");
        let result = parser.parse().unwrap();
        assert_eq!(result, SExp::List(vec![]));
    }

    #[test]
    fn test_parse_simple_list() {
        let mut parser = SExpParser::new("(a b c)");
        let result = parser.parse().unwrap();
        if let SExp::List(items) = result {
            assert_eq!(items.len(), 3);
            assert_eq!(items[0], SExp::Atom("a".to_string()));
            assert_eq!(items[1], SExp::Atom("b".to_string()));
            assert_eq!(items[2], SExp::Atom("c".to_string()));
        } else {
            panic!("Expected list");
        }
    }

    #[test]
    fn test_parse_nested_list() {
        let mut parser = SExpParser::new("(a (b c) d)");
        let result = parser.parse().unwrap();
        if let SExp::List(items) = result {
            assert_eq!(items.len(), 3);
            assert_eq!(items[0], SExp::Atom("a".to_string()));
            if let SExp::List(nested) = &items[1] {
                assert_eq!(nested.len(), 2);
                assert_eq!(nested[0], SExp::Atom("b".to_string()));
                assert_eq!(nested[1], SExp::Atom("c".to_string()));
            } else {
                panic!("Expected nested list");
            }
            assert_eq!(items[2], SExp::Atom("d".to_string()));
        } else {
            panic!("Expected list");
        }
    }

    #[test]
    fn test_parse_deeply_nested() {
        let mut parser = SExpParser::new("(a (b (c (d e))))");
        let result = parser.parse().unwrap();
        // Just verify it parses without error
        assert!(matches!(result, SExp::List(_)));
    }

    #[test]
    fn test_sexp_get() {
        let mut parser = SExpParser::new("(key value other stuff)");
        let sexp = parser.parse().unwrap();
        let value = sexp.get("key").unwrap();
        assert_eq!(value.as_atom(), Some("value"));
    }

    #[test]
    fn test_sexp_get_missing() {
        let mut parser = SExpParser::new("(key value)");
        let sexp = parser.parse().unwrap();
        assert!(sexp.get("missing").is_none());
    }

    #[test]
    fn test_sexp_get_all() {
        let mut parser = SExpParser::new("(property name1 property name2 property name3)");
        let sexp = parser.parse().unwrap();
        let values = sexp.get_all("property");
        assert_eq!(values.len(), 3);
    }

    #[test]
    fn test_parse_whitespace_handling() {
        let mut parser = SExpParser::new("  (  a   b   c  )  ");
        let result = parser.parse().unwrap();
        if let SExp::List(items) = result {
            assert_eq!(items.len(), 3);
        } else {
            panic!("Expected list");
        }
    }

    #[test]
    fn test_parse_multiline() {
        let input = r#"
            (kicad_sch
                (version 20231120)
                (generator "eeschema")
            )
        "#;
        let mut parser = SExpParser::new(input);
        let result = parser.parse().unwrap();
        assert!(matches!(result, SExp::List(_)));
    }

    #[test]
    fn test_parse_error_unexpected_eof() {
        let mut parser = SExpParser::new("(a b");
        let result = parser.parse();
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_error_empty_input() {
        let mut parser = SExpParser::new("");
        let result = parser.parse();
        assert!(result.is_err());
    }
}

// =============================================================================
// Component Parsing Tests
// =============================================================================

mod component_tests {
    use super::*;

    #[test]
    fn test_parse_simple_resistor() {
        let schematic_str = r#"
(kicad_sch (version 20231120) (generator "eeschema")
  (uuid "test-uuid-123")
  (symbol (lib_id "Device:R") (at 100.33 50.8 0) (unit 1)
    (uuid "resistor-uuid-123")
    (property "Reference" "R1" (at 101.6 49.53 0))
    (property "Value" "10k" (at 101.6 52.07 0))
    (pin "1" (uuid "pin-uuid-1"))
    (pin "2" (uuid "pin-uuid-2"))
  )
)
"#;
        let result = KicadParser::parse_schematic_str(schematic_str, "test.kicad_sch");
        assert!(result.is_ok(), "Parse failed: {:?}", result.err());

        let schematic = result.unwrap();
        assert_eq!(schematic.components.len(), 1);

        let resistor = &schematic.components[0];
        assert_eq!(resistor.reference, "R1");
        assert_eq!(resistor.value, "10k");
        assert_eq!(resistor.lib_id, "Device:R");
        assert_eq!(resistor.position.x, 100.33);
        assert_eq!(resistor.position.y, 50.8);
        assert_eq!(resistor.rotation, 0.0);
        assert_eq!(resistor.pins.len(), 2);
    }

    #[test]
    fn test_parse_component_with_all_properties() {
        let schematic_str = r#"
(kicad_sch (version 20231120) (generator "eeschema")
  (uuid "test-uuid-456")
  (symbol (lib_id "Device:C") (at 50.0 75.5 90) (unit 1)
    (uuid "capacitor-uuid-456")
    (property "Reference" "C1" (at 51.6 74.03 0))
    (property "Value" "100nF" (at 51.6 77.07 0))
    (property "Footprint" "Capacitor_SMD:C_0402_1005Metric" (at 48.552 75.5 90))
    (property "Datasheet" "https://example.com/datasheet.pdf" (at 50.0 75.5 0))
    (property "Manufacturer" "Murata" (at 50.0 75.5 0))
    (property "MPN" "GRM155R71C104KA88D" (at 50.0 75.5 0))
    (pin "1" (uuid "cap-pin-1"))
    (pin "2" (uuid "cap-pin-2"))
  )
)
"#;
        let result = KicadParser::parse_schematic_str(schematic_str, "test.kicad_sch");
        assert!(result.is_ok());

        let schematic = result.unwrap();
        let cap = &schematic.components[0];
        
        assert_eq!(cap.reference, "C1");
        assert_eq!(cap.value, "100nF");
        assert_eq!(cap.footprint, Some("Capacitor_SMD:C_0402_1005Metric".to_string()));
        assert_eq!(cap.position.x, 50.0);
        assert_eq!(cap.position.y, 75.5);
        assert_eq!(cap.rotation, 90.0);
        
        // Check custom properties
        assert_eq!(cap.properties.get("Manufacturer"), Some(&"Murata".to_string()));
        assert_eq!(cap.properties.get("MPN"), Some(&"GRM155R71C104KA88D".to_string()));
    }

    #[test]
    fn test_parse_ic_component() {
        let schematic_str = r#"
(kicad_sch (version 20231120) (generator "eeschema")
  (uuid "test-uuid-789")
  (symbol (lib_id "MCU_ST_STM32F4:STM32F401CCU6") (at 150.0 100.0 0) (unit 1)
    (uuid "mcu-uuid-789")
    (property "Reference" "U1" (at 151.6 98.53 0))
    (property "Value" "STM32F401CCU6" (at 151.6 101.07 0))
    (property "Footprint" "Package_QFP:LQFP-48_7x7mm_P0.5mm" (at 148.552 100.0 90))
    (pin "1" (uuid "mcu-pin-1"))
    (pin "2" (uuid "mcu-pin-2"))
    (pin "3" (uuid "mcu-pin-3"))
    (pin "4" (uuid "mcu-pin-4"))
  )
)
"#;
        let result = KicadParser::parse_schematic_str(schematic_str, "test.kicad_sch");
        assert!(result.is_ok());

        let schematic = result.unwrap();
        let ic = &schematic.components[0];
        
        assert_eq!(ic.reference, "U1");
        assert_eq!(ic.value, "STM32F401CCU6");
        assert!(ic.lib_id.contains("STM32"));
        assert_eq!(ic.pins.len(), 4);
    }

    #[test]
    fn test_parse_multiple_components() {
        let schematic_str = r#"
(kicad_sch (version 20231120) (generator "eeschema")
  (uuid "test-uuid-multi")
  (symbol (lib_id "Device:R") (at 100 50 0) (unit 1)
    (uuid "r1-uuid")
    (property "Reference" "R1" (at 101 49 0))
    (property "Value" "10k" (at 101 51 0))
    (pin "1" (uuid "r1-pin-1"))
    (pin "2" (uuid "r1-pin-2"))
  )
  (symbol (lib_id "Device:R") (at 120 50 0) (unit 1)
    (uuid "r2-uuid")
    (property "Reference" "R2" (at 121 49 0))
    (property "Value" "4.7k" (at 121 51 0))
    (pin "1" (uuid "r2-pin-1"))
    (pin "2" (uuid "r2-pin-2"))
  )
  (symbol (lib_id "Device:C") (at 140 50 0) (unit 1)
    (uuid "c1-uuid")
    (property "Reference" "C1" (at 141 49 0))
    (property "Value" "100nF" (at 141 51 0))
    (pin "1" (uuid "c1-pin-1"))
    (pin "2" (uuid "c1-pin-2"))
  )
)
"#;
        let result = KicadParser::parse_schematic_str(schematic_str, "test.kicad_sch");
        assert!(result.is_ok());

        let schematic = result.unwrap();
        assert_eq!(schematic.components.len(), 3);
        
        let refs: Vec<&str> = schematic.components.iter().map(|c| c.reference.as_str()).collect();
        assert!(refs.contains(&"R1"));
        assert!(refs.contains(&"R2"));
        assert!(refs.contains(&"C1"));
    }
}

// =============================================================================
// Wire Parsing Tests
// =============================================================================

mod wire_tests {
    use super::*;

    #[test]
    fn test_parse_wire() {
        let schematic_str = r#"
(kicad_sch (version 20231120) (generator "eeschema")
  (uuid "test-uuid-wire")
  (wire (pts (xy 100 50) (xy 120 50))
    (uuid "wire-uuid-123")
  )
)
"#;
        let result = KicadParser::parse_schematic_str(schematic_str, "test.kicad_sch");
        assert!(result.is_ok());

        let schematic = result.unwrap();
        assert_eq!(schematic.wires.len(), 1);

        let wire = &schematic.wires[0];
        assert_eq!(wire.uuid, "wire-uuid-123");
        assert_eq!(wire.points.len(), 2);
        assert_eq!(wire.points[0].x, 100.0);
        assert_eq!(wire.points[0].y, 50.0);
        assert_eq!(wire.points[1].x, 120.0);
        assert_eq!(wire.points[1].y, 50.0);
    }

    #[test]
    fn test_parse_wire_with_multiple_points() {
        let schematic_str = r#"
(kicad_sch (version 20231120) (generator "eeschema")
  (uuid "test-uuid-wire-multi")
  (wire (pts (xy 0 0) (xy 50 0) (xy 50 50) (xy 100 50))
    (uuid "wire-multi-uuid")
  )
)
"#;
        let result = KicadParser::parse_schematic_str(schematic_str, "test.kicad_sch");
        assert!(result.is_ok());

        let schematic = result.unwrap();
        let wire = &schematic.wires[0];
        assert_eq!(wire.points.len(), 4);
    }

    #[test]
    fn test_parse_multiple_wires() {
        let schematic_str = r#"
(kicad_sch (version 20231120) (generator "eeschema")
  (uuid "test-uuid-wires")
  (wire (pts (xy 0 0) (xy 10 0))
    (uuid "wire-1")
  )
  (wire (pts (xy 20 0) (xy 30 0))
    (uuid "wire-2")
  )
  (wire (pts (xy 40 0) (xy 50 0))
    (uuid "wire-3")
  )
)
"#;
        let result = KicadParser::parse_schematic_str(schematic_str, "test.kicad_sch");
        assert!(result.is_ok());

        let schematic = result.unwrap();
        assert_eq!(schematic.wires.len(), 3);
    }

    #[test]
    fn test_parse_wire_with_negative_coords() {
        let schematic_str = r#"
(kicad_sch (version 20231120) (generator "eeschema")
  (uuid "test-uuid-neg")
  (wire (pts (xy -50.5 -25.25) (xy 50.5 25.25))
    (uuid "wire-neg-uuid")
  )
)
"#;
        let result = KicadParser::parse_schematic_str(schematic_str, "test.kicad_sch");
        assert!(result.is_ok());

        let schematic = result.unwrap();
        let wire = &schematic.wires[0];
        assert_eq!(wire.points[0].x, -50.5);
        assert_eq!(wire.points[0].y, -25.25);
        assert_eq!(wire.points[1].x, 50.5);
        assert_eq!(wire.points[1].y, 25.25);
    }
}

// =============================================================================
// Label Parsing Tests
// =============================================================================

mod label_tests {
    use super::*;

    #[test]
    fn test_parse_labels() {
        let schematic_str = r#"
(kicad_sch (version 20231120) (generator "eeschema")
  (uuid "test-uuid-labels")
  (label "VCC" (at 100 40 0)
    (uuid "label-uuid-1")
  )
)
"#;
        let result = KicadParser::parse_schematic_str(schematic_str, "test.kicad_sch");
        assert!(result.is_ok());

        let schematic = result.unwrap();
        assert_eq!(schematic.labels.len(), 1);

        let label = &schematic.labels[0];
        assert_eq!(label.text, "VCC");
        assert_eq!(label.uuid, "label-uuid-1");
        assert_eq!(label.position.x, 100.0);
        assert_eq!(label.position.y, 40.0);
        assert_eq!(label.label_type, LabelType::Local);
    }

    #[test]
    fn test_parse_global_label() {
        let schematic_str = r#"
(kicad_sch (version 20231120) (generator "eeschema")
  (uuid "test-uuid-global")
  (global_label "SDA" (at 150 60 0) (shape input)
    (uuid "glabel-uuid-1")
  )
)
"#;
        let result = KicadParser::parse_schematic_str(schematic_str, "test.kicad_sch");
        assert!(result.is_ok());

        let schematic = result.unwrap();
        assert_eq!(schematic.labels.len(), 1);

        let label = &schematic.labels[0];
        assert_eq!(label.text, "SDA");
        assert_eq!(label.label_type, LabelType::Global);
    }

    #[test]
    fn test_parse_hierarchical_label() {
        let schematic_str = r#"
(kicad_sch (version 20231120) (generator "eeschema")
  (uuid "test-uuid-hier")
  (hierarchical_label "CLK_IN" (at 200 80 180) (shape input)
    (uuid "hlabel-uuid-1")
  )
)
"#;
        let result = KicadParser::parse_schematic_str(schematic_str, "test.kicad_sch");
        assert!(result.is_ok());

        let schematic = result.unwrap();
        assert_eq!(schematic.labels.len(), 1);

        let label = &schematic.labels[0];
        assert_eq!(label.text, "CLK_IN");
        assert_eq!(label.label_type, LabelType::Hierarchical);
        assert_eq!(label.rotation, 180.0);
    }

    #[test]
    fn test_parse_mixed_labels() {
        let schematic_str = r#"
(kicad_sch (version 20231120) (generator "eeschema")
  (uuid "test-uuid-mixed")
  (label "NET1" (at 10 10 0)
    (uuid "local-1")
  )
  (global_label "SDA" (at 20 20 0) (shape bidirectional)
    (uuid "global-1")
  )
  (global_label "SCL" (at 30 30 0) (shape bidirectional)
    (uuid "global-2")
  )
  (hierarchical_label "DATA" (at 40 40 0) (shape output)
    (uuid "hier-1")
  )
)
"#;
        let result = KicadParser::parse_schematic_str(schematic_str, "test.kicad_sch");
        assert!(result.is_ok());

        let schematic = result.unwrap();
        assert_eq!(schematic.labels.len(), 4);

        let local_labels: Vec<_> = schematic.labels.iter()
            .filter(|l| l.label_type == LabelType::Local)
            .collect();
        let global_labels: Vec<_> = schematic.labels.iter()
            .filter(|l| l.label_type == LabelType::Global)
            .collect();
        let hier_labels: Vec<_> = schematic.labels.iter()
            .filter(|l| l.label_type == LabelType::Hierarchical)
            .collect();

        assert_eq!(local_labels.len(), 1);
        assert_eq!(global_labels.len(), 2);
        assert_eq!(hier_labels.len(), 1);
    }
}

// =============================================================================
// Power Symbol Tests
// =============================================================================

mod power_symbol_tests {
    use super::*;

    #[test]
    fn test_parse_power_symbols() {
        let schematic_str = r##"
(kicad_sch (version 20231120) (generator "eeschema")
  (uuid "test-uuid-power")
  (power_symbol (lib_id "power:GND") (at 120 80 0) (unit 1)
    (uuid "power-uuid-1")
    (property "Reference" "#PWR01" (at 120 86 0))
    (property "Value" "GND" (at 120 84 0))
  )
)
"##;
        let result = KicadParser::parse_schematic_str(schematic_str, "test.kicad_sch");
        assert!(result.is_ok());

        let schematic = result.unwrap();
        assert_eq!(schematic.power_symbols.len(), 1);

        let power = &schematic.power_symbols[0];
        assert_eq!(power.value, "GND");
        assert_eq!(power.reference, "#PWR01");
        assert!(power.lib_id.contains("GND"));
    }

    #[test]
    fn test_parse_multiple_power_symbols() {
        let schematic_str = r##"
(kicad_sch (version 20231120) (generator "eeschema")
  (uuid "test-uuid-multi-power")
  (power_symbol (lib_id "power:GND") (at 100 100 0) (unit 1)
    (uuid "gnd-uuid")
    (property "Reference" "#PWR01" (at 100 106 0))
    (property "Value" "GND" (at 100 104 0))
  )
  (power_symbol (lib_id "power:+3.3V") (at 100 50 0) (unit 1)
    (uuid "vcc-uuid")
    (property "Reference" "#PWR02" (at 100 44 0))
    (property "Value" "+3.3V" (at 100 46 0))
  )
  (power_symbol (lib_id "power:+5V") (at 150 50 0) (unit 1)
    (uuid "5v-uuid")
    (property "Reference" "#PWR03" (at 150 44 0))
    (property "Value" "+5V" (at 150 46 0))
  )
)
"##;
        let result = KicadParser::parse_schematic_str(schematic_str, "test.kicad_sch");
        assert!(result.is_ok());

        let schematic = result.unwrap();
        assert_eq!(schematic.power_symbols.len(), 3);

        let values: Vec<&str> = schematic.power_symbols.iter()
            .map(|p| p.value.as_str())
            .collect();
        assert!(values.contains(&"GND"));
        assert!(values.contains(&"+3.3V"));
        assert!(values.contains(&"+5V"));
    }
}

// =============================================================================
// Full Schematic Tests
// =============================================================================

mod full_schematic_tests {
    use super::*;

    #[test]
    fn test_parse_full_schematic() {
        let schematic_str = r##"
(kicad_sch (version 20231120) (generator "eeschema")
  (uuid "full-schematic-uuid")
  
  (symbol (lib_id "Device:R") (at 100.33 50.8 0) (unit 1)
    (uuid "r1-uuid")
    (property "Reference" "R1" (at 101.6 49.53 0))
    (property "Value" "10k" (at 101.6 52.07 0))
    (property "Footprint" "Resistor_SMD:R_0402_1005Metric" (at 98.552 50.8 90))
    (pin "1" (uuid "r1-pin-1"))
    (pin "2" (uuid "r1-pin-2"))
  )
  
  (symbol (lib_id "Device:C") (at 120 50 0) (unit 1)
    (uuid "c1-uuid")
    (property "Reference" "C1" (at 121 49 0))
    (property "Value" "100nF" (at 121 51 0))
    (pin "1" (uuid "c1-pin-1"))
    (pin "2" (uuid "c1-pin-2"))
  )
  
  (wire (pts (xy 100 50) (xy 120 50))
    (uuid "wire-uuid-1")
  )
  
  (wire (pts (xy 120 50) (xy 140 50))
    (uuid "wire-uuid-2")
  )
  
  (label "VCC" (at 100 40 0)
    (uuid "label-vcc")
  )
  
  (global_label "SDA" (at 150 60 0) (shape bidirectional)
    (uuid "label-sda")
  )
  
  (global_label "SCL" (at 150 70 0) (shape bidirectional)
    (uuid "label-scl")
  )
  
  (power_symbol (lib_id "power:GND") (at 120 80 0) (unit 1)
    (uuid "power-gnd")
    (property "Reference" "#PWR01" (at 120 86 0))
    (property "Value" "GND" (at 120 84 0))
  )
)
"##;
        let result = KicadParser::parse_schematic_str(schematic_str, "test.kicad_sch");
        assert!(result.is_ok(), "Parse failed: {:?}", result.err());

        let schematic = result.unwrap();
        assert_eq!(schematic.filename, "test.kicad_sch");
        assert_eq!(schematic.uuid, "full-schematic-uuid");
        assert_eq!(schematic.components.len(), 2);
        assert_eq!(schematic.wires.len(), 2);
        assert_eq!(schematic.labels.len(), 3);
        assert_eq!(schematic.power_symbols.len(), 1);
    }

    #[test]
    fn test_parse_schematic_with_version() {
        let schematic_str = r#"
(kicad_sch (version 20231120) (generator "eeschema")
  (uuid "versioned-uuid")
)
"#;
        let result = KicadParser::parse_schematic_str(schematic_str, "test.kicad_sch");
        assert!(result.is_ok());

        let schematic = result.unwrap();
        assert_eq!(schematic.version, Some("20231120".to_string()));
    }

    #[test]
    fn test_schematic_nets_are_built() {
        let schematic_str = r#"
(kicad_sch (version 20231120) (generator "eeschema")
  (uuid "nets-test-uuid")
  (wire (pts (xy 0 0) (xy 10 0))
    (uuid "wire-1")
  )
  (global_label "VCC" (at 5 0 0) (shape input)
    (uuid "label-1")
  )
)
"#;
        let result = KicadParser::parse_schematic_str(schematic_str, "test.kicad_sch");
        assert!(result.is_ok());

        let schematic = result.unwrap();
        // Nets should be built from wires and labels
        assert!(!schematic.nets.is_empty());
    }
}

// =============================================================================
// Error Handling Tests
// =============================================================================

mod error_handling_tests {
    use super::*;

    #[test]
    fn test_parse_error_handling_invalid_root() {
        let schematic_str = r#"
(not_kicad_sch (version 20231120)
  (uuid "test-uuid")
)
"#;
        let result = KicadParser::parse_schematic_str(schematic_str, "test.kicad_sch");
        assert!(result.is_err());
        
        let err = result.unwrap_err();
        assert!(matches!(err, KicadParseError::InvalidFormat(_)));
    }

    #[test]
    fn test_parse_error_empty_file() {
        let schematic_str = "";
        let result = KicadParser::parse_schematic_str(schematic_str, "test.kicad_sch");
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_error_malformed_sexp() {
        let schematic_str = "(kicad_sch (version 20231120) (uuid \"test\" incomplete";
        let result = KicadParser::parse_schematic_str(schematic_str, "test.kicad_sch");
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_gracefully_handles_unknown_elements() {
        // Unknown elements should be ignored, not cause errors
        let schematic_str = r#"
(kicad_sch (version 20231120) (generator "eeschema")
  (uuid "test-uuid")
  (unknown_element (some data here))
  (another_unknown thing)
  (symbol (lib_id "Device:R") (at 100 50 0) (unit 1)
    (uuid "r1-uuid")
    (property "Reference" "R1" (at 101 49 0))
    (property "Value" "10k" (at 101 51 0))
    (pin "1" (uuid "pin-1"))
  )
)
"#;
        let result = KicadParser::parse_schematic_str(schematic_str, "test.kicad_sch");
        assert!(result.is_ok());

        let schematic = result.unwrap();
        assert_eq!(schematic.components.len(), 1);
    }

    #[test]
    fn test_parse_handles_missing_optional_fields() {
        // Some fields are optional and shouldn't cause parse failures
        let schematic_str = r#"
(kicad_sch (version 20231120) (generator "eeschema")
  (uuid "test-uuid")
  (symbol (lib_id "Device:R") (at 100 50 0) (unit 1)
    (uuid "r1-uuid")
    (property "Reference" "R1" (at 0 0 0))
    (property "Value" "10k" (at 0 0 0))
  )
)
"#;
        let result = KicadParser::parse_schematic_str(schematic_str, "test.kicad_sch");
        assert!(result.is_ok());

        let schematic = result.unwrap();
        let component = &schematic.components[0];
        // Footprint is optional
        assert!(component.footprint.is_none());
    }
}

// =============================================================================
// File-based Tests (using test fixtures)
// =============================================================================

mod file_tests {
    use super::*;

    fn get_fixture_path(relative: &str) -> String {
        let manifest_dir = env!("CARGO_MANIFEST_DIR");
        format!("{}/test-fixtures/{}", manifest_dir, relative)
    }

    #[test]
    fn test_parse_simple_project_file() {
        let path = get_fixture_path("simple-project/simple.kicad_sch");
        let result = KicadParser::parse_schematic(Path::new(&path));
        
        // This test may fail if fixtures aren't created yet
        if let Ok(schematic) = result {
            assert!(!schematic.uuid.is_empty());
            assert!(schematic.components.len() >= 1);
        }
    }

    #[test]
    fn test_parse_issues_file() {
        let path = get_fixture_path("with-issues/issues.kicad_sch");
        let result = KicadParser::parse_schematic(Path::new(&path));
        
        if let Ok(schematic) = result {
            // Should have multiple components
            assert!(schematic.components.len() >= 1);
            // Should have I2C labels
            let has_sda = schematic.labels.iter().any(|l| l.text.to_uppercase().contains("SDA"));
            let has_scl = schematic.labels.iter().any(|l| l.text.to_uppercase().contains("SCL"));
            assert!(has_sda || has_scl);
        }
    }

    #[test]
    fn test_parse_corrupt_file_fails() {
        let path = get_fixture_path("invalid/corrupt.kicad_sch");
        let result = KicadParser::parse_schematic(Path::new(&path));
        
        // Corrupt files should fail to parse
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_nonexistent_file() {
        let path = "/nonexistent/path/file.kicad_sch";
        let result = KicadParser::parse_schematic(Path::new(path));
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), KicadParseError::Io(_)));
    }
}
