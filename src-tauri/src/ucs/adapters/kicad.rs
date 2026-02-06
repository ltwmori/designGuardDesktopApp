//! KiCAD Adapter
//!
//! This adapter converts KiCAD schematic files (.kicad_sch) to the
//! Unified Circuit Schema (UCS) format.

use std::path::Path;
use std::collections::HashMap;

use crate::parser::kicad::KicadParser;
use crate::parser::schema::{Schematic, Component, Label, LabelType, Net};

use super::{CircuitAdapter, AdapterError};
use crate::ucs::{
    UnifiedCircuitSchema, UcsComponent, UcsNet, UcsPin, UcsPosition,
    CircuitMetadata, SourceCAD, ElectricalType, NetConnection, SignalType,
};

/// Adapter for KiCAD schematic files
pub struct KicadAdapter;

impl KicadAdapter {
    pub fn new() -> Self {
        Self
    }
    
    /// Convert a KiCAD Schematic to UCS
    fn convert_schematic(&self, schematic: Schematic) -> UnifiedCircuitSchema {
        let mut ucs = UnifiedCircuitSchema {
            metadata: CircuitMetadata {
                project_name: schematic.filename
                    .replace(".kicad_sch", "")
                    .replace(".sch", ""),
                source_cad: SourceCAD::KiCad,
                cad_version: schematic.version,
                source_file: Some(schematic.filename.clone()),
                ..Default::default()
            },
            components: Vec::new(),
            nets: Vec::new(),
        };
        
        // Convert regular components
        for component in schematic.components {
            ucs.add_component(self.convert_component(component, false));
        }
        
        // Convert power symbols as virtual components
        for power_symbol in schematic.power_symbols {
            ucs.add_component(self.convert_component(power_symbol, true));
        }
        
        // Convert nets
        for net in schematic.nets {
            ucs.add_net(self.convert_net(net));
        }
        
        // Add nets from labels that might not have explicit net entries
        self.add_nets_from_labels(&schematic.labels, &mut ucs);
        
        ucs
    }
    
    /// Convert a KiCAD Component to UcsComponent
    fn convert_component(&self, component: Component, is_virtual: bool) -> UcsComponent {
        let mut ucs_comp = UcsComponent {
            ref_des: component.reference.clone(),
            mpn: self.extract_mpn(&component),
            value: if component.value.is_empty() { None } else { Some(component.value.clone()) },
            footprint: component.footprint.clone(),
            lib_id: Some(component.lib_id.clone()),
            is_virtual,
            pins: self.convert_pins(&component),
            position: Some(UcsPosition::new(component.position.x, component.position.y)),
            rotation: component.rotation,
            attributes: self.convert_properties(&component.properties),
            uuid: component.uuid.clone(),
        };
        
        // Try to infer MPN from value if not found in properties
        if ucs_comp.mpn.is_none() {
            ucs_comp.mpn = self.infer_mpn_from_value(&component.value, &component.lib_id);
        }
        
        ucs_comp
    }
    
    /// Extract MPN from component properties
    fn extract_mpn(&self, component: &Component) -> Option<String> {
        // Common property names for MPN
        let mpn_keys = ["MPN", "mpn", "Mpn", "Part Number", "PartNumber", 
                        "Manufacturer Part Number", "Mfr Part", "P/N"];
        
        for key in mpn_keys {
            if let Some(mpn) = component.properties.get(key) {
                if !mpn.is_empty() && mpn != "~" {
                    return Some(mpn.clone());
                }
            }
        }
        
        None
    }
    
    /// Try to infer MPN from value and lib_id
    fn infer_mpn_from_value(&self, value: &str, lib_id: &str) -> Option<String> {
        let value_upper = value.to_uppercase();
        
        // Known IC patterns
        let ic_patterns = [
            "STM32", "ESP32", "ATMEGA", "RP2040", "PIC", "MSP430",
            "LM7805", "LM7812", "LM1117", "AMS1117", "LM317",
            "CH340", "CP2102", "FT232", "NE555", "LM358", "LM324",
        ];
        
        for pattern in ic_patterns {
            if value_upper.contains(pattern) {
                return Some(value.to_string());
            }
        }
        
        // Check lib_id for MCU/IC indicators
        let lib_upper = lib_id.to_uppercase();
        if lib_upper.contains("MCU") || lib_upper.contains("MICROCONTROLLER") 
            || lib_upper.contains("REGULATOR") || lib_upper.contains("INTERFACE") {
            return Some(value.to_string());
        }
        
        None
    }
    
    /// Convert KiCAD pins to UCS pins
    fn convert_pins(&self, component: &Component) -> Vec<UcsPin> {
        component.pins.iter().map(|pin| {
            UcsPin {
                number: pin.number.clone(),
                name: None, // KiCAD basic schema doesn't include pin names
                electrical_type: ElectricalType::Unspecified,
                connected_net: None,
                position: None,
            }
        }).collect()
    }
    
    /// Convert properties HashMap to attributes
    fn convert_properties(&self, properties: &HashMap<String, String>) -> HashMap<String, crate::ucs::AttributeValue> {
        properties.iter()
            .filter(|(k, v)| {
                // Filter out standard properties that are already in UcsComponent
                !["Reference", "Value", "Footprint", "Datasheet"].contains(&k.as_str())
                    && !v.is_empty() && v.as_str() != "~"
            })
            .map(|(k, v)| {
                (k.clone(), crate::ucs::AttributeValue::String(v.clone()))
            })
            .collect()
    }
    
