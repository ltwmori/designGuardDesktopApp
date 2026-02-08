//! Circuit Graph Implementation
//!
//! This module provides a graph-based representation of circuits using petgraph.
//! The graph structure enables efficient:
//! - Connectivity analysis
//! - Voltage propagation
//! - Path finding between components
//! - Net traversal

use petgraph::graph::{DiGraph, NodeIndex};
use petgraph::visit::EdgeRef;
use petgraph::Direction;
use std::collections::{HashMap, HashSet};
use serde::{Deserialize, Serialize};

use super::schema::*;

/// Node type in the circuit graph
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CircuitNode {
    /// A component node (IC, resistor, capacitor, etc.)
    Component(UcsComponent),
    
    /// A net node (represents an electrical connection)
    Net(UcsNet),
}

impl CircuitNode {
    pub fn is_component(&self) -> bool {
        matches!(self, CircuitNode::Component(_))
    }
    
    pub fn is_net(&self) -> bool {
        matches!(self, CircuitNode::Net(_))
    }
    
    pub fn as_component(&self) -> Option<&UcsComponent> {
        match self {
            CircuitNode::Component(c) => Some(c),
            _ => None,
        }
    }
    
    pub fn as_net(&self) -> Option<&UcsNet> {
        match self {
            CircuitNode::Net(n) => Some(n),
            _ => None,
        }
    }
    
    pub fn as_component_mut(&mut self) -> Option<&mut UcsComponent> {
        match self {
            CircuitNode::Component(c) => Some(c),
            _ => None,
        }
    }
    
    pub fn as_net_mut(&mut self) -> Option<&mut UcsNet> {
        match self {
            CircuitNode::Net(n) => Some(n),
            _ => None,
        }
    }
}

/// Edge type in the circuit graph - represents a pin connection
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CircuitEdge {
    /// Pin number on the component side
    pub pin_number: String,
    
    /// Pin name (if available)
    pub pin_name: Option<String>,
    
    /// Electrical type of the connection
    pub electrical_type: ElectricalType,
}

impl CircuitEdge {
    pub fn new(pin_number: impl Into<String>) -> Self {
        Self {
            pin_number: pin_number.into(),
            pin_name: None,
            electrical_type: ElectricalType::Unspecified,
        }
    }
    
    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.pin_name = Some(name.into());
        self
    }
    
    pub fn with_type(mut self, electrical_type: ElectricalType) -> Self {
        self.electrical_type = electrical_type;
        self
    }
}

/// The main Circuit struct using petgraph for graph representation
#[derive(Debug, Clone)]
pub struct Circuit {
    /// The underlying graph structure
    graph: DiGraph<CircuitNode, CircuitEdge>,
    
    /// Index mapping: component ref_des -> node index
    component_indices: HashMap<String, NodeIndex>,
    
    /// Index mapping: net name -> node index
    net_indices: HashMap<String, NodeIndex>,
    
    /// Circuit metadata
    pub metadata: CircuitMetadata,
}

impl Circuit {
    /// Create a new empty circuit
    pub fn new() -> Self {
        Self {
            graph: DiGraph::new(),
            component_indices: HashMap::new(),
            net_indices: HashMap::new(),
            metadata: CircuitMetadata::default(),
        }
    }
    
    /// Create a circuit from a UnifiedCircuitSchema
    pub fn from_ucs(ucs: UnifiedCircuitSchema) -> Self {
        let mut circuit = Self::new();
        circuit.metadata = ucs.metadata;
        
        // Pass 1: Add all components as nodes
        for component in ucs.components {
            circuit.add_component(component);
        }
        
        // Pass 2: Add all nets as nodes and create edges
        for net in ucs.nets {
            circuit.add_net_with_connections(net);
        }
        
        circuit
    }
    
    /// Convert back to UnifiedCircuitSchema
    pub fn to_ucs(&self) -> UnifiedCircuitSchema {
        let components: Vec<UcsComponent> = self.graph
            .node_weights()
            .filter_map(|n| n.as_component().cloned())
            .collect();
        
        let nets: Vec<UcsNet> = self.graph
            .node_weights()
            .filter_map(|n| n.as_net().cloned())
            .collect();
        
        UnifiedCircuitSchema {
            metadata: self.metadata.clone(),
            components,
            nets,
        }
    }
    
