//! Unified Circuit Schema (UCS) Data Types
//!
//! This module defines the core data structures for the UCS v1.0 schema.
//! These types are designed to be:
//! - AI-readable: Clear, semantic field names with rich metadata
//! - Strictly typed: Full Rust type safety with serde support
//! - CAD-agnostic: Works with any EDA tool's output
//!
//! JSON Schema: http://json-schema.org/draft-07/schema#

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use chrono::{DateTime, Utc};

/// Source CAD tool that the circuit was imported from
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SourceCAD {
    KiCad,
    Altium,
    EasyEDA,
    Eagle,
    Netlist,
    EDIF,
    Unknown,
}

impl Default for SourceCAD {
    fn default() -> Self {
        SourceCAD::Unknown
    }
}

impl std::fmt::Display for SourceCAD {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SourceCAD::KiCad => write!(f, "KiCAD"),
            SourceCAD::Altium => write!(f, "Altium"),
            SourceCAD::EasyEDA => write!(f, "EasyEDA"),
            SourceCAD::Eagle => write!(f, "Eagle"),
            SourceCAD::Netlist => write!(f, "Netlist"),
            SourceCAD::EDIF => write!(f, "EDIF"),
            SourceCAD::Unknown => write!(f, "Unknown"),
        }
    }
}

/// Metadata about the circuit source and version
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CircuitMetadata {
    /// Project name
    pub project_name: String,
    
    /// Source CAD tool
    pub source_cad: SourceCAD,
    
    /// Version of the CAD tool
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cad_version: Option<String>,
    
    /// Timestamp of when this was parsed/created
    pub timestamp: DateTime<Utc>,
    
    /// Design variant (e.g., "default", "rev_a", "prototype")
    #[serde(default = "default_variant")]
    pub variant: String,
    
    /// Original file path
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_file: Option<String>,
    
    /// Schema version for forward compatibility
    #[serde(default = "default_schema_version")]
    pub schema_version: String,
}

fn default_variant() -> String {
    "default".to_string()
}

fn default_schema_version() -> String {
    "1.0".to_string()
}

impl Default for CircuitMetadata {
    fn default() -> Self {
        Self {
            project_name: "Untitled".to_string(),
            source_cad: SourceCAD::Unknown,
            cad_version: None,
            timestamp: Utc::now(),
            variant: default_variant(),
            source_file: None,
            schema_version: default_schema_version(),
        }
    }
}

/// Electrical type of a pin
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ElectricalType {
    Input,
    Output,
    Bidirectional,
    TriState,
    Passive,
    PowerIn,
    PowerOut,
    OpenCollector,
    OpenEmitter,
    NoConnect,
    Unspecified,
}

impl Default for ElectricalType {
    fn default() -> Self {
        ElectricalType::Unspecified
    }
}

impl std::fmt::Display for ElectricalType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ElectricalType::Input => write!(f, "Input"),
            ElectricalType::Output => write!(f, "Output"),
            ElectricalType::Bidirectional => write!(f, "Bidirectional"),
            ElectricalType::TriState => write!(f, "Tri-State"),
            ElectricalType::Passive => write!(f, "Passive"),
            ElectricalType::PowerIn => write!(f, "Power Input"),
            ElectricalType::PowerOut => write!(f, "Power Output"),
            ElectricalType::OpenCollector => write!(f, "Open Collector"),
            ElectricalType::OpenEmitter => write!(f, "Open Emitter"),
            ElectricalType::NoConnect => write!(f, "No Connect"),
            ElectricalType::Unspecified => write!(f, "Unspecified"),
        }
    }
}

/// A pin on a component
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UcsPin {
    /// Pin number (e.g., "1", "A1", "VCC")
    pub number: String,
    
    /// Pin name (e.g., "VDD", "GPIO0", "RESET")
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    
    /// Electrical type of the pin
    #[serde(default)]
    pub electrical_type: ElectricalType,
    
    /// Name of the net this pin is connected to
    #[serde(skip_serializing_if = "Option::is_none")]
    pub connected_net: Option<String>,
    
    /// Position relative to component (for layout)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub position: Option<UcsPosition>,
}