    /// Convert a KiCAD Net to UcsNet
    fn convert_net(&self, net: Net) -> UcsNet {
        let signal_type = SignalType::from_net_name(&net.name);
        let is_power_rail = matches!(signal_type, SignalType::Power | SignalType::Ground);
        
        UcsNet {
            net_name: net.name.clone(),
            voltage_level: self.infer_voltage_from_name(&net.name),
            is_power_rail,
            signal_type,
            connections: net.connections.iter().map(|c| {
                NetConnection::new(&c.component_ref, &c.pin_number)
            }).collect(),
            attributes: HashMap::new(),
        }
    }
    
    /// Add nets from labels that might not have explicit net entries
    fn add_nets_from_labels(&self, labels: &[Label], ucs: &mut UnifiedCircuitSchema) {
        for label in labels {
            let net_name = match label.label_type {
                LabelType::Global => label.text.clone(),
                LabelType::Local => format!("Net-({})", label.text),
                LabelType::Hierarchical => format!("Hier-{}", label.text),
            };
            
            // Check if this net already exists
            if ucs.get_net(&net_name).is_none() {
                let signal_type = SignalType::from_net_name(&net_name);
                let is_power_rail = matches!(signal_type, SignalType::Power | SignalType::Ground);
                
                ucs.add_net(UcsNet {
                    net_name,
                    voltage_level: self.infer_voltage_from_name(&label.text),
                    is_power_rail,
                    signal_type,
                    connections: Vec::new(),
                    attributes: HashMap::new(),
                });
            }
        }
    }
    
    /// Infer voltage from net name
    fn infer_voltage_from_name(&self, name: &str) -> Option<f64> {
        let upper = name.to_uppercase();
        
        if upper.contains("GND") || upper.contains("VSS") || upper == "0V" {
            Some(0.0)
        } else if upper.contains("3V3") || upper.contains("3.3V") {
            Some(3.3)
        } else if upper.contains("5V") && !upper.contains("12V") {
            Some(5.0)
        } else if upper.contains("12V") {
            Some(12.0)
        } else if upper.contains("1V8") || upper.contains("1.8V") {
            Some(1.8)
        } else if upper.contains("2V5") || upper.contains("2.5V") {
            Some(2.5)
        } else {
            None
        }
    }
}

impl Default for KicadAdapter {
    fn default() -> Self {
        Self::new()
    }
}

impl CircuitAdapter for KicadAdapter {
    fn source_cad(&self) -> SourceCAD {
        SourceCAD::KiCad
    }
    
    fn supported_extensions(&self) -> &[&str] {
        // Support both legacy (KiCad 4-5) and modern (KiCad 6+) formats
        &["kicad_sch", "sch"]
    }
    
    fn parse_file(&self, path: &Path) -> Result<UnifiedCircuitSchema, AdapterError> {
        let schematic = KicadParser::parse_schematic(path)
            .map_err(|e| AdapterError::Parse(e.to_string()))?;
        
        Ok(self.convert_schematic(schematic))
    }
    
    fn parse_string(&self, content: &str, filename: &str) -> Result<UnifiedCircuitSchema, AdapterError> {
        let schematic = KicadParser::parse_schematic_str(content, filename)
            .map_err(|e| AdapterError::Parse(e.to_string()))?;
        
        Ok(self.convert_schematic(schematic))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_adapter_extensions() {
        let adapter = KicadAdapter::new();
        assert!(adapter.supported_extensions().contains(&"kicad_sch"));
    }
    
    #[test]
    fn test_can_handle() {
        let adapter = KicadAdapter::new();
        
        assert!(adapter.can_handle(Path::new("test.kicad_sch")));
        assert!(adapter.can_handle(Path::new("/path/to/project.kicad_sch")));
        assert!(adapter.can_handle(Path::new("test.sch"))); // Legacy format supported
        assert!(adapter.can_handle(Path::new("/path/to/project.sch"))); // Legacy format supported
        assert!(!adapter.can_handle(Path::new("test.txt")));
    }
    
    #[test]
    fn test_infer_voltage() {
        let adapter = KicadAdapter::new();
        
        assert_eq!(adapter.infer_voltage_from_name("GND"), Some(0.0));
        assert_eq!(adapter.infer_voltage_from_name("3V3"), Some(3.3));
        assert_eq!(adapter.infer_voltage_from_name("VCC_5V"), Some(5.0));
        assert_eq!(adapter.infer_voltage_from_name("12V_RAIL"), Some(12.0));
        assert_eq!(adapter.infer_voltage_from_name("SDA"), None);
    }
    
    #[test]
    fn test_signal_type_inference() {
        assert_eq!(SignalType::from_net_name("GND"), SignalType::Ground);
        assert_eq!(SignalType::from_net_name("VCC"), SignalType::Power);
        assert_eq!(SignalType::from_net_name("CLK"), SignalType::Clock);
        assert_eq!(SignalType::from_net_name("NRST"), SignalType::Reset);
        assert_eq!(SignalType::from_net_name("SDA"), SignalType::Data);
    }
}
