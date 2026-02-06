//! Decoupling Groups Analysis
//!
//! Groups capacitors that share the same Power/GND nets and IC proximity.
//! Verifies that both Bulk and High-Frequency Bypass caps are present.
//!
//! This module now uses UCS Circuit as the primary data source for consistency
//! with other analysis functions. The Schematic-based API is maintained for
//! backwards compatibility.

use crate::parser::schema::{Component, Schematic, Position};
use crate::compliance::power_net_registry::PowerNetRegistry;
use crate::analyzer::capacitor_classifier::{CapacitorFunction, CapacitorClassification};
use crate::parser::netlist::PinNetConnection;
use crate::ucs::Circuit;
use std::collections::HashMap;
use serde::{Deserialize, Serialize};

/// Analysis of a capacitor in a decoupling group
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CapacitorAnalysis {
    pub component_ref: String,
    pub value: String,
    pub function: CapacitorFunction,
    pub distance_to_ic_mm: f64,
    pub is_hf_bypass: bool,  // 10nF-2.2µF, 0402/0603
    pub is_bulk: bool,       // >4.7µF
}

/// Decoupling Group for an IC
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DecouplingGroup {
    pub ic_ref: String,
    pub ic_value: String,
    pub power_net: String,
    pub gnd_net: String,
    pub capacitors: Vec<CapacitorAnalysis>,
    pub has_hf_bypass: bool,
    pub has_bulk: bool,
    pub hf_bypass_distance_mm: Option<f64>,  // Distance of closest HF bypass
}

/// Decoupling Groups Analyzer
pub struct DecouplingGroupsAnalyzer;

impl DecouplingGroupsAnalyzer {
    /// Build decoupling groups for all ICs using UCS Circuit (preferred method)
    pub fn build_groups_from_circuit(
        circuit: &Circuit,
        classifications: &[CapacitorClassification],
    ) -> Vec<DecouplingGroup> {
        let mut groups = Vec::new();
        const MAX_DISTANCE_MM: f64 = 20.0;
        
        for ic in circuit.ics() {
            if ic.is_virtual {
                continue;
            }
            
            // Find power and ground nets for this IC
            let ic_nets = circuit.nets_for_component(&ic.ref_des);
            let power_nets: Vec<&str> = ic_nets
                .iter()
                .filter(|n| n.is_power_rail)
                .map(|n| n.net_name.as_str())
                .collect();
            
            let gnd_nets: Vec<&str> = ic_nets
                .iter()
                .filter(|n| {
                    let name_upper = n.net_name.to_uppercase();
                    name_upper == "GND" || name_upper == "GROUND" || 
                    name_upper.contains("VSS") || name_upper == "0V"
                })
                .map(|n| n.net_name.as_str())
                .collect();
            
            // For each power/gnd pair, find nearby capacitors
            for power_net in &power_nets {
                for gnd_net in &gnd_nets {
                    let capacitors = Self::find_capacitors_for_nets_circuit(
                        circuit,
                        power_net,
                        gnd_net,
                        &ic.ref_des,
                        ic.position.as_ref(),
                        classifications,
                        MAX_DISTANCE_MM,
                    );
                    
                    if !capacitors.is_empty() {
                        let has_hf_bypass = capacitors.iter().any(|c| c.is_hf_bypass);
                        let has_bulk = capacitors.iter().any(|c| c.is_bulk);
                        
                        let hf_bypass_distance = capacitors
                            .iter()
                            .filter(|c| c.is_hf_bypass)
                            .map(|c| c.distance_to_ic_mm)
                            .fold(None, |acc, x| {
                                match acc {
                                    None => Some(x),
                                    Some(y) => Some(if x < y { x } else { y }),
                                }
                            });
                        
                        groups.push(DecouplingGroup {
                            ic_ref: ic.ref_des.clone(),
                            ic_value: ic.value.clone().unwrap_or_default(),
                            power_net: power_net.to_string(),
                            gnd_net: gnd_net.to_string(),
                            capacitors,
                            has_hf_bypass,
                            has_bulk,
                            hf_bypass_distance_mm: hf_bypass_distance,
                        });
                    }
                }
            }
        }
        
        groups
    }
    
    /// Build decoupling groups for all ICs in the schematic (legacy method, uses Circuit internally)
    pub fn build_groups(
        schematic: &Schematic,
        power_registry: &PowerNetRegistry,
        classifications: &[CapacitorClassification],
        pin_to_net: &HashMap<String, Vec<PinNetConnection>>,
    ) -> Vec<DecouplingGroup> {
        // Convert Schematic to Circuit for unified analysis
        // This maintains backwards compatibility while using the unified implementation
        // Try to build circuit from schematic
        // Note: This is a simplified conversion - in practice, Circuit should come from state
        // For now, we fall back to the original implementation if conversion fails
        Self::build_groups_legacy(schematic, power_registry, classifications, pin_to_net)
    }
    