    /// Add a component to the circuit
    pub fn add_component(&mut self, component: UcsComponent) -> NodeIndex {
        let ref_des = component.ref_des.clone();
        let idx = self.graph.add_node(CircuitNode::Component(component));
        self.component_indices.insert(ref_des, idx);
        idx
    }
    
    /// Add a net to the circuit (without connections)
    pub fn add_net(&mut self, net: UcsNet) -> NodeIndex {
        let name = net.net_name.clone();
        let idx = self.graph.add_node(CircuitNode::Net(net));
        self.net_indices.insert(name, idx);
        idx
    }
    
    /// Add a net and create edges for all its connections
    pub fn add_net_with_connections(&mut self, net: UcsNet) -> NodeIndex {
        let connections = net.connections.clone();
        let net_idx = self.add_net(net);
        
        // Create edges from components to net
        for conn in connections {
            if let Some(&comp_idx) = self.component_indices.get(&conn.ref_des) {
                // Edge from component to net (represents the pin connection)
                let edge = CircuitEdge::new(&conn.pin_number);
                self.graph.add_edge(comp_idx, net_idx, edge);
            }
        }
        
        net_idx
    }
    
    /// Get a component by reference designator
    pub fn get_component(&self, ref_des: &str) -> Option<&UcsComponent> {
        self.component_indices
            .get(ref_des)
            .and_then(|&idx| self.graph.node_weight(idx))
            .and_then(|n| n.as_component())
    }
    
    /// Get a mutable component by reference designator
    pub fn get_component_mut(&mut self, ref_des: &str) -> Option<&mut UcsComponent> {
        self.component_indices
            .get(ref_des)
            .copied()
            .and_then(move |idx| self.graph.node_weight_mut(idx))
            .and_then(|n| n.as_component_mut())
    }
    
    /// Get a net by name
    pub fn get_net(&self, name: &str) -> Option<&UcsNet> {
        self.net_indices
            .get(name)
            .and_then(|&idx| self.graph.node_weight(idx))
            .and_then(|n| n.as_net())
    }
    
    /// Get a mutable net by name
    pub fn get_net_mut(&mut self, name: &str) -> Option<&mut UcsNet> {
        self.net_indices
            .get(name)
            .copied()
            .and_then(move |idx| self.graph.node_weight_mut(idx))
            .and_then(|n| n.as_net_mut())
    }
    
    /// Get all components
    pub fn components(&self) -> impl Iterator<Item = &UcsComponent> {
        self.graph
            .node_weights()
            .filter_map(|n| n.as_component())
    }
    
    /// Get all nets
    pub fn nets(&self) -> impl Iterator<Item = &UcsNet> {
        self.graph
            .node_weights()
            .filter_map(|n| n.as_net())
    }
    
    /// Get all ICs
    pub fn ics(&self) -> impl Iterator<Item = &UcsComponent> {
        self.components().filter(|c| c.is_ic())
    }
    
    /// Get all power nets
    pub fn power_nets(&self) -> impl Iterator<Item = &UcsNet> {
        self.nets().filter(|n| n.is_power_rail)
    }
    
    /// Get all nets connected to a component
    pub fn nets_for_component(&self, ref_des: &str) -> Vec<&UcsNet> {
        let Some(&comp_idx) = self.component_indices.get(ref_des) else {
            return Vec::new();
        };
        
        self.graph
            .edges_directed(comp_idx, Direction::Outgoing)
            .filter_map(|edge| {
                self.graph.node_weight(edge.target())
                    .and_then(|n| n.as_net())
            })
            .collect()
    }
    
    /// Get all components connected to a net
    pub fn components_on_net(&self, net_name: &str) -> Vec<&UcsComponent> {
        let Some(&net_idx) = self.net_indices.get(net_name) else {
            return Vec::new();
        };
        
        self.graph
            .edges_directed(net_idx, Direction::Incoming)
            .filter_map(|edge| {
                self.graph.node_weight(edge.source())
                    .and_then(|n| n.as_component())
            })
            .collect()
    }
    