impl UcsPin {
    pub fn new(number: impl Into<String>) -> Self {
        Self {
            number: number.into(),
            name: None,
            electrical_type: ElectricalType::Unspecified,
            connected_net: None,
            position: None,
        }
    }
    
    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }
    
    pub fn with_type(mut self, electrical_type: ElectricalType) -> Self {
        self.electrical_type = electrical_type;
        self
    }
    
    pub fn with_net(mut self, net: impl Into<String>) -> Self {
        self.connected_net = Some(net.into());
        self
    }
}

/// Position in the schematic (in mm)
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct UcsPosition {
    pub x: f64,
    pub y: f64,
}

impl UcsPosition {
    pub fn new(x: f64, y: f64) -> Self {
        Self { x, y }
    }
    
    pub fn distance_to(&self, other: &UcsPosition) -> f64 {
        let dx = self.x - other.x;
        let dy = self.y - other.y;
        (dx * dx + dy * dy).sqrt()
    }
}

impl Default for UcsPosition {
    fn default() -> Self {
        Self { x: 0.0, y: 0.0 }
    }
}

/// A component in the circuit
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UcsComponent {
    /// Reference designator (e.g., "U1", "R1", "C10")
    pub ref_des: String,
    
    /// Manufacturer Part Number for datasheet lookup
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mpn: Option<String>,
    
    /// Component value (e.g., "10k", "0.1uF", "STM32F411")
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value: Option<String>,
    
    /// Footprint identifier
    #[serde(skip_serializing_if = "Option::is_none")]
    pub footprint: Option<String>,
    
    /// Library ID from source CAD
    #[serde(skip_serializing_if = "Option::is_none")]
    pub lib_id: Option<String>,
    
    /// Whether this is a virtual component (power symbol, label, etc.)
    #[serde(default)]
    pub is_virtual: bool,
    
    /// Component pins
    #[serde(default)]
    pub pins: Vec<UcsPin>,
    
    /// Position in schematic
    #[serde(skip_serializing_if = "Option::is_none")]
    pub position: Option<UcsPosition>,
    
    /// Rotation angle in degrees
    #[serde(default)]
    pub rotation: f64,
    
    /// Additional attributes (CAD-specific or AI-extracted data)
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub attributes: HashMap<String, AttributeValue>,
    
    /// Unique identifier
    #[serde(default = "generate_uuid")]
    pub uuid: String,
}

fn generate_uuid() -> String {
    uuid::Uuid::new_v4().to_string()
}

impl UcsComponent {
    pub fn new(ref_des: impl Into<String>) -> Self {
        Self {
            ref_des: ref_des.into(),
            mpn: None,
            value: None,
            footprint: None,
            lib_id: None,
            is_virtual: false,
            pins: Vec::new(),
            position: None,
            rotation: 0.0,
            attributes: HashMap::new(),
            uuid: generate_uuid(),
        }
    }
    
    pub fn with_value(mut self, value: impl Into<String>) -> Self {
        self.value = Some(value.into());
        self
    }
    
    pub fn with_mpn(mut self, mpn: impl Into<String>) -> Self {
        self.mpn = Some(mpn.into());
        self
    }
    
    pub fn with_footprint(mut self, footprint: impl Into<String>) -> Self {
        self.footprint = Some(footprint.into());
        self
    }
    
    pub fn with_position(mut self, x: f64, y: f64) -> Self {
        self.position = Some(UcsPosition::new(x, y));
        self
    }
    
    pub fn add_pin(&mut self, pin: UcsPin) {
        self.pins.push(pin);
    }
    
    pub fn set_attribute(&mut self, key: impl Into<String>, value: AttributeValue) {
        self.attributes.insert(key.into(), value);
    }
    
    /// Get the component type from reference designator
    pub fn component_type(&self) -> ComponentType {
        ComponentType::from_ref_des(&self.ref_des)
    }
    
    /// Check if this component is an IC (reference starts with 'U')
    pub fn is_ic(&self) -> bool {
        matches!(self.component_type(), ComponentType::IC)
    }
    
    /// Check if this component is a capacitor
    pub fn is_capacitor(&self) -> bool {
        matches!(self.component_type(), ComponentType::Capacitor)
    }
    
    /// Check if this component is a resistor
    pub fn is_resistor(&self) -> bool {
        matches!(self.component_type(), ComponentType::Resistor)
    }
    
