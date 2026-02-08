use std::collections::HashMap;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Schematic {
    pub uuid: String,
    pub filename: String,
    pub version: Option<String>,
    pub components: Vec<Component>,
    pub wires: Vec<Wire>,
    pub labels: Vec<Label>,
    pub nets: Vec<Net>,
    pub power_symbols: Vec<Component>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Component {
    pub uuid: String,
    pub reference: String,   // R1, C1, U1
    pub value: String,       // 10k, 100nF, STM32F4
    pub lib_id: String,      // Device:R
    pub footprint: Option<String>,
    pub position: Position,
    pub rotation: f64,       // Rotation in degrees
    pub properties: HashMap<String, String>,
    pub pins: Vec<Pin>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Position {
    pub x: f64,
    pub y: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Pin {
    pub number: String,
    pub uuid: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Wire {
    pub uuid: String,
    pub points: Vec<Position>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Label {
    pub uuid: String,
    pub text: String,
    pub position: Position,
    pub rotation: f64,
    pub label_type: LabelType,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum LabelType {
    Local,
    Global,
    Hierarchical,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Net {
    pub name: String,
    pub connections: Vec<Connection>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct Connection {
    pub component_ref: String,
    pub pin_number: String,
}
