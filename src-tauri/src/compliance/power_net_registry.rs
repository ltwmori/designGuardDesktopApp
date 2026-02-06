//! Power Net Registry
//!
//! Reliably identifies Power Nets using keyword patterns and regulator output tracing.
//! This is a critical prerequisite for capacitor classification.

use crate::parser::schema::{Schematic, Component, Net};
use crate::ai::classifier::ComponentRole;
use std::collections::HashMap;
use serde::{Deserialize, Serialize};

/// Source of power net identification
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PowerNetSource {
    Keyword,        // Identified by keyword pattern
    RegulatorOutput, // Connected to regulator output pin
    UserDefined,    // Explicitly marked by user
}

/// Power Net Registry entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PowerNetEntry {
    pub net_name: String,
    pub is_power_net: bool,
    pub source: PowerNetSource,
    pub voltage: Option<f64>, // Inferred voltage level (V)
}

/// Power Net Registry
pub struct PowerNetRegistry {
    registry: HashMap<String, PowerNetEntry>,
}

impl PowerNetRegistry {
    /// Create a new Power Net Registry and build it from schematic
    pub fn new(schematic: &Schematic) -> Self {
        let mut registry = Self {
            registry: HashMap::new(),
        };
        
        registry.build_registry(schematic);
        registry
    }
    
    /// Build the registry from schematic
    fn build_registry(&mut self, schematic: &Schematic) {
        // Step 1: Keyword-based detection
        for net in &schematic.nets {
            if let Some(entry) = Self::check_keyword_patterns(&net.name) {
                self.registry.insert(net.name.clone(), entry);
            }
        }
        
        // Also check labels
        for label in &schematic.labels {
            let net_name = match &label.label_type {
                crate::parser::schema::LabelType::Global => label.text.clone(),
                crate::parser::schema::LabelType::Local => format!("Net-({})", label.text),
                crate::parser::schema::LabelType::Hierarchical => format!("Hier-{}", label.text),
            };
            
            if let Some(entry) = Self::check_keyword_patterns(&net_name) {
                self.registry.insert(net_name, entry);
            }
        }
        
        // Step 2: Regulator output tracing
        self.trace_regulator_outputs(schematic);
    }
    
    /// Check if a net name matches power net keyword patterns
    fn check_keyword_patterns(net_name: &str) -> Option<PowerNetEntry> {
        let upper = net_name.to_uppercase();
        
        // Explicit power patterns
        let power_patterns = [
            "VCC", "VDD", "VBAT", "VBUS", "VIN", "VOUT",
            "AVCC", "AVDD", "DVCC", "DVDD", "PVCC", "PVDD",
            "VCCA", "VCCD", "VCCIO", "VDDIO",
            "V_CORE", "VCORE", "VREF", "VDDA", "VSSA",
        ];
        
        for pattern in &power_patterns {
            if upper.contains(pattern) {
                let voltage = Self::extract_voltage(net_name);
                return Some(PowerNetEntry {
                    net_name: net_name.to_string(),
                    is_power_net: true,
                    source: PowerNetSource::Keyword,
                    voltage,
                });
            }
        }
        
        // Voltage patterns: +5V, +12V, 3V3, 1.8V, 2.5V, etc.
        if upper.starts_with('+') || upper.contains("3V3") || upper.contains("3.3V") {
            let voltage = Self::extract_voltage(net_name);
            if voltage.is_some() {
                return Some(PowerNetEntry {
                    net_name: net_name.to_string(),
                    is_power_net: true,
                    source: PowerNetSource::Keyword,
                    voltage,
                });
            }
        }
        
        // Pattern matching for voltage values
        if let Some(voltage) = Self::extract_voltage(net_name) {
            if voltage > 0.0 && voltage <= 50.0 {
                // Reasonable voltage range
                return Some(PowerNetEntry {
                    net_name: net_name.to_string(),
                    is_power_net: true,
                    source: PowerNetSource::Keyword,
                    voltage: Some(voltage),
                });
            }
        }
        
        None
    }
    
    /// Extract voltage from net name
    fn extract_voltage(net_name: &str) -> Option<f64> {
        let upper = net_name.to_uppercase();
        
        // Patterns: 3V3, 3.3V
        if upper.contains("3V3") || upper.contains("3.3V") {
            return Some(3.3);
        }
        
        // Try to find voltage pattern: number followed by V
        // Simple parsing without regex
        let v_pos = upper.find('V')?;
        if v_pos == 0 {
            return None;
        }
        
        // Look backwards from V to find number
        let mut end = v_pos;
        let mut start = v_pos;
        
        // Skip + sign if present
        if end > 0 && upper.as_bytes()[end - 1] == b'+' {
            end -= 1;
            if end == 0 {
                return None;
            }
        }
        
        // Find start of number
        while start > 0 {
            let ch = upper.as_bytes()[start - 1];
            if ch.is_ascii_digit() || ch == b'.' {
                start -= 1;
            } else {
                break;
            }
        }
        
        if start < end {
            let num_str = &upper[start..end];
            return num_str.parse::<f64>().ok();
        }
        
        None
    }
    
