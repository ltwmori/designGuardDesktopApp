//! Capacitor Function Classifier
//!
//! Classifies capacitors by their function: Decoupling, Bulk, Filtering, Timing, or Snubber.
//! Uses sharpened heuristics based on net connectivity, value, footprint, and proximity.

use crate::parser::schema::{Component, Schematic, Position};
use crate::compliance::power_net_registry::PowerNetRegistry;
use crate::parser::netlist::PinNetConnection;
use std::collections::HashMap;
use serde::{Deserialize, Serialize};

/// Capacitor function classification
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CapacitorFunction {
    Decoupling,  // High-frequency bypass (10nF-2.2µF, Power↔GND)
    Bulk,        // Energy storage (>4.7µF, Power↔GND)
    Filtering,   // Signal filtering (In-Series or Low-Pass)
    Timing,      // Crystal load caps (1pF-47pF, XTAL↔GND)
    Snubber,     // Switch node snubber (high voltage, SW↔GND)
    Unknown,     // Cannot determine function
}

/// Classification result with confidence
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CapacitorClassification {
    pub component_ref: String,
    pub function: CapacitorFunction,
    pub confidence: f64,  // 0.0 to 1.0
    pub reasoning: String,
}

/// Capacitor Classifier
pub struct CapacitorClassifier;

impl CapacitorClassifier {
    /// Classify all capacitors in a schematic
    pub fn classify_capacitors(
        schematic: &Schematic,
        power_registry: &PowerNetRegistry,
        pin_to_net: &HashMap<String, Vec<PinNetConnection>>,
    ) -> Vec<CapacitorClassification> {
        let mut results = Vec::new();
        
        // Find all capacitors
        let capacitors: Vec<&Component> = schematic
            .components
            .iter()
            .filter(|c| {
                let ref_upper = c.reference.to_uppercase();
                ref_upper.starts_with('C')
            })
            .collect();
        
        for capacitor in capacitors {
            if let Some(classification) = Self::classify_capacitor(
                capacitor,
                schematic,
                power_registry,
                pin_to_net,
            ) {
                results.push(classification);
            }
        }
        
        results
    }
    
    /// Classify a single capacitor
    fn classify_capacitor(
        capacitor: &Component,
        schematic: &Schematic,
        power_registry: &PowerNetRegistry,
        pin_to_net: &HashMap<String, Vec<PinNetConnection>>,
    ) -> Option<CapacitorClassification> {
        // Get nets connected to this capacitor's pins
        let mut nets = Vec::new();
        for pin in &capacitor.pins {
            let key = format!("{}:{}", capacitor.reference, pin.number);
            if let Some(connections) = pin_to_net.get(&key) {
                for conn in connections {
                    nets.push(conn.net_name.clone());
                }
            }
        }
        
        if nets.is_empty() {
            return None;
        }
        
        // Parse capacitor value
        let (value_f, unit) = Self::parse_capacitor_value(&capacitor.value)?;
        let value_pf = Self::to_picofarads(value_f, &unit);
        
        // Get footprint size
        let footprint_size = Self::get_footprint_size(capacitor.footprint.as_deref());
        
        // Check each classification in order of specificity
        if let Some(result) = Self::check_timing(capacitor, &nets, value_pf, schematic) {
            return Some(result);
        }
        
        if let Some(result) = Self::check_snubber(capacitor, &nets, value_pf, schematic) {
            return Some(result);
        }
        
        if let Some(result) = Self::check_filtering(capacitor, &nets, value_pf, power_registry, schematic) {
            return Some(result);
        }
        
        if let Some(result) = Self::check_bulk(capacitor, &nets, value_pf, power_registry, footprint_size) {
            return Some(result);
        }
        
        if let Some(result) = Self::check_decoupling(capacitor, &nets, value_pf, power_registry, footprint_size) {
            return Some(result);
        }
        
        // Default to Unknown
        Some(CapacitorClassification {
            component_ref: capacitor.reference.clone(),
            function: CapacitorFunction::Unknown,
            confidence: 0.0,
            reasoning: format!("Cannot determine function for {} ({}). Nets: {:?}", 
                capacitor.reference, capacitor.value, nets),
        })
    }
    