    /// Get pin by number
    pub fn get_pin(&self, number: &str) -> Option<&UcsPin> {
        self.pins.iter().find(|p| p.number == number)
    }
    
    /// Get pin by name
    pub fn get_pin_by_name(&self, name: &str) -> Option<&UcsPin> {
        self.pins.iter().find(|p| p.name.as_deref() == Some(name))
    }
}

/// Component type inferred from reference designator
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ComponentType {
    Resistor,      // R
    Capacitor,     // C
    Inductor,      // L
    IC,            // U
    Transistor,    // Q
    Diode,         // D
    Connector,     // J, P
    Crystal,       // Y, X
    Fuse,          // F
    Switch,        // SW, S
    Relay,         // K
    Transformer,   // T
    LED,           // LED, D (with LED in value)
    PowerSymbol,   // PWR, #
    Unknown,
}

impl ComponentType {
    pub fn from_ref_des(ref_des: &str) -> Self {
        let upper = ref_des.to_uppercase();
        let prefix: String = upper.chars().take_while(|c| c.is_alphabetic()).collect();
        
        match prefix.as_str() {
            "R" => ComponentType::Resistor,
            "C" => ComponentType::Capacitor,
            "L" => ComponentType::Inductor,
            "U" => ComponentType::IC,
            "Q" => ComponentType::Transistor,
            "D" => ComponentType::Diode,
            "J" | "P" | "CN" => ComponentType::Connector,
            "Y" | "X" => ComponentType::Crystal,
            "F" => ComponentType::Fuse,
            "SW" | "S" => ComponentType::Switch,
            "K" => ComponentType::Relay,
            "T" => ComponentType::Transformer,
            "LED" => ComponentType::LED,
            "PWR" | "#" => ComponentType::PowerSymbol,
            _ => ComponentType::Unknown,
        }
    }
}

/// Attribute value that can hold different types
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum AttributeValue {
    String(String),
    Number(f64),
    Integer(i64),
    Boolean(bool),
    List(Vec<AttributeValue>),
    Object(HashMap<String, AttributeValue>),
}

impl From<String> for AttributeValue {
    fn from(s: String) -> Self {
        AttributeValue::String(s)
    }
}

impl From<&str> for AttributeValue {
    fn from(s: &str) -> Self {
        AttributeValue::String(s.to_string())
    }
}

impl From<f64> for AttributeValue {
    fn from(n: f64) -> Self {
        AttributeValue::Number(n)
    }
}

impl From<i64> for AttributeValue {
    fn from(n: i64) -> Self {
        AttributeValue::Integer(n)
    }
}

impl From<bool> for AttributeValue {
    fn from(b: bool) -> Self {
        AttributeValue::Boolean(b)
    }
}

/// Signal type classification for nets
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SignalType {
    Analog,
    Digital,
    HighSpeed,
    Power,
    Ground,
    Clock,
    Reset,
    Data,
    Control,
    Unknown,
}

impl Default for SignalType {
    fn default() -> Self {
        SignalType::Unknown
    }
}

impl SignalType {
    /// Infer signal type from net name
    pub fn from_net_name(name: &str) -> Self {
        let upper = name.to_uppercase();
        
        if upper.contains("GND") || upper.contains("VSS") || upper == "0V" {
            SignalType::Ground
        } else if upper.contains("VCC") || upper.contains("VDD") || upper.contains("3V3") 
            || upper.contains("5V") || upper.contains("12V") || upper.contains("VBAT")
            || upper.contains("VIN") || upper.contains("VOUT") {
            SignalType::Power
        } else if upper.contains("CLK") || upper.contains("CLOCK") || upper.contains("OSC") 
            || upper.contains("XTAL") {
            SignalType::Clock
        } else if upper.contains("RST") || upper.contains("RESET") || upper.contains("NRST") {
            SignalType::Reset
        } else if upper.contains("SDA") || upper.contains("SCL") || upper.contains("MOSI") 
            || upper.contains("MISO") || upper.contains("TX") || upper.contains("RX")
            || upper.contains("D+") || upper.contains("D-") {
            SignalType::Data
        } else if upper.contains("CS") || upper.contains("SS") || upper.contains("EN") 
            || upper.contains("OE") || upper.contains("WE") || upper.contains("CE") {
            SignalType::Control
        } else {
            SignalType::Unknown
        }
    }
}