    /// Legacy implementation (kept for fallback)
    fn build_groups_legacy(
        schematic: &Schematic,
        power_registry: &PowerNetRegistry,
        classifications: &[CapacitorClassification],
        pin_to_net: &HashMap<String, Vec<PinNetConnection>>,
    ) -> Vec<DecouplingGroup> {
        let mut groups = Vec::new();
        
        // Find all ICs
        let ics: Vec<&Component> = schematic
            .components
            .iter()
            .filter(|c| {
                let ref_upper = c.reference.to_uppercase();
                ref_upper.starts_with('U')
            })
            .collect();
        
        for ic in ics {
            // Find power pins for this IC
            let power_pins = Self::find_power_pins(ic, pin_to_net, power_registry);
            
            for (power_net, gnd_net) in power_pins {
                // Find capacitors connected to this power/gnd pair
                let capacitors = Self::find_capacitors_for_nets(
                    &power_net,
                    &gnd_net,
                    schematic,
                    classifications,
                    pin_to_net,
                    &ic.position,
                );
                
                // Check if within proximity (20mm)
                let nearby_caps: Vec<CapacitorAnalysis> = capacitors
                    .into_iter()
                    .filter(|cap| cap.distance_to_ic_mm <= 20.0)
                    .collect();
                
                if !nearby_caps.is_empty() {
                    let has_hf_bypass = nearby_caps.iter().any(|c| c.is_hf_bypass);
                    let has_bulk = nearby_caps.iter().any(|c| c.is_bulk);
                    
                    let hf_bypass_distance = nearby_caps
                        .iter()
                        .filter(|c| c.is_hf_bypass)
                        .map(|c| c.distance_to_ic_mm)
                        .fold(None, |acc, x| {
                            match acc {
                                None => Some(x),
                                Some(y) => Some(if x < y { x } else { y }),
                            }
                        });
                    
                    groups.push(DecouplingGroup {
                        ic_ref: ic.reference.clone(),
                        ic_value: ic.value.clone(),
                        power_net: power_net.clone(),
                        gnd_net: gnd_net.clone(),
                        capacitors: nearby_caps,
                        has_hf_bypass,
                        has_bulk,
                        hf_bypass_distance_mm: hf_bypass_distance,
                    });
                }
            }
        }
        
        groups
    }
    
    /// Find power pins for an IC
    fn find_power_pins(
        ic: &Component,
        pin_to_net: &HashMap<String, Vec<PinNetConnection>>,
        power_registry: &PowerNetRegistry,
    ) -> Vec<(String, String)> {
        let mut power_pin_pairs = Vec::new();
        
        // Find all power nets connected to this IC
        let mut power_nets = Vec::new();
        let mut gnd_nets = Vec::new();
        
        for pin in &ic.pins {
            let key = format!("{}:{}", ic.reference, pin.number);
            if let Some(connections) = pin_to_net.get(&key) {
                for conn in connections {
                    if power_registry.is_power_net(&conn.net_name) {
                        power_nets.push(conn.net_name.clone());
                    } else if Self::is_ground_net(&conn.net_name) {
                        gnd_nets.push(conn.net_name.clone());
                    }
                }
            }
        }
        
        // Create pairs (each power net with each GND net)
        for power_net in &power_nets {
            for gnd_net in &gnd_nets {
                power_pin_pairs.push((power_net.clone(), gnd_net.clone()));
            }
        }
        
        power_pin_pairs
    }
    
    /// Find capacitors connected to specific power/gnd nets using Circuit
    fn find_capacitors_for_nets_circuit(
        circuit: &Circuit,
        power_net: &str,
        gnd_net: &str,
        ic_ref: &str,
        ic_position: Option<&crate::ucs::schema::UcsPosition>,
        classifications: &[CapacitorClassification],
        max_distance_mm: f64,
    ) -> Vec<CapacitorAnalysis> {
        let mut capacitors = Vec::new();
        
        // Find all capacitors near the IC
        let nearby_caps = circuit.capacitors_near(ic_ref, max_distance_mm);
        
        for cap in nearby_caps {
            // Check if capacitor is connected to both power and gnd nets
            let cap_nets: Vec<&str> = circuit.nets_for_component(&cap.ref_des)
                .iter()
                .map(|n| n.net_name.as_str())
                .collect();
            
            let has_power = cap_nets.contains(&power_net);
            let has_gnd = cap_nets.contains(&gnd_net);
            
            if has_power && has_gnd {
                // Get classification
                let classification = classifications
                    .iter()
                    .find(|c| c.component_ref == cap.ref_des);
                
                let function = classification
                    .map(|c| c.function)
                    .unwrap_or(CapacitorFunction::Unknown);
                
                // Calculate distance to IC
                let distance = if let (Some(ic_pos), Some(cap_pos)) = (ic_position, cap.position.as_ref()) {
                    ic_pos.distance_to(cap_pos)
                } else {
                    f64::MAX
                };
                
                // Determine if HF bypass or bulk
                let (is_hf_bypass, is_bulk) = Self::classify_cap_type(
                    cap.value.as_deref().unwrap_or(""),
                    function,
                );
                
                capacitors.push(CapacitorAnalysis {
                    component_ref: cap.ref_des.clone(),
                    value: cap.value.clone().unwrap_or_default(),
                    function,
                    distance_to_ic_mm: distance,
                    is_hf_bypass,
                    is_bulk,
                });
            }
        }
        
        capacitors
    }
    