    /// Trace regulator outputs to identify power nets
    fn trace_regulator_outputs(&mut self, schematic: &Schematic) {
        // Identify voltage regulators
        let regulators: Vec<&Component> = schematic
            .components
            .iter()
            .filter(|c| Self::is_regulator(c))
            .collect();
        
        // For each regulator, find output pins and mark connected nets as power
        for regulator in regulators {
            let output_pins = Self::find_regulator_output_pins(regulator);
            
            // Find nets connected to these output pins
            for net in &schematic.nets {
                for connection in &net.connections {
                    if connection.component_ref == regulator.reference {
                        if output_pins.contains(&connection.pin_number) {
                            // This net is connected to a regulator output
                            self.registry.insert(net.name.clone(), PowerNetEntry {
                                net_name: net.name.clone(),
                                is_power_net: true,
                                source: PowerNetSource::RegulatorOutput,
                                voltage: None, // Could be inferred from regulator type
                            });
                        }
                    }
                }
            }
        }
    }
    
    /// Check if a component is a voltage regulator
    fn is_regulator(component: &Component) -> bool {
        let value_upper = component.value.to_uppercase();
        let ref_upper = component.reference.to_uppercase();
        
        // Part number patterns
        let regulator_patterns = [
            "LM7805", "LM7812", "LM7809", "LM7815",
            "LM1117", "AMS1117", "LD1117",
            "LM317", "LM2596", "MP1584",
            "TPS54", "TPS62", "TPS63",
            "LTC3", "LTC4",
        ];
        
        for pattern in &regulator_patterns {
            if value_upper.contains(pattern) {
                return true;
            }
        }
        
        // Library ID patterns
        let lib_upper = component.lib_id.to_uppercase();
        if lib_upper.contains("REGULATOR") || lib_upper.contains("LDO") {
            return true;
        }
        
        // Reference designator (U for IC, but check value)
        if ref_upper.starts_with('U') {
            if value_upper.contains("REG") || value_upper.contains("LDO") {
                return true;
            }
        }
        
        false
    }
    
    /// Find output pins of a regulator
    /// Typical output pin names: VOUT, OUT, VDD, VCC (when component is regulator)
    fn find_regulator_output_pins(regulator: &Component) -> Vec<String> {
        let mut output_pins = Vec::new();
        
        // Common output pin names
        let output_patterns = ["VOUT", "OUT", "VDD", "VCC", "OUTPUT"];
        
        // Check pin numbers/names (if available in properties or pin names)
        for pin in &regulator.pins {
            let pin_upper = pin.number.to_uppercase();
            for pattern in &output_patterns {
                if pin_upper.contains(pattern) {
                    output_pins.push(pin.number.clone());
                }
            }
        }
        
        // If no pattern match, use heuristics:
        // For 3-pin regulators (7805, 1117), pin 3 is typically output
        // For SOT-223, pin 2 or 3 might be output
        if output_pins.is_empty() {
            // Default: assume pin "2" or "3" for common regulators
            if regulator.pins.len() >= 2 {
                output_pins.push("2".to_string());
            }
            if regulator.pins.len() >= 3 {
                output_pins.push("3".to_string());
            }
        }
        
        output_pins
    }
    
    /// Check if a net is a power net
    pub fn is_power_net(&self, net_name: &str) -> bool {
        self.registry
            .get(net_name)
            .map(|e| e.is_power_net)
            .unwrap_or(false)
    }
    
    /// Get voltage level for a power net (if known)
    pub fn get_voltage(&self, net_name: &str) -> Option<f64> {
        self.registry
            .get(net_name)
            .and_then(|e| e.voltage)
    }
    
    /// Get all power nets
    pub fn power_nets(&self) -> Vec<&str> {
        self.registry
            .iter()
            .filter(|(_, e)| e.is_power_net)
            .map(|(name, _)| name.as_str())
            .collect()
    }
    
    /// Get entry for a net
    pub fn get_entry(&self, net_name: &str) -> Option<&PowerNetEntry> {
        self.registry.get(net_name)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_keyword_detection() {
        let entry = PowerNetRegistry::check_keyword_patterns("VCC");
        assert!(entry.is_some());
        assert!(entry.unwrap().is_power_net);
        
        let entry = PowerNetRegistry::check_keyword_patterns("3V3");
        assert!(entry.is_some());
        assert!(entry.unwrap().is_power_net);
        
        let entry = PowerNetRegistry::check_keyword_patterns("GND");
        assert!(entry.is_none());
    }
    
    #[test]
    fn test_voltage_extraction() {
        assert_eq!(PowerNetRegistry::extract_voltage("3V3"), Some(3.3));
        assert_eq!(PowerNetRegistry::extract_voltage("+5V"), Some(5.0));
        assert_eq!(PowerNetRegistry::extract_voltage("1.8V"), Some(1.8));
        assert_eq!(PowerNetRegistry::extract_voltage("VCC"), None);
    }
}
