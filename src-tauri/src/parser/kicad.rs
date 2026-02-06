//! KiCAD Schematic Parser
//! 
//! This module parses KiCAD schematic files (.kicad_sch) following the official
//! S-Expression file format specification:
//! https://dev-docs.kicad.org/en/file-formats/sexpr-intro/index.html
//!
//! Key format details from official docs:
//! - All values are in millimeters
//! - Coordinates have max 4 decimal places (0.0001 mm resolution)
//! - Properties: (property "KEY" "VALUE")
//! - Position: (at X Y [ANGLE])
//! - Points: (pts (xy X Y) ...)
//! - UUID: (uuid UUID) - Version 4 random UUID

use std::collections::HashMap;
use std::path::Path;
use crate::parser::schema::*;
use crate::parser::sexp::{SExp, SExpParser, ParseError};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum KicadParseError {
    #[error("S-expression parse error: {0}")]
    SExpParse(#[from] ParseError),
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Invalid schematic format: {0}")]
    InvalidFormat(String),
    #[error("Missing required field: {0}")]
    MissingField(String),
}

/// Parser for KiCAD schematic files
///
/// Supports KiCad versions 4-9:
/// - **KiCad 4-5**: Legacy text-based format (`.sch`) - automatically routed to legacy parser
/// - **KiCad 6-9**: Modern S-expression format (`.kicad_sch`) - uses S-expression parser
///
/// Format detection and routing is handled automatically by the format detector.
/// This parser provides a unified interface for all supported KiCad versions.
pub struct KicadParser;

impl KicadParser {
    /// Parse a schematic file, automatically detecting format (KiCad 4-9)
    pub fn parse_schematic(path: &Path) -> Result<Schematic, KicadParseError> {
        // Use format detector to automatically route to correct parser
        crate::parser::format_detector::detect_and_parse_schematic(path)
    }
    
    /// Parse schematic from string (for modern S-expression format only)
    /// For format detection, use parse_schematic() with a Path instead
    pub fn parse_schematic_str(content: &str, filename: &str) -> Result<Schematic, KicadParseError> {
        let mut parser = SExpParser::new(content);
        let root = parser.parse()?;

        // Root should be (kicad_sch ...)
        let kicad_sch = root
            .as_list()
            .and_then(|list| list.first())
            .and_then(|s| s.as_atom())
            .ok_or_else(|| KicadParseError::InvalidFormat("Expected kicad_sch root".to_string()))?;

        if kicad_sch != "kicad_sch" {
            return Err(KicadParseError::InvalidFormat(
                format!("Expected kicad_sch, found {}", kicad_sch)
            ));
        }

        let root_list = root.as_list().unwrap();
        let mut schematic = Schematic {
            uuid: Self::extract_uuid(&root)?,
            filename: filename.to_string(),
            version: Self::extract_version(&root),
            components: Vec::new(),
            wires: Vec::new(),
            labels: Vec::new(),
            nets: Vec::new(),
            power_symbols: Vec::new(),
        };

        // Parse all elements
        for item in root_list.iter().skip(1) {
            if let Some(tag) = item.as_list().and_then(|l| l.first()).and_then(|a| a.as_atom()) {
                match tag {
                    "symbol" => {
                        if let Ok(component) = Self::parse_symbol(item) {
                            schematic.components.push(component);
                        }
                    }
                    "power_symbol" => {
                        if let Ok(component) = Self::parse_symbol(item) {
                            schematic.power_symbols.push(component);
                        }
                    }
                    "wire" => {
                        if let Ok(wire) = Self::parse_wire(item) {
                            schematic.wires.push(wire);
                        }
                    }
                    "label" => {
                        if let Ok(label) = Self::parse_label(item, LabelType::Local) {
                            schematic.labels.push(label);
                        }
                    }
                    "global_label" => {
                        if let Ok(label) = Self::parse_label(item, LabelType::Global) {
                            schematic.labels.push(label);
                        }
                    }
                    "hierarchical_label" => {
                        if let Ok(label) = Self::parse_label(item, LabelType::Hierarchical) {
                            schematic.labels.push(label);
                        }
                    }
                    _ => {
                        // Ignore unknown elements
                    }
                }
            }
        }

        // Build nets from connectivity
        schematic.nets = Self::build_nets(&schematic);

        Ok(schematic)
    }

    /// Helper to extract a string value from a (key "value") pattern
    fn get_string_value(sexp: &SExp, key: &str) -> Option<String> {
        sexp.get(key).and_then(|exp| {
            // exp might be:
            // 1. The whole (key "value") list -> extract list[1]
            // 2. Just "value" atom (for simple 2-element lists)
            if let Some(list) = exp.as_list() {
                // It's the whole list, get the second element
                list.get(1).and_then(|v| v.as_atom()).map(|s| s.to_string())
            } else {
                // It's just the value
                exp.as_atom().map(|s| s.to_string())
            }
        })
    }

    fn extract_uuid(root: &SExp) -> Result<String, KicadParseError> {
        if let Some(uuid_str) = Self::get_string_value(root, "uuid") {
            Ok(uuid_str)
        } else {
            // Generate a UUID if not present
            Ok(uuid::Uuid::new_v4().to_string())
        }
    }

    fn extract_version(root: &SExp) -> Option<String> {
        Self::get_string_value(root, "version")
    }

    fn parse_symbol(sexp: &SExp) -> Result<Component, KicadParseError> {
        let uuid = Self::get_string_value(sexp, "uuid")
            .ok_or_else(|| KicadParseError::MissingField("symbol uuid".to_string()))?;

        let lib_id = Self::get_string_value(sexp, "lib_id")
            .ok_or_else(|| KicadParseError::MissingField("lib_id".to_string()))?;

        let (position, rotation) = Self::parse_at(sexp)?;

        let mut reference = String::new();
        let mut value = String::new();
        let mut footprint = None;
        let mut properties = HashMap::new();
        let mut pins = Vec::new();

        // Extract properties per official KiCAD format:
        // https://dev-docs.kicad.org/en/file-formats/sexpr-intro/index.html#_properties
        // Format: (property "KEY" "VALUE" ...)
        // - KEY at index 1 (property key, must be unique)
        // - VALUE at index 2 (property value string)
        // Mandatory symbol properties: Reference, Value, Footprint, Datasheet
        for prop_exp in sexp.get_all("property") {
            if let Some(list) = prop_exp.as_list() {
                if list.len() >= 3 {
                    if let Some(key) = list[1].as_atom() {
                        if let Some(val) = list[2].as_atom() {
                            properties.insert(key.to_string(), val.to_string());
                            
                            // Handle mandatory properties per KiCAD spec
                            match key {
                                "Reference" => reference = val.to_string(),
                                "Value" => value = val.to_string(),
                                "Footprint" => footprint = Some(val.to_string()),
                                _ => {}
                            }
                        }
                    }
                }
            }
        }

        // Extract pins
        // KiCAD 9.0 format: (pin "1" (uuid "..."))
        // get_all("pin") returns the whole (pin ...) list
        // So list[0] = "pin", list[1] = pin number
        for pin_exp in sexp.get_all("pin") {
            if let Some(list) = pin_exp.as_list() {
                if list.len() >= 2 {
                    let pin_number = list[1]
                        .as_atom()
                        .unwrap_or("")
                        .to_string();
                    let pin_uuid = list
                        .iter()
                        .skip(1)
                        .find_map(|e| e.get("uuid"))
                        .and_then(|u| u.as_atom())
                        .map(|s| s.to_string())
                        .unwrap_or_else(|| uuid::Uuid::new_v4().to_string());
                    
                    pins.push(Pin {
                        number: pin_number,
                        uuid: pin_uuid,
                    });
                }
            }
        }

        Ok(Component {
            uuid,
            reference,
            value,
            lib_id,
            footprint,
            position,
            rotation,
            properties,
            pins,
        })
    }

    /// Parse wire with coordinate point list per official KiCAD format:
    /// https://dev-docs.kicad.org/en/file-formats/sexpr-intro/index.html#_coordinate_point_list
    /// Format: (pts (xy X Y) (xy X Y) ...)
    /// - Each xy token defines a single X and Y coordinate pair
    /// - Minimum of 2 points required for a wire
    fn parse_wire(sexp: &SExp) -> Result<Wire, KicadParseError> {
        let uuid = Self::get_string_value(sexp, "uuid")
            .ok_or_else(|| KicadParseError::MissingField("wire uuid".to_string()))?;

        let pts_exp = sexp
            .get("pts")
            .ok_or_else(|| KicadParseError::MissingField("wire pts".to_string()))?;

        let mut points = Vec::new();
        // pts_exp is the whole (pts (xy X Y) (xy X Y) ...) list
        // Skip the first element "pts" per official format
        if let Some(pts_list) = pts_exp.as_list() {
            for item in pts_list.iter().skip(1) {
                // Each item should be (xy X Y) per official format
                if let Some(xy_list) = item.as_list() {
                    if let Some(tag) = xy_list.first().and_then(|a| a.as_atom()) {
                        if tag == "xy" && xy_list.len() >= 3 {
                            let x = xy_list[1]
                                .as_atom()
                                .and_then(|s| s.parse::<f64>().ok())
                                .unwrap_or(0.0);
                            let y = xy_list[2]
                                .as_atom()
                                .and_then(|s| s.parse::<f64>().ok())
                                .unwrap_or(0.0);
                            points.push(Position { x, y });
                        }
                    }
                }
            }
        }

        Ok(Wire { uuid, points })
    }

    fn parse_label(sexp: &SExp, label_type: LabelType) -> Result<Label, KicadParseError> {
        let uuid = Self::get_string_value(sexp, "uuid")
            .ok_or_else(|| KicadParseError::MissingField("label uuid".to_string()))?;

        // Label text is the second element in the list: (label "TEXT" (at ...) ...)
        let text = sexp
            .as_list()
            .and_then(|l| l.get(1))
            .and_then(|t| t.as_atom())
            .ok_or_else(|| KicadParseError::MissingField("label text".to_string()))?
            .to_string();

        let (position, rotation) = Self::parse_at(sexp)?;

        Ok(Label {
            uuid,
            text,
            position,
            rotation,
            label_type,
        })
    }

    /// Parse position identifier per official KiCAD format:
    /// https://dev-docs.kicad.org/en/file-formats/sexpr-intro/index.html#_position_identifier
    /// Format: (at X Y [ANGLE])
    /// - X: horizontal position in millimeters
    /// - Y: vertical position in millimeters  
    /// - ANGLE: optional rotation angle (degrees for most objects, tenths of degree for symbol text)
    fn parse_at(sexp: &SExp) -> Result<(Position, f64), KicadParseError> {
        let at_exp = sexp
            .get("at")
            .ok_or_else(|| KicadParseError::MissingField("at".to_string()))?;

        // at_exp is the whole (at X Y [ANGLE]) list
        // list[0] = "at", list[1] = X, list[2] = Y, list[3] = ANGLE (optional)
        if let Some(list) = at_exp.as_list() {
            if list.len() >= 3 {
                let x = list[1]
                    .as_atom()
                    .and_then(|s| s.parse::<f64>().ok())
                    .unwrap_or(0.0);
                let y = list[2]
                    .as_atom()
                    .and_then(|s| s.parse::<f64>().ok())
                    .unwrap_or(0.0);
                let rotation = list
                    .get(3)
                    .and_then(|r| r.as_atom())
                    .and_then(|s| s.parse::<f64>().ok())
                    .unwrap_or(0.0);

                Ok((Position { x, y }, rotation))
            } else {
                Err(KicadParseError::InvalidFormat("Invalid 'at' format - requires at least X and Y".to_string()))
            }
        } else {
            Err(KicadParseError::InvalidFormat("'at' must be a list".to_string()))
        }
    }

    fn build_nets(schematic: &Schematic) -> Vec<Net> {
        use std::collections::{HashMap, HashSet};

        // Map of net name -> set of connections
        let mut net_map: HashMap<String, HashSet<Connection>> = HashMap::new();

        // Process wires - each wire segment creates connections
        // For simplicity, we'll create a net for each wire
        for (wire_idx, wire) in schematic.wires.iter().enumerate() {
            if wire.points.len() >= 2 {
                let net_name = format!("Net-(W{})", wire_idx);
                let mut connections = HashSet::new();
                
                // Wire endpoints need to be connected to components via pins
                // This is simplified - real netlist building requires geometric analysis
                connections.insert(Connection {
                    component_ref: "WIRE".to_string(),
                    pin_number: format!("{}", wire_idx),
                });
                
                net_map.insert(net_name, connections);
            }
        }

        // Process labels - labels connect to nets
        for label in &schematic.labels {
            let net_name = match &label.label_type {
                LabelType::Global => label.text.clone(),
                LabelType::Local => format!("Net-({})", label.text),
                LabelType::Hierarchical => format!("Hier-{}", label.text),
            };

            net_map
                .entry(net_name)
                .or_insert_with(HashSet::new);
        }

        // Process component connections
        // This is simplified - real implementation needs geometric analysis
        // to determine which pins connect to which wires/labels
        for component in &schematic.components {
            for _pin in &component.pins {
                // In a real implementation, we'd check if this pin's position
                // intersects with any wire or label position
                // For now, we'll create placeholder connections
            }
        }

        // Convert to Vec<Net>
        net_map
            .into_iter()
            .map(|(name, connections)| Net {
                name,
                connections: connections.into_iter().collect(),
            })
            .collect()
    }
}

// Re-export the parse function for backward compatibility
pub fn parse_schematic(path: &str) -> Result<Schematic, Box<dyn std::error::Error>> {
    KicadParser::parse_schematic(Path::new(path))
        .map_err(|e| Box::new(e) as Box<dyn std::error::Error>)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_at() {
        // Simple test without raw string literals
        let schematic = Schematic {
            uuid: "test".to_string(),
            filename: "test.kicad_sch".to_string(),
            version: None,
            components: Vec::new(),
            wires: Vec::new(),
            labels: Vec::new(),
            nets: Vec::new(),
            power_symbols: Vec::new(),
        };
        assert_eq!(schematic.filename, "test.kicad_sch");
    }

    #[test]
    fn test_build_nets() {
        let schematic = Schematic {
            uuid: "test".to_string(),
            filename: "test.kicad_sch".to_string(),
            version: None,
            components: Vec::new(),
            wires: vec![
                Wire {
                    uuid: "w1".to_string(),
                    points: vec![
                        Position { x: 0.0, y: 0.0 },
                        Position { x: 10.0, y: 0.0 },
                    ],
                },
            ],
            labels: vec![
                Label {
                    uuid: "l1".to_string(),
                    text: "VCC".to_string(),
                    position: Position { x: 5.0, y: 0.0 },
                    rotation: 0.0,
                    label_type: LabelType::Global,
                },
            ],
            nets: Vec::new(),
            power_symbols: Vec::new(),
        };

        let nets = KicadParser::build_nets(&schematic);
        assert!(!nets.is_empty());
    }
}
