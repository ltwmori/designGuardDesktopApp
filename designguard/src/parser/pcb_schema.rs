//! PCB Schema Definitions
//!
//! Data structures for representing KiCAD PCB files (.kicad_pcb)
//! Based on KiCAD 6.0+ file format specification.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Represents a complete PCB design
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PcbDesign {
    pub uuid: String,
    pub filename: String,
    pub version: Option<String>,
    pub general: PcbGeneral,
    pub layers: Vec<PcbLayer>,
    pub setup: PcbSetup,
    pub nets: Vec<PcbNet>,
    pub footprints: Vec<Footprint>,
    pub traces: Vec<Trace>,
    pub vias: Vec<Via>,
    pub zones: Vec<Zone>,
    pub graphics: Vec<GraphicItem>,
}

/// General PCB settings
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PcbGeneral {
    pub thickness: f64,           // Board thickness in mm
    pub drawings: u32,
    pub tracks: u32,
    pub zones: u32,
    pub modules: u32,
    pub nets: u32,
}

/// PCB Layer definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PcbLayer {
    pub ordinal: u32,             // Layer number (0-31 for copper)
    pub canonical_name: String,   // e.g., "F.Cu", "B.Cu", "In1.Cu"
    pub layer_type: LayerType,
    pub user_name: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum LayerType {
    Signal,      // Copper signal layer
    Power,       // Power plane
    Mixed,       // Mixed signal/power
    Jumper,      // Jumper layer
    User,        // User-defined
    Unknown,
}

impl Default for LayerType {
    fn default() -> Self {
        LayerType::Signal
    }
}

/// PCB Setup/Design Rules
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PcbSetup {
    pub trace_min: f64,           // Minimum trace width (mm)
    pub via_size: f64,            // Default via size (mm)
    pub via_drill: f64,           // Default via drill (mm)
    pub via_min_size: f64,        // Minimum via size (mm)
    pub via_min_drill: f64,       // Minimum via drill (mm)
    pub copper_thickness: CopperThickness,
    pub clearance: f64,           // Default clearance (mm)
    pub track_width: f64,         // Default track width (mm)
}

/// Copper thickness per layer (in oz or mm)
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CopperThickness {
    pub outer: f64,               // Outer layer thickness (oz)
    pub inner: f64,               // Inner layer thickness (oz)
}

impl CopperThickness {
    /// Convert oz to mm (1 oz = 0.035mm = 35µm)
    pub fn outer_mm(&self) -> f64 {
        self.outer * 0.035
    }
    
    pub fn inner_mm(&self) -> f64 {
        self.inner * 0.035
    }
}

/// Net definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PcbNet {
    pub id: u32,
    pub name: String,
}

/// Footprint (component) on PCB
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Footprint {
    pub uuid: String,
    pub reference: String,
    pub value: String,
    pub footprint_lib: String,
    pub layer: String,
    pub position: Position3D,
    pub rotation: f64,
    pub pads: Vec<Pad>,
    pub properties: HashMap<String, String>,
}

/// 3D Position
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct Position3D {
    pub x: f64,
    pub y: f64,
    pub z: f64,
}

impl Position3D {
    pub fn new(x: f64, y: f64) -> Self {
        Self { x, y, z: 0.0 }
    }
}