/// A connection point in a net
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct NetConnection {
    /// Reference designator of the component
    pub ref_des: String,
    
    /// Pin number on the component
    pub pin_number: String,
}

impl NetConnection {
    pub fn new(ref_des: impl Into<String>, pin_number: impl Into<String>) -> Self {
        Self {
            ref_des: ref_des.into(),
            pin_number: pin_number.into(),
        }
    }
}

/// A net (electrical connection) in the circuit
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UcsNet {
    /// Net name
    pub net_name: String,
    
    /// Voltage level (if known, in Volts)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub voltage_level: Option<f64>,
    
    /// Whether this is a power rail
    #[serde(default)]
    pub is_power_rail: bool,
    
    /// Signal type classification
    #[serde(default)]
    pub signal_type: SignalType,
    
    /// All connections on this net
    pub connections: Vec<NetConnection>,
    
    /// Additional attributes
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub attributes: HashMap<String, AttributeValue>,
}

impl UcsNet {
    pub fn new(name: impl Into<String>) -> Self {
        let name = name.into();
        let signal_type = SignalType::from_net_name(&name);
        let is_power_rail = matches!(signal_type, SignalType::Power | SignalType::Ground);
        
        Self {
            net_name: name,
            voltage_level: None,
            is_power_rail,
            signal_type,
            connections: Vec::new(),
            attributes: HashMap::new(),
        }
    }
    
    pub fn with_voltage(mut self, voltage: f64) -> Self {
        self.voltage_level = Some(voltage);
        self
    }
    
    pub fn add_connection(&mut self, ref_des: impl Into<String>, pin: impl Into<String>) {
        self.connections.push(NetConnection::new(ref_des, pin));
    }
    
    /// Check if a component is connected to this net
    pub fn has_component(&self, ref_des: &str) -> bool {
        self.connections.iter().any(|c| c.ref_des == ref_des)
    }
    
    /// Get all components connected to this net
    pub fn connected_components(&self) -> Vec<&str> {
        self.connections.iter().map(|c| c.ref_des.as_str()).collect()
    }
}

/// The complete Unified Circuit Schema
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnifiedCircuitSchema {
    /// Circuit metadata
    pub metadata: CircuitMetadata,
    
    /// All components in the circuit
    pub components: Vec<UcsComponent>,
    
    /// All nets (connections) in the circuit
    pub nets: Vec<UcsNet>,
}

impl UnifiedCircuitSchema {
    pub fn new(project_name: impl Into<String>, source_cad: SourceCAD) -> Self {
        Self {
            metadata: CircuitMetadata {
                project_name: project_name.into(),
                source_cad,
                ..Default::default()
            },
            components: Vec::new(),
            nets: Vec::new(),
        }
    }
    
    pub fn add_component(&mut self, component: UcsComponent) {
        self.components.push(component);
    }
    
    pub fn add_net(&mut self, net: UcsNet) {
        self.nets.push(net);
    }
    
    /// Get a component by reference designator
    pub fn get_component(&self, ref_des: &str) -> Option<&UcsComponent> {
        self.components.iter().find(|c| c.ref_des == ref_des)
    }
    
    /// Get a mutable component by reference designator
    pub fn get_component_mut(&mut self, ref_des: &str) -> Option<&mut UcsComponent> {
        self.components.iter_mut().find(|c| c.ref_des == ref_des)
    }
    
    /// Get a net by name
    pub fn get_net(&self, name: &str) -> Option<&UcsNet> {
        self.nets.iter().find(|n| n.net_name == name)
    }
    
    /// Get a mutable net by name
    pub fn get_net_mut(&mut self, name: &str) -> Option<&mut UcsNet> {
        self.nets.iter_mut().find(|n| n.net_name == name)
    }
    
    /// Get all components of a specific type
    pub fn components_of_type(&self, comp_type: ComponentType) -> Vec<&UcsComponent> {
        self.components
            .iter()
            .filter(|c| c.component_type() == comp_type)
            .collect()
    }
    