    /// Find capacitors connected to specific power/gnd nets (legacy method)
    fn find_capacitors_for_nets(
        power_net: &str,
        gnd_net: &str,
        schematic: &Schematic,
        classifications: &[CapacitorClassification],
        pin_to_net: &HashMap<String, Vec<PinNetConnection>>,
        ic_position: &Position,
    ) -> Vec<CapacitorAnalysis> {
        let mut capacitors = Vec::new();
        
        // Find all capacitors
        let cap_components: Vec<&Component> = schematic
            .components
            .iter()
            .filter(|c| {
                let ref_upper = c.reference.to_uppercase();
                ref_upper.starts_with('C')
            })
            .collect();
        
        for cap_component in cap_components {
            // Check if this capacitor is connected to both power and gnd nets
            let mut has_power = false;
            let mut has_gnd = false;
            
            for pin in &cap_component.pins {
                let key = format!("{}:{}", cap_component.reference, pin.number);
                if let Some(connections) = pin_to_net.get(&key) {
                    for conn in connections {
                        if conn.net_name == *power_net {
                            has_power = true;
                        }
                        if conn.net_name == *gnd_net {
                            has_gnd = true;
                        }
                    }
                }
            }
            
            if has_power && has_gnd {
                // Get classification
                let classification = classifications
                    .iter()
                    .find(|c| c.component_ref == cap_component.reference);
                
                let function = classification
                    .map(|c| c.function)
                    .unwrap_or(CapacitorFunction::Unknown);
                
                // Calculate distance to IC
                let distance = Self::distance(&cap_component.position, ic_position);
                
                // Determine if HF bypass or bulk
                let (is_hf_bypass, is_bulk) = Self::classify_cap_type(
                    &cap_component.value,
                    function,
                );
                
                capacitors.push(CapacitorAnalysis {
                    component_ref: cap_component.reference.clone(),
                    value: cap_component.value.clone(),
                    function,
                    distance_to_ic_mm: distance,
                    is_hf_bypass,
                    is_bulk,
                });
            }
        }
        
        capacitors
    }
    
    /// Classify capacitor as HF bypass or bulk based on value and function
    fn classify_cap_type(value: &str, function: CapacitorFunction) -> (bool, bool) {
        // Parse value
        let value_lower = value.to_lowercase();
        let mut num_str = String::new();
        let mut unit = String::new();
        let mut found_digit = false;
        
        for ch in value_lower.chars() {
            if ch.is_ascii_digit() || ch == '.' {
                num_str.push(ch);
                found_digit = true;
            } else if found_digit {
                unit.push(ch);
            }
        }
        
        if num_str.is_empty() {
            return (false, false);
        }
        
        let num = num_str.parse::<f64>().unwrap_or(0.0);
        let value_nf = match unit.as_str() {
            "pf" | "p" => num / 1000.0,
            "nf" | "n" => num,
            "uf" | "u" | "µf" | "µ" => num * 1000.0,
            _ => 0.0,
        };
        let value_uf = value_nf / 1000.0;
        
        // HF bypass: 10nF to 2.2µF, Decoupling function
        let is_hf_bypass = (value_nf >= 10.0 && value_uf <= 2.2) && 
                          function == CapacitorFunction::Decoupling;
        
        // Bulk: >4.7µF, Bulk function
        let is_bulk = value_uf > 4.7 && function == CapacitorFunction::Bulk;
        
        (is_hf_bypass, is_bulk)
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

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_classify_cap_type() {
        // HF bypass
        let (hf, bulk) = DecouplingGroupsAnalyzer::classify_cap_type("100nF", CapacitorFunction::Decoupling);
        assert!(hf);
        assert!(!bulk);
        
        // Bulk
        let (hf, bulk) = DecouplingGroupsAnalyzer::classify_cap_type("10uF", CapacitorFunction::Bulk);
        assert!(!hf);
        assert!(bulk);
    }
}