/// Pad on a footprint
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Pad {
    pub number: String,
    pub pad_type: PadType,
    pub shape: PadShape,
    pub position: Position3D,
    pub size: Size2D,
    pub drill: Option<DrillInfo>,
    pub layers: Vec<String>,
    pub net: Option<u32>,
    pub net_name: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum PadType {
    ThruHole,
    SMD,
    Connect,
    NPThruHole,  // Non-plated through hole
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum PadShape {
    Circle,
    Rect,
    Oval,
    Trapezoid,
    RoundRect,
    Custom,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Size2D {
    pub width: f64,
    pub height: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DrillInfo {
    pub diameter: f64,
    pub offset: Option<Position3D>,
}

/// PCB Trace (track segment)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Trace {
    pub uuid: String,
    pub start: Position3D,
    pub end: Position3D,
    pub width: f64,               // Trace width in mm
    pub layer: String,            // Layer name (e.g., "F.Cu")
    pub net: u32,                 // Net ID
    pub net_name: Option<String>, // Net name for convenience
    pub locked: bool,
}

impl Trace {
    /// Calculate trace length in mm
    pub fn length(&self) -> f64 {
        let dx = self.end.x - self.start.x;
        let dy = self.end.y - self.start.y;
        (dx * dx + dy * dy).sqrt()
    }
    
    /// Get trace cross-sectional area in mm²
    pub fn cross_section_area(&self, copper_thickness_mm: f64) -> f64 {
        self.width * copper_thickness_mm
    }
}

/// Via (vertical interconnect)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Via {
    pub uuid: String,
    pub position: Position3D,
    pub size: f64,                // Via pad size (mm)
    pub drill: f64,               // Drill diameter (mm)
    pub layers: (String, String), // Start and end layers
    pub net: u32,
    pub net_name: Option<String>,
    pub via_type: ViaType,
    pub locked: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ViaType {
    Through,
    Blind,
    Buried,
    Micro,
}

impl Default for ViaType {
    fn default() -> Self {
        ViaType::Through
    }
}

/// Copper zone (pour)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Zone {
    pub uuid: String,
    pub net: u32,
    pub net_name: String,
    pub layer: String,
    pub priority: u32,
    pub connect_pads: ZoneConnectType,
    pub min_thickness: f64,
    pub filled: bool,
    pub outline: Vec<Position3D>,     // Zone boundary polygon
    pub filled_polygons: Vec<FilledPolygon>,
    pub keepout: Option<ZoneKeepout>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ZoneConnectType {
    Solid,
    ThermalRelief,
    None,
}

impl Default for ZoneConnectType {
    fn default() -> Self {
        ZoneConnectType::ThermalRelief
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FilledPolygon {
    pub layer: String,
    pub points: Vec<Position3D>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ZoneKeepout {
    pub tracks: bool,
    pub vias: bool,
    pub pads: bool,
    pub copperpour: bool,
    pub footprints: bool,
}

/// Graphic items (lines, arcs, circles, text)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphicItem {
    pub item_type: GraphicType,
    pub layer: String,
    pub start: Position3D,
    pub end: Option<Position3D>,
    pub width: f64,
    pub fill: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum GraphicType {
    Line,
    Arc,
    Circle,
    Rect,
    Polygon,
    Text,
}

// ============================================================================
// Helper structures for analysis
// ============================================================================

/// Represents a trace segment with full context for analysis
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TraceSegmentAnalysis {
    pub trace: Trace,
    pub copper_thickness_mm: f64,
    pub is_outer_layer: bool,
    pub length_mm: f64,
    pub cross_section_mm2: f64,
}

/// Net classification for EMI analysis
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum NetClassification {
    HighSpeed,      // USB, HDMI, Ethernet, etc.
    Clock,          // Clock signals
    Power,          // Power rails
    Ground,         // Ground nets
    Analog,         // Analog signals
    Digital,        // Standard digital
    Unknown,
}

/// Layer stack information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LayerStack {
    pub layers: Vec<LayerStackEntry>,
    pub total_thickness_mm: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LayerStackEntry {
    pub name: String,
    pub layer_type: LayerType,
    pub thickness_mm: f64,
    pub material: String,
}

impl Default for PcbDesign {
    fn default() -> Self {
        Self {
            uuid: String::new(),
            filename: String::new(),
            version: None,
            general: PcbGeneral::default(),
            layers: Vec::new(),
            setup: PcbSetup::default(),
            nets: Vec::new(),
            footprints: Vec::new(),
            traces: Vec::new(),
            vias: Vec::new(),
            zones: Vec::new(),
            graphics: Vec::new(),
        }
    }
}