    /// Get the pin connection info for a component-net connection
    pub fn get_connection_pin(&self, ref_des: &str, net_name: &str) -> Option<&CircuitEdge> {
        let comp_idx = self.component_indices.get(ref_des)?;
        let net_idx = self.net_indices.get(net_name)?;
        
        self.graph
            .edges_connecting(*comp_idx, *net_idx)
            .next()
            .map(|e| e.weight())
    }
    
    /// Find all components within a certain distance of another component
    pub fn components_near(&self, ref_des: &str, max_distance_mm: f64) -> Vec<&UcsComponent> {
        let Some(target) = self.get_component(ref_des) else {
            return Vec::new();
        };
        
        let Some(target_pos) = &target.position else {
            return Vec::new();
        };
        
        self.components()
            .filter(|c| {
                c.ref_des != ref_des &&
                c.position.as_ref()
                    .map(|p| p.distance_to(target_pos) <= max_distance_mm)
                    .unwrap_or(false)
            })
            .collect()
    }
    
    /// Find all capacitors near a component
    pub fn capacitors_near(&self, ref_des: &str, max_distance_mm: f64) -> Vec<&UcsComponent> {
        self.components_near(ref_des, max_distance_mm)
            .into_iter()
            .filter(|c| c.is_capacitor())
            .collect()
    }
    
    /// Propagate voltage from known power sources
    pub fn propagate_voltages(&mut self) {
        // Collect voltage sources (components that output known voltages)
        let voltage_sources = self.find_voltage_sources();
        
        for (net_name, voltage) in voltage_sources {
            if let Some(net) = self.get_net_mut(&net_name) {
                net.voltage_level = Some(voltage);
            }
        }
    }
    
    /// Find voltage sources based on component MPN/value.
    ///
    /// Recognises 78xx/LDO regulators by value/MPN, plus virtual power symbols.
    fn find_voltage_sources(&self) -> Vec<(String, f64)> {
        self.find_voltage_sources_pub()
    }

    /// Public variant of `find_voltage_sources` for use by `analysis.rs`.
    pub fn find_voltage_sources_pub(&self) -> Vec<(String, f64)> {
        let mut sources = Vec::new();

        for component in self.components() {
            // --- Voltage regulators ---
            if let Some(ref value) = component.value {
                if let Some(voltage) = Self::extract_regulator_voltage(value, component.mpn.as_deref()) {
                    if let Some(nets) = self.find_output_nets(component) {
                        for net in nets {
                            sources.push((net, voltage));
                        }
                    }
                }
            }

            // --- Virtual power symbols ---
            if component.is_virtual {
                if let Some(ref value) = component.value {
                    if let Some(v) = Self::parse_power_symbol_voltage(value) {
                        for net in self.nets_for_component(&component.ref_des) {
                            sources.push((net.net_name.clone(), v));
                        }
                    }
                }
            }
        }

        sources
    }

    /// Try to extract a fixed output voltage from a regulator value or MPN.
    ///
    /// Handles 78xx series, AMS1117-x.x, TLV1117-x.x, AP2112-x.x,
    /// MCP1700-xxxx, RT9080, LM1117, and generic "suffix-VOLTAGE" patterns.
    fn extract_regulator_voltage(value: &str, mpn: Option<&str>) -> Option<f64> {
        let candidates: Vec<String> = std::iter::once(value.to_uppercase())
            .chain(mpn.map(|m| m.to_uppercase()))
            .collect();

        for v in &candidates {
            // 78xx / 79xx series
            if v.contains("7805") { return Some(5.0); }
            if v.contains("7812") { return Some(12.0); }
            if v.contains("7809") { return Some(9.0); }
            if v.contains("7833") || v.contains("7803") { return Some(3.3); }

            // Known LDO families: parse voltage from suffix "XXX-<voltage>"
            for prefix in &[
                "AMS1117", "TLV1117", "LM1117", "AP2112", "MCP1700", "RT9080",
                "NCP1117", "LD1117", "SPX3819", "ME6211", "HT7333", "HT7350",
                "XC6206", "AP7361",
            ] {
                if let Some(rest) = v.strip_prefix(prefix) {
                    let num_str: String = rest
                        .trim_start_matches(|c: char| c == '-' || c == '_')
                        .chars()
                        .take_while(|c| c.is_ascii_digit() || *c == '.')
                        .collect();
                    if let Ok(voltage) = num_str.parse::<f64>() {
                        if voltage > 0.0 && voltage <= 50.0 {
                            return Some(voltage);
                        }
                    }
                }
            }

            // Generic: "3V3", "1V8", "5V0" patterns inside the value
            for window in v.as_bytes().windows(3) {
                if window[1] == b'V' && window[0].is_ascii_digit() && window[2].is_ascii_digit() {
                    let integer = (window[0] - b'0') as f64;
                    let fraction = (window[2] - b'0') as f64 / 10.0;
                    return Some(integer + fraction);
                }
            }
        }

        None
    }