    /// Get all ICs
    pub fn ics(&self) -> Vec<&UcsComponent> {
        self.components_of_type(ComponentType::IC)
    }
    
    /// Get all power nets
    pub fn power_nets(&self) -> Vec<&UcsNet> {
        self.nets.iter().filter(|n| n.is_power_rail).collect()
    }
    
    /// Find nets connected to a component
    pub fn nets_for_component(&self, ref_des: &str) -> Vec<&UcsNet> {
        self.nets
            .iter()
            .filter(|n| n.has_component(ref_des))
            .collect()
    }
    
    /// Serialize to JSON
    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(self)
    }
    
    /// Deserialize from JSON
    pub fn from_json(json: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(json)
    }
}

impl Default for UnifiedCircuitSchema {
    fn default() -> Self {
        Self::new("Untitled", SourceCAD::Unknown)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_component_type_from_ref_des() {
        assert_eq!(ComponentType::from_ref_des("R1"), ComponentType::Resistor);
        assert_eq!(ComponentType::from_ref_des("C10"), ComponentType::Capacitor);
        assert_eq!(ComponentType::from_ref_des("U1"), ComponentType::IC);
        assert_eq!(ComponentType::from_ref_des("Q2"), ComponentType::Transistor);
        assert_eq!(ComponentType::from_ref_des("D1"), ComponentType::Diode);
        assert_eq!(ComponentType::from_ref_des("Y1"), ComponentType::Crystal);
    }
    
    #[test]
    fn test_signal_type_from_net_name() {
        assert_eq!(SignalType::from_net_name("GND"), SignalType::Ground);
        assert_eq!(SignalType::from_net_name("VCC"), SignalType::Power);
        assert_eq!(SignalType::from_net_name("3V3"), SignalType::Power);
        assert_eq!(SignalType::from_net_name("CLK"), SignalType::Clock);
        assert_eq!(SignalType::from_net_name("NRST"), SignalType::Reset);
        assert_eq!(SignalType::from_net_name("SDA"), SignalType::Data);
    }
    
    #[test]
    fn test_ucs_component_builder() {
        let comp = UcsComponent::new("U1")
            .with_value("STM32F411")
            .with_mpn("STM32F411CEU6")
            .with_position(100.0, 50.0);
        
        assert_eq!(comp.ref_des, "U1");
        assert_eq!(comp.value, Some("STM32F411".to_string()));
        assert_eq!(comp.mpn, Some("STM32F411CEU6".to_string()));
        assert!(comp.position.is_some());
    }
    
    #[test]
    fn test_ucs_net_builder() {
        let mut net = UcsNet::new("VCC").with_voltage(3.3);
        net.add_connection("U1", "1");
        net.add_connection("C1", "1");
        
        assert_eq!(net.net_name, "VCC");
        assert_eq!(net.voltage_level, Some(3.3));
        assert!(net.is_power_rail);
        assert_eq!(net.signal_type, SignalType::Power);
        assert_eq!(net.connections.len(), 2);
    }
    
    #[test]
    fn test_unified_circuit_schema() {
        let mut ucs = UnifiedCircuitSchema::new("Test Project", SourceCAD::KiCad);
        
        ucs.add_component(UcsComponent::new("U1").with_value("STM32F411"));
        ucs.add_component(UcsComponent::new("C1").with_value("100nF"));
        
        let mut vcc = UcsNet::new("VCC").with_voltage(3.3);
        vcc.add_connection("U1", "VDD");
        vcc.add_connection("C1", "1");
        ucs.add_net(vcc);
        
        assert_eq!(ucs.components.len(), 2);
        assert_eq!(ucs.nets.len(), 1);
        assert!(ucs.get_component("U1").is_some());
        assert!(ucs.get_net("VCC").is_some());
    }
    
    #[test]
    fn test_json_serialization() {
        let mut ucs = UnifiedCircuitSchema::new("Test", SourceCAD::KiCad);
        ucs.add_component(UcsComponent::new("R1").with_value("10k"));
        
        let json = ucs.to_json().unwrap();
        let parsed = UnifiedCircuitSchema::from_json(&json).unwrap();
        
        assert_eq!(parsed.metadata.project_name, "Test");
        assert_eq!(parsed.components.len(), 1);
    }
}