    /// Check if capacitor is a timing (crystal load) cap
    fn check_timing(
        capacitor: &Component,
        nets: &[String],
        value_pf: f64,
        schematic: &Schematic,
    ) -> Option<CapacitorClassification> {
        // Timing caps: 1pF to 47pF, connected to crystal pins or GND
        if value_pf < 1.0 || value_pf > 47.0 {
            return None;
        }
        
        // Check if connected to crystal-related nets
        let mut has_xtal_net = false;
        let mut has_gnd = false;
        
        for net in nets {
            let net_upper = net.to_uppercase();
            if net_upper.contains("XTAL") || net_upper.contains("OSC") || 
               net_upper.contains("CRYSTAL") || net_upper.contains("CLK") {
                has_xtal_net = true;
            }
            if net_upper == "GND" || net_upper.contains("VSS") || net_upper == "GROUND" {
                has_gnd = true;
            }
        }
        
        // Also check proximity to crystal
        let mut near_crystal = false;
        for component in &schematic.components {
            let ref_upper = component.reference.to_uppercase();
            if ref_upper.starts_with('Y') || ref_upper.starts_with('X') {
                let distance = Self::distance(&capacitor.position, &component.position);
                if distance < 30.0 {  // Within 30mm
                    near_crystal = true;
                    break;
                }
            }
        }
        
        if (has_xtal_net && has_gnd) || (near_crystal && has_gnd && value_pf >= 10.0 && value_pf <= 33.0) {
            return Some(CapacitorClassification {
                component_ref: capacitor.reference.clone(),
                function: CapacitorFunction::Timing,
                confidence: if has_xtal_net { 0.95 } else { 0.7 },
                reasoning: format!("Timing cap: {}pF, near crystal or connected to XTAL net", value_pf),
            });
        }
        
        None
    }
    
    /// Check if capacitor is a snubber
    fn check_snubber(
        capacitor: &Component,
        nets: &[String],
        value_pf: f64,
        schematic: &Schematic,
    ) -> Option<CapacitorClassification> {
        // Snubber: connected to switch node and GND, high voltage rating
        let mut has_switch_node = false;
        let mut has_gnd = false;
        
        for net in nets {
            let net_upper = net.to_uppercase();
            if net_upper.contains("SW") || net_upper.contains("SWITCH") {
                has_switch_node = true;
            }
            if net_upper == "GND" || net_upper.contains("VSS") {
                has_gnd = true;
            }
        }
        
        // Check proximity to MOSFETs or inductors
        let mut near_switch = false;
        for component in &schematic.components {
            let ref_upper = component.reference.to_uppercase();
            if ref_upper.starts_with('Q') || ref_upper.starts_with('L') {
                let distance = Self::distance(&capacitor.position, &component.position);
                if distance < 20.0 {
                    near_switch = true;
                    break;
                }
            }
        }
        
        if (has_switch_node && has_gnd) || (near_switch && has_gnd) {
            // Check for high voltage rating (would need to parse from properties)
            // For now, assume snubber if connected to switch node
            return Some(CapacitorClassification {
                component_ref: capacitor.reference.clone(),
                function: CapacitorFunction::Snubber,
                confidence: if has_switch_node { 0.9 } else { 0.6 },
                reasoning: format!("Snubber: connected to switch node or near MOSFET/inductor"),
            });
        }
        
        None
    }
    