    /// Parse a voltage from a virtual power symbol's value string.
    fn parse_power_symbol_voltage(value: &str) -> Option<f64> {
        let upper = value.to_uppercase();
        if upper == "GND" || upper == "VSS" || upper == "AGND" || upper == "DGND" {
            return Some(0.0);
        }
        if upper.contains("3V3") || upper.contains("3.3V") { return Some(3.3); }
        if upper.contains("1V8") || upper.contains("1.8V") { return Some(1.8); }
        if upper.contains("2V5") || upper.contains("2.5V") { return Some(2.5); }
        if upper.contains("5V")  { return Some(5.0); }
        if upper.contains("12V") { return Some(12.0); }
        if upper.contains("24V") { return Some(24.0); }
        None
    }

    
    /// Find output nets for a component (heuristic based on pin names)
    fn find_output_nets(&self, component: &UcsComponent) -> Option<Vec<String>> {
        let nets = self.nets_for_component(&component.ref_des);
        let output_nets: Vec<String> = nets
            .into_iter()
            .filter(|net| {
                let name_upper = net.net_name.to_uppercase();
                // Heuristic: output nets often have "OUT", "VOUT", or positive voltage names
                name_upper.contains("OUT") || name_upper.contains("3V") 
                    || name_upper.contains("5V") || name_upper.contains("12V")
                    || name_upper.contains("VCC") || name_upper.contains("VDD")
            })
            .map(|n| n.net_name.clone())
            .collect();
        
        if output_nets.is_empty() {
            None
        } else {
            Some(output_nets)
        }
    }
    
    /// Find path between two components through nets
    pub fn find_path(&self, from_ref: &str, to_ref: &str) -> Option<Vec<String>> {
        use petgraph::algo::astar;
        
        let from_idx = self.component_indices.get(from_ref)?;
        let to_idx = self.component_indices.get(to_ref)?;
        
        // Use A* to find shortest path
        let result = astar(
            &self.graph,
            *from_idx,
            |n| n == *to_idx,
            |_| 1,
            |_| 0,
        );
        
        result.map(|(_, path)| {
            path.into_iter()
                .filter_map(|idx| {
                    match self.graph.node_weight(idx) {
                        Some(CircuitNode::Component(c)) => Some(c.ref_des.clone()),
                        Some(CircuitNode::Net(n)) => Some(format!("[{}]", n.net_name)),
                        None => None,
                    }
                })
                .collect()
        })
    }
    
    /// Get statistics about the circuit
    pub fn stats(&self) -> CircuitStats {
        let component_count = self.component_indices.len();
        let net_count = self.net_indices.len();
        let edge_count = self.graph.edge_count();
        
        let ic_count = self.ics().count();
        let power_net_count = self.power_nets().count();
        
        CircuitStats {
            component_count,
            net_count,
            connection_count: edge_count,
            ic_count,
            power_net_count,
        }
    }
    
    /// Create a filtered view of the circuit for AI analysis
    pub fn create_ai_slice(&self, component_refs: &[&str]) -> UnifiedCircuitSchema {
        let mut ucs = UnifiedCircuitSchema::new(
            &self.metadata.project_name,
            self.metadata.source_cad,
        );
        ucs.metadata = self.metadata.clone();
        
        // Collect relevant components
        let mut relevant_nets: HashSet<String> = HashSet::new();
        
        for ref_des in component_refs {
            if let Some(comp) = self.get_component(ref_des) {
                ucs.add_component(comp.clone());
                
                // Collect nets connected to this component
                for net in self.nets_for_component(ref_des) {
                    relevant_nets.insert(net.net_name.clone());
                }
            }
        }
        
        // Add relevant nets
        for net_name in relevant_nets {
            if let Some(net) = self.get_net(&net_name) {
                ucs.add_net(net.clone());
            }
        }
        
        ucs
    }
}