    /// Check if capacitor is for filtering
    fn check_filtering(
        capacitor: &Component,
        nets: &[String],
        value_pf: f64,
        power_registry: &PowerNetRegistry,
        schematic: &Schematic,
    ) -> Option<CapacitorClassification> {
        if nets.len() < 2 {
            return None;
        }
        
        let net1 = &nets[0];
        let net2 = &nets[1];
        
        // In-Series: Signal_In ↔ Signal_Out (both non-power, non-ground)
        let net1_is_signal = !power_registry.is_power_net(net1) && 
                            !Self::is_ground_net(net1);
        let net2_is_signal = !power_registry.is_power_net(net2) && 
                            !Self::is_ground_net(net2);
        
        if net1_is_signal && net2_is_signal {
            return Some(CapacitorClassification {
                component_ref: capacitor.reference.clone(),
                function: CapacitorFunction::Filtering,
                confidence: 0.8,
                reasoning: format!("Filtering (in-series): between signal nets {} and {}", net1, net2),
            });
        }
        
        // Low Pass: Signal ↔ GND
        if (net1_is_signal && Self::is_ground_net(net2)) ||
           (net2_is_signal && Self::is_ground_net(net1)) {
            // Check proximity to connectors or ADCs
            let mut near_connector_or_adc = false;
            for component in &schematic.components {
                let ref_upper = component.reference.to_uppercase();
                if ref_upper.starts_with('J') || ref_upper.starts_with('P') || 
                   ref_upper.starts_with('C') && ref_upper.contains("CN") {
                    let distance = Self::distance(&capacitor.position, &component.position);
                    if distance < 20.0 {
                        near_connector_or_adc = true;
                        break;
                    }
                }
                if ref_upper.starts_with('U') {
                    let value_upper = component.value.to_uppercase();
                    if value_upper.contains("ADC") {
                        let distance = Self::distance(&capacitor.position, &component.position);
                        if distance < 20.0 {
                            near_connector_or_adc = true;
                            break;
                        }
                    }
                }
            }
            
            return Some(CapacitorClassification {
                component_ref: capacitor.reference.clone(),
                function: CapacitorFunction::Filtering,
                confidence: if near_connector_or_adc { 0.85 } else { 0.65 },
                reasoning: format!("Filtering (low-pass): {} to GND", 
                    if net1_is_signal { net1 } else { net2 }),
            });
        }
        
        None
    }
    
    /// Check if capacitor is bulk
    fn check_bulk(
        capacitor: &Component,
        nets: &[String],
        value_pf: f64,
        power_registry: &PowerNetRegistry,
        footprint_size: Option<FootprintSize>,
    ) -> Option<CapacitorClassification> {
        if nets.len() < 2 {
            return None;
        }
        
        let value_uf = value_pf / 1_000_000.0;
        
        // Bulk: >4.7µF, Power ↔ GND
        if value_uf < 4.7 {
            return None;
        }
        
        let net1 = &nets[0];
        let net2 = &nets[1];
        
        let has_power = power_registry.is_power_net(net1) || power_registry.is_power_net(net2);
        let has_gnd = Self::is_ground_net(net1) || Self::is_ground_net(net2);
        
        if has_power && has_gnd {
            // Check footprint (0805, 1206, or through-hole)
            let is_large_footprint = matches!(footprint_size, 
                Some(FootprintSize::Large) | Some(FootprintSize::ThroughHole));
            
            return Some(CapacitorClassification {
                component_ref: capacitor.reference.clone(),
                function: CapacitorFunction::Bulk,
                confidence: if is_large_footprint { 0.95 } else { 0.8 },
                reasoning: format!("Bulk cap: {:.1}µF, Power↔GND", value_uf),
            });
        }
        
        None
    }
    
    /// Check if capacitor is decoupling
    fn check_decoupling(
        capacitor: &Component,
        nets: &[String],
        value_pf: f64,
        power_registry: &PowerNetRegistry,
        footprint_size: Option<FootprintSize>,
    ) -> Option<CapacitorClassification> {
        if nets.len() < 2 {
            return None;
        }
        
        let value_nf = value_pf / 1000.0;
        let value_uf = value_pf / 1_000_000.0;
        
        // Decoupling: 10nF to 2.2µF, Power ↔ GND, usually 0402 or 0603
        if value_nf < 10.0 || value_uf > 2.2 {
            return None;
        }
        
        let net1 = &nets[0];
        let net2 = &nets[1];
        
        let has_power = power_registry.is_power_net(net1) || power_registry.is_power_net(net2);
        let has_gnd = Self::is_ground_net(net1) || Self::is_ground_net(net2);
        
        if has_power && has_gnd {
            // Check footprint (0402 or 0603 preferred)
            let is_small_footprint = matches!(footprint_size, 
                Some(FootprintSize::Small) | Some(FootprintSize::Medium));
            
            return Some(CapacitorClassification {
                component_ref: capacitor.reference.clone(),
                function: CapacitorFunction::Decoupling,
                confidence: if is_small_footprint { 0.95 } else { 0.75 },
                reasoning: format!("Decoupling cap: {:.1}nF, Power↔GND", value_nf),
            });
        }
        
        None
    }
    