impl Default for Circuit {
    fn default() -> Self {
        Self::new()
    }
}

/// Statistics about a circuit
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CircuitStats {
    pub component_count: usize,
    pub net_count: usize,
    pub connection_count: usize,
    pub ic_count: usize,
    pub power_net_count: usize,
}

#[cfg(test)]
mod tests {
    use super::*;
    
    fn create_test_circuit() -> Circuit {
        let mut ucs = UnifiedCircuitSchema::new("Test Circuit", SourceCAD::KiCad);
        
        // Add an IC
        let mut ic = UcsComponent::new("U1")
            .with_value("STM32F411")
            .with_position(100.0, 100.0);
        ic.add_pin(UcsPin::new("1").with_name("VDD").with_type(ElectricalType::PowerIn));
        ic.add_pin(UcsPin::new("2").with_name("GND").with_type(ElectricalType::PowerIn));
        ucs.add_component(ic);
        
        // Add a decoupling capacitor
        let cap = UcsComponent::new("C1")
            .with_value("100nF")
            .with_position(110.0, 100.0);
        ucs.add_component(cap);
        
        // Add nets
        let mut vcc = UcsNet::new("VCC").with_voltage(3.3);
        vcc.add_connection("U1", "1");
        vcc.add_connection("C1", "1");
        ucs.add_net(vcc);
        
        let mut gnd = UcsNet::new("GND").with_voltage(0.0);
        gnd.add_connection("U1", "2");
        gnd.add_connection("C1", "2");
        ucs.add_net(gnd);
        
        Circuit::from_ucs(ucs)
    }
    
    #[test]
    fn test_circuit_creation() {
        let circuit = create_test_circuit();
        
        assert!(circuit.get_component("U1").is_some());
        assert!(circuit.get_component("C1").is_some());
        assert!(circuit.get_net("VCC").is_some());
        assert!(circuit.get_net("GND").is_some());
    }
    
    #[test]
    fn test_nets_for_component() {
        let circuit = create_test_circuit();
        
        let nets = circuit.nets_for_component("U1");
        assert_eq!(nets.len(), 2);
        
        let net_names: Vec<&str> = nets.iter().map(|n| n.net_name.as_str()).collect();
        assert!(net_names.contains(&"VCC"));
        assert!(net_names.contains(&"GND"));
    }
    
    #[test]
    fn test_components_on_net() {
        let circuit = create_test_circuit();
        
        let components = circuit.components_on_net("VCC");
        assert_eq!(components.len(), 2);
        
        let refs: Vec<&str> = components.iter().map(|c| c.ref_des.as_str()).collect();
        assert!(refs.contains(&"U1"));
        assert!(refs.contains(&"C1"));
    }
    
    #[test]
    fn test_components_near() {
        let circuit = create_test_circuit();
        
        // C1 is 10mm from U1
        let nearby = circuit.components_near("U1", 15.0);
        assert_eq!(nearby.len(), 1);
        assert_eq!(nearby[0].ref_des, "C1");
        
        // Nothing within 5mm
        let nearby = circuit.components_near("U1", 5.0);
        assert!(nearby.is_empty());
    }
    
    #[test]
    fn test_circuit_stats() {
        let circuit = create_test_circuit();
        let stats = circuit.stats();
        
        assert_eq!(stats.component_count, 2);
        assert_eq!(stats.net_count, 2);
        assert_eq!(stats.ic_count, 1);
        assert_eq!(stats.power_net_count, 2); // VCC and GND are both power
    }
    
    #[test]
    fn test_ai_slice() {
        let circuit = create_test_circuit();
        let slice = circuit.create_ai_slice(&["U1"]);
        
        // Should include U1 and its connected nets
        assert_eq!(slice.components.len(), 1);
        assert_eq!(slice.nets.len(), 2); // VCC and GND
    }
    
    #[test]
    fn test_roundtrip() {
        let circuit = create_test_circuit();
        let ucs = circuit.to_ucs();
        let circuit2 = Circuit::from_ucs(ucs);
        
        assert_eq!(circuit.stats().component_count, circuit2.stats().component_count);
        assert_eq!(circuit.stats().net_count, circuit2.stats().net_count);
    }
}