    /// Parse capacitor value string
    fn parse_capacitor_value(value: &str) -> Option<(f64, String)> {
        let value = value.trim();
        let value_lower = value.to_lowercase();
        
        let mut num_str = String::new();
        let mut unit = String::new();
        let mut found_digit = false;
        
        for ch in value_lower.chars() {
            if ch.is_ascii_digit() || ch == '.' || ch == '-' {
                num_str.push(ch);
                found_digit = true;
            } else if found_digit {
                unit.push(ch);
            }
        }
        
        if num_str.is_empty() {
            return None;
        }
        
        let num = num_str.parse::<f64>().ok()?;
        
        let normalized_unit = match unit.as_str() {
            "pf" | "p" => "pF",
            "nf" | "n" => "nF",
            "uf" | "u" | "µf" | "µ" => "uF",
            "mf" | "m" => "mF",
            "f" => "F",
            _ => &unit,
        };
        
        Some((num, normalized_unit.to_string()))
    }
    
    /// Convert value to picofarads
    fn to_picofarads(value: f64, unit: &str) -> f64 {
        match unit {
            "pF" => value,
            "nF" => value * 1000.0,
            "uF" | "µF" => value * 1_000_000.0,
            "mF" => value * 1_000_000_000.0,
            "F" => value * 1_000_000_000_000.0,
            _ => value, // Assume pF if unknown
        }
    }
    
    /// Get footprint size category
    fn get_footprint_size(footprint: Option<&str>) -> Option<FootprintSize> {
        let footprint = footprint?.to_uppercase();
        
        // Small: 0402, 0201
        if footprint.contains("0402") || footprint.contains("0201") {
            return Some(FootprintSize::Small);
        }
        
        // Medium: 0603
        if footprint.contains("0603") {
            return Some(FootprintSize::Medium);
        }
        
        // Large: 0805, 1206, 1210
        if footprint.contains("0805") || footprint.contains("1206") || footprint.contains("1210") {
            return Some(FootprintSize::Large);
        }
        
        // Through-hole: any through-hole package
        if footprint.contains("TH") || footprint.contains("THT") || 
           footprint.contains("RADIAL") || footprint.contains("AXIAL") {
            return Some(FootprintSize::ThroughHole);
        }
        
        None
    }
    
    /// Check if net is ground
    fn is_ground_net(net: &str) -> bool {
        let upper = net.to_uppercase();
        upper == "GND" || upper == "GROUND" || upper.contains("VSS") || 
        upper == "0V" || upper == "COM" || upper == "COMMON"
    }
    
    /// Calculate distance between two positions
    fn distance(p1: &Position, p2: &Position) -> f64 {
        let dx = p1.x - p2.x;
        let dy = p1.y - p2.y;
        (dx * dx + dy * dy).sqrt()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum FootprintSize {
    Small,        // 0402, 0201
    Medium,       // 0603
    Large,        // 0805, 1206, 1210
    ThroughHole,  // Radial, Axial, etc.
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_parse_capacitor_value() {
        assert_eq!(
            CapacitorClassifier::parse_capacitor_value("100nF"),
            Some((100.0, "nF".to_string()))
        );
        assert_eq!(
            CapacitorClassifier::parse_capacitor_value("2.2uF"),
            Some((2.2, "uF".to_string()))
        );
        assert_eq!(
            CapacitorClassifier::parse_capacitor_value("22pF"),
            Some((22.0, "pF".to_string()))
        );
    }
    
    #[test]
    fn test_to_picofarads() {
        assert_eq!(CapacitorClassifier::to_picofarads(100.0, "nF"), 100_000.0);
        assert_eq!(CapacitorClassifier::to_picofarads(2.2, "uF"), 2_200_000.0);
        assert_eq!(CapacitorClassifier::to_picofarads(22.0, "pF"), 22.0);
    }
    
    #[test]
    fn test_is_ground_net() {
        assert!(CapacitorClassifier::is_ground_net("GND"));
        assert!(CapacitorClassifier::is_ground_net("VSS"));
        assert!(!CapacitorClassifier::is_ground_net("VCC"));
    }
}

// Comprehensive tests
#[cfg(test)]
#[path = "capacitor_classifier_tests.rs"]
mod comprehensive_tests;
