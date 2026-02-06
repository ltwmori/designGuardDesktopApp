//! KiCAD PCB Parser
//!
//! This module parses KiCAD PCB files (.kicad_pcb) following the official
//! S-Expression file format specification.
//!
//! Key format details:
//! - All values are in millimeters
//! - Layers are identified by ordinal number and canonical name
//! - Traces are stored as (segment ...) elements
//! - Zones contain filled polygon data

use std::path::Path;
use crate::parser::pcb_schema::*;
use crate::parser::sexp::{SExp, SExpParser, ParseError};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum PcbParseError {
    #[error("S-expression parse error: {0}")]
    SExpParse(#[from] ParseError),
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Invalid PCB format: {0}")]
    InvalidFormat(String),
    #[error("Missing required field: {0}")]
    MissingField(String),
}

/// Parser for KiCAD PCB files
///
/// Supports KiCad versions 4-9:
/// - **KiCad 4-5**: Legacy text-based format (`.brd`) - automatically routed to legacy parser
/// - **KiCad 6-9**: Modern S-expression format (`.kicad_pcb`) - uses S-expression parser
///
/// Format detection and routing is handled automatically by the format detector.
/// This parser provides a unified interface for all supported KiCad versions.
pub struct PcbParser;

impl PcbParser {
    /// Parse a PCB file, automatically detecting format (KiCad 4-9)
    pub fn parse_pcb(path: &Path) -> Result<PcbDesign, PcbParseError> {
        // Use format detector to automatically route to correct parser
        crate::parser::format_detector::detect_and_parse_pcb(path)
    }
    
    /// Parse PCB from string (for modern S-expression format only)
    /// For format detection, use parse_pcb() with a Path instead
    pub fn parse_pcb_str(content: &str, filename: &str) -> Result<PcbDesign, PcbParseError> {
        let mut parser = SExpParser::new(content);
        let root = parser.parse()?;

        // Root should be (kicad_pcb ...)
        let kicad_pcb = root
            .as_list()
            .and_then(|list| list.first())
            .and_then(|s| s.as_atom())
            .ok_or_else(|| PcbParseError::InvalidFormat("Expected kicad_pcb root".to_string()))?;

        if kicad_pcb != "kicad_pcb" {
            return Err(PcbParseError::InvalidFormat(
                format!("Expected kicad_pcb, found {}", kicad_pcb)
            ));
        }

        let root_list = root.as_list().unwrap();
        let mut pcb = PcbDesign {
            uuid: Self::extract_uuid(&root)?,
            filename: filename.to_string(),
            version: Self::extract_version(&root),
            ..Default::default()
        };

        // Parse all elements
        for item in root_list.iter().skip(1) {
            if let Some(tag) = item.as_list().and_then(|l| l.first()).and_then(|a| a.as_atom()) {
                match tag {
                    "general" => {
                        pcb.general = Self::parse_general(item)?;
                    }
                    "layers" => {
                        pcb.layers = Self::parse_layers(item)?;
                    }
                    "setup" => {
                        pcb.setup = Self::parse_setup(item)?;
                    }
                    "net" => {
                        if let Ok(net) = Self::parse_net(item) {
                            pcb.nets.push(net);
                        }
                    }
                    "footprint" | "module" => {
                        if let Ok(fp) = Self::parse_footprint(item, &pcb.nets) {
                            pcb.footprints.push(fp);
                        }
                    }
                    "segment" => {
                        if let Ok(trace) = Self::parse_trace(item, &pcb.nets) {
                            pcb.traces.push(trace);
                        }
                    }
                    "via" => {
                        if let Ok(via) = Self::parse_via(item, &pcb.nets) {
                            pcb.vias.push(via);
                        }
                    }
                    "zone" => {
                        if let Ok(zone) = Self::parse_zone(item) {
                            pcb.zones.push(zone);
                        }
                    }
                    "gr_line" | "gr_arc" | "gr_circle" | "gr_rect" | "gr_poly" | "gr_text" => {
                        if let Ok(graphic) = Self::parse_graphic(item, tag) {
                            pcb.graphics.push(graphic);
                        }
                    }
                    _ => {
                        // Ignore unknown elements
                    }
                }
            }
        }

        Ok(pcb)
    }

    fn get_string_value(sexp: &SExp, key: &str) -> Option<String> {
        sexp.get(key).and_then(|exp| {
            if let Some(list) = exp.as_list() {
                list.get(1).and_then(|v| v.as_atom()).map(|s| s.to_string())
            } else {
                exp.as_atom().map(|s| s.to_string())
            }
        })
    }

    fn get_float_value(sexp: &SExp, key: &str) -> Option<f64> {
        Self::get_string_value(sexp, key).and_then(|s| s.parse().ok())
    }

    fn get_int_value(sexp: &SExp, key: &str) -> Option<u32> {
        Self::get_string_value(sexp, key).and_then(|s| s.parse().ok())
    }

    fn extract_uuid(root: &SExp) -> Result<String, PcbParseError> {
        if let Some(uuid_str) = Self::get_string_value(root, "uuid") {
            Ok(uuid_str)
        } else {
            Ok(uuid::Uuid::new_v4().to_string())
        }
    }

    fn extract_version(root: &SExp) -> Option<String> {
        Self::get_string_value(root, "version")
    }

    fn parse_general(sexp: &SExp) -> Result<PcbGeneral, PcbParseError> {
        Ok(PcbGeneral {
            thickness: Self::get_float_value(sexp, "thickness").unwrap_or(1.6),
            drawings: Self::get_int_value(sexp, "drawings").unwrap_or(0),
            tracks: Self::get_int_value(sexp, "tracks").unwrap_or(0),
            zones: Self::get_int_value(sexp, "zones").unwrap_or(0),
            modules: Self::get_int_value(sexp, "modules").unwrap_or(0),
            nets: Self::get_int_value(sexp, "nets").unwrap_or(0),
        })
    }

    fn parse_layers(sexp: &SExp) -> Result<Vec<PcbLayer>, PcbParseError> {
        let mut layers = Vec::new();
        
        if let Some(list) = sexp.as_list() {
            for item in list.iter().skip(1) {
                if let Some(layer_list) = item.as_list() {
                    if layer_list.len() >= 3 {
                        let ordinal = layer_list[0]
                            .as_atom()
                            .and_then(|s| s.parse().ok())
                            .unwrap_or(0);
                        let canonical_name = layer_list[1]
                            .as_atom()
                            .unwrap_or("")
                            .to_string();
                        let type_str = layer_list[2]
                            .as_atom()
                            .unwrap_or("signal");
                        
                        let layer_type = match type_str {
                            "signal" => LayerType::Signal,
                            "power" => LayerType::Power,
                            "mixed" => LayerType::Mixed,
                            "jumper" => LayerType::Jumper,
                            "user" => LayerType::User,
                            _ => LayerType::Unknown,
                        };
                        
                        let user_name = layer_list.get(3)
                            .and_then(|s| s.as_atom())
                            .map(|s| s.to_string());
                        
                        layers.push(PcbLayer {
                            ordinal,
                            canonical_name,
                            layer_type,
                            user_name,
                        });
                    }
                }
            }
        }
        
        Ok(layers)
    }

    fn parse_setup(sexp: &SExp) -> Result<PcbSetup, PcbParseError> {
        let mut setup = PcbSetup::default();
        
        // Default copper thickness (1oz outer, 0.5oz inner)
        setup.copper_thickness = CopperThickness {
            outer: 1.0,
            inner: 0.5,
        };
        
        if let Some(list) = sexp.as_list() {
            for item in list.iter().skip(1) {
                if let Some(item_list) = item.as_list() {
                    if let Some(key) = item_list.first().and_then(|a| a.as_atom()) {
                        let value = item_list.get(1)
                            .and_then(|v| v.as_atom())
                            .and_then(|s| s.parse::<f64>().ok())
                            .unwrap_or(0.0);
                        
                        match key {
                            "trace_min" => setup.trace_min = value,
                            "via_size" => setup.via_size = value,
                            "via_drill" => setup.via_drill = value,
                            "via_min_size" => setup.via_min_size = value,
                            "via_min_drill" => setup.via_min_drill = value,
                            "clearance" => setup.clearance = value,
                            "track_width" => setup.track_width = value,
                            "stackup" => {
                                // Parse stackup for copper thickness
                                if let Some(stackup_list) = item.as_list() {
                                    for layer_item in stackup_list.iter() {
                                        if let Some(layer_list) = layer_item.as_list() {
                                            if let Some(first) = layer_list.first().and_then(|a| a.as_atom()) {
                                                if first == "layer" {
                                                    // Parse copper thickness from layer definition
                                                    if let Some(thickness) = Self::get_float_value(layer_item, "thickness") {
                                                        // Convert µm to oz (35µm = 1oz)
                                                        let oz = thickness / 35.0;
                                                        if let Some(name) = layer_list.get(1).and_then(|a| a.as_atom()) {
                                                            if name == "F.Cu" || name == "B.Cu" {
                                                                setup.copper_thickness.outer = oz;
                                                            } else if name.contains(".Cu") {
                                                                setup.copper_thickness.inner = oz;
                                                            }
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                            _ => {}
                        }
                    }
                }
            }
        }
        
        Ok(setup)
    }

    fn parse_net(sexp: &SExp) -> Result<PcbNet, PcbParseError> {
        let list = sexp.as_list()
            .ok_or_else(|| PcbParseError::InvalidFormat("Net must be a list".to_string()))?;
        
        if list.len() < 3 {
            return Err(PcbParseError::InvalidFormat("Net requires id and name".to_string()));
        }
        
        let id = list[1]
            .as_atom()
            .and_then(|s| s.parse().ok())
            .ok_or_else(|| PcbParseError::MissingField("net id".to_string()))?;
        
        let name = list[2]
            .as_atom()
            .unwrap_or("")
            .to_string();
        
        Ok(PcbNet { id, name })
    }

    fn parse_footprint(sexp: &SExp, nets: &[PcbNet]) -> Result<Footprint, PcbParseError> {
        let uuid = Self::get_string_value(sexp, "uuid")
            .unwrap_or_else(|| uuid::Uuid::new_v4().to_string());
        
        // Footprint library is the second element
        let footprint_lib = sexp.as_list()
            .and_then(|l| l.get(1))
            .and_then(|a| a.as_atom())
            .unwrap_or("")
            .to_string();
        
        let layer = Self::get_string_value(sexp, "layer").unwrap_or_else(|| "F.Cu".to_string());
        let position = Self::parse_at(sexp).unwrap_or_default();
        
        let mut reference = String::new();
        let mut value = String::new();
        let mut properties = std::collections::HashMap::new();
        let mut pads = Vec::new();
        
        // Parse properties and pads
        for prop_exp in sexp.get_all("property") {
            if let Some(list) = prop_exp.as_list() {
                if list.len() >= 3 {
                    if let Some(key) = list[1].as_atom() {
                        if let Some(val) = list[2].as_atom() {
                            properties.insert(key.to_string(), val.to_string());
                            match key {
                                "Reference" => reference = val.to_string(),
                                "Value" => value = val.to_string(),
                                _ => {}
                            }
                        }
                    }
                }
            }
        }
        
        // Also check fp_text for reference/value (older format)
        for text_exp in sexp.get_all("fp_text") {
            if let Some(list) = text_exp.as_list() {
                if list.len() >= 3 {
                    if let Some(text_type) = list[1].as_atom() {
                        if let Some(text_value) = list[2].as_atom() {
                            match text_type {
                                "reference" => reference = text_value.to_string(),
                                "value" => value = text_value.to_string(),
                                _ => {}
                            }
                        }
                    }
                }
            }
        }
        
        // Parse pads
        for pad_exp in sexp.get_all("pad") {
            if let Ok(pad) = Self::parse_pad(pad_exp, nets) {
                pads.push(pad);
            }
        }
        
        Ok(Footprint {
            uuid,
            reference,
            value,
            footprint_lib,
            layer,
            position,
            rotation: 0.0,
            pads,
            properties,
        })
    }

    fn parse_pad(sexp: &SExp, nets: &[PcbNet]) -> Result<Pad, PcbParseError> {
        let list = sexp.as_list()
            .ok_or_else(|| PcbParseError::InvalidFormat("Pad must be a list".to_string()))?;
        
        if list.len() < 4 {
            return Err(PcbParseError::InvalidFormat("Pad requires number, type, shape".to_string()));
        }
        
        let number = list[1].as_atom().unwrap_or("").to_string();
        
        let pad_type = match list[2].as_atom().unwrap_or("") {
            "thru_hole" => PadType::ThruHole,
            "smd" => PadType::SMD,
            "connect" => PadType::Connect,
            "np_thru_hole" => PadType::NPThruHole,
            _ => PadType::SMD,
        };
        
        let shape = match list[3].as_atom().unwrap_or("") {
            "circle" => PadShape::Circle,
            "rect" => PadShape::Rect,
            "oval" => PadShape::Oval,
            "trapezoid" => PadShape::Trapezoid,
            "roundrect" => PadShape::RoundRect,
            "custom" => PadShape::Custom,
            _ => PadShape::Circle,
        };
        
        let position = Self::parse_at(sexp).unwrap_or_default();
        let size = Self::parse_size(sexp).unwrap_or_default();
        
        // Parse drill
        let drill = sexp.get("drill").and_then(|d| {
            if let Some(drill_list) = d.as_list() {
                let diameter = drill_list.get(1)
                    .and_then(|v| v.as_atom())
                    .and_then(|s| s.parse().ok())
                    .unwrap_or(0.0);
                Some(DrillInfo {
                    diameter,
                    offset: None,
                })
            } else {
                None
            }
        });
        
        // Parse layers
        let mut layers = Vec::new();
        if let Some(layers_exp) = sexp.get("layers") {
            if let Some(layers_list) = layers_exp.as_list() {
                for layer in layers_list.iter().skip(1) {
                    if let Some(layer_name) = layer.as_atom() {
                        layers.push(layer_name.to_string());
                    }
                }
            }
        }
        
        // Parse net
        let (net, net_name) = if let Some(net_exp) = sexp.get("net") {
            if let Some(net_list) = net_exp.as_list() {
                let net_id = net_list.get(1)
                    .and_then(|v| v.as_atom())
                    .and_then(|s| s.parse().ok());
                let name = net_list.get(2)
                    .and_then(|v| v.as_atom())
                    .map(|s| s.to_string())
                    .or_else(|| {
                        net_id.and_then(|id| {
                            nets.iter().find(|n| n.id == id).map(|n| n.name.clone())
                        })
                    });
                (net_id, name)
            } else {
                (None, None)
            }
        } else {
            (None, None)
        };
        
        Ok(Pad {
            number,
            pad_type,
            shape,
            position,
            size,
            drill,
            layers,
            net,
            net_name,
        })
    }

    fn parse_trace(sexp: &SExp, nets: &[PcbNet]) -> Result<Trace, PcbParseError> {
        let uuid = Self::get_string_value(sexp, "uuid")
            .unwrap_or_else(|| uuid::Uuid::new_v4().to_string());
        
        let start = Self::parse_start(sexp)?;
        let end = Self::parse_end(sexp)?;
        
        let width = Self::get_float_value(sexp, "width")
            .ok_or_else(|| PcbParseError::MissingField("trace width".to_string()))?;
        
        let layer = Self::get_string_value(sexp, "layer")
            .ok_or_else(|| PcbParseError::MissingField("trace layer".to_string()))?;
        
        let net = Self::get_int_value(sexp, "net").unwrap_or(0);
        let net_name = nets.iter().find(|n| n.id == net).map(|n| n.name.clone());
        
        let locked = sexp.get("locked").is_some();
        
        Ok(Trace {
            uuid,
            start,
            end,
            width,
            layer,
            net,
            net_name,
            locked,
        })
    }

    fn parse_via(sexp: &SExp, nets: &[PcbNet]) -> Result<Via, PcbParseError> {
        let uuid = Self::get_string_value(sexp, "uuid")
            .unwrap_or_else(|| uuid::Uuid::new_v4().to_string());
        
        let position = Self::parse_at(sexp).unwrap_or_default();
        
        let size = Self::get_float_value(sexp, "size")
            .ok_or_else(|| PcbParseError::MissingField("via size".to_string()))?;
        
        let drill = Self::get_float_value(sexp, "drill")
            .ok_or_else(|| PcbParseError::MissingField("via drill".to_string()))?;
        
        // Parse layers
        let layers = if let Some(layers_exp) = sexp.get("layers") {
            if let Some(layers_list) = layers_exp.as_list() {
                let start = layers_list.get(1)
                    .and_then(|v| v.as_atom())
                    .unwrap_or("F.Cu")
                    .to_string();
                let end = layers_list.get(2)
                    .and_then(|v| v.as_atom())
                    .unwrap_or("B.Cu")
                    .to_string();
                (start, end)
            } else {
                ("F.Cu".to_string(), "B.Cu".to_string())
            }
        } else {
            ("F.Cu".to_string(), "B.Cu".to_string())
        };
        
        let net = Self::get_int_value(sexp, "net").unwrap_or(0);
        let net_name = nets.iter().find(|n| n.id == net).map(|n| n.name.clone());
        
        let via_type = if let Some(type_exp) = sexp.get("type") {
            match type_exp.as_atom().unwrap_or("") {
                "blind" => ViaType::Blind,
                "buried" => ViaType::Buried,
                "micro" => ViaType::Micro,
                _ => ViaType::Through,
            }
        } else {
            ViaType::Through
        };
        
        let locked = sexp.get("locked").is_some();
        
        Ok(Via {
            uuid,
            position,
            size,
            drill,
            layers,
            net,
            net_name,
            via_type,
            locked,
        })
    }

    fn parse_zone(sexp: &SExp) -> Result<Zone, PcbParseError> {
        let uuid = Self::get_string_value(sexp, "uuid")
            .unwrap_or_else(|| uuid::Uuid::new_v4().to_string());
        
        let net = Self::get_int_value(sexp, "net").unwrap_or(0);
        let net_name = Self::get_string_value(sexp, "net_name").unwrap_or_default();
        let layer = Self::get_string_value(sexp, "layer").unwrap_or_default();
        let priority = Self::get_int_value(sexp, "priority").unwrap_or(0);
        let min_thickness = Self::get_float_value(sexp, "min_thickness").unwrap_or(0.0);
        let filled = sexp.get("filled_areas_thickness").is_some();
        
        // Parse connect_pads
        let connect_pads = if let Some(cp_exp) = sexp.get("connect_pads") {
            if let Some(cp_list) = cp_exp.as_list() {
                if cp_list.len() > 1 {
                    match cp_list[1].as_atom().unwrap_or("") {
                        "yes" => ZoneConnectType::Solid,
                        "no" => ZoneConnectType::None,
                        _ => ZoneConnectType::ThermalRelief,
                    }
                } else {
                    ZoneConnectType::ThermalRelief
                }
            } else {
                ZoneConnectType::ThermalRelief
            }
        } else {
            ZoneConnectType::ThermalRelief
        };
        
        // Parse outline polygon
        let mut outline = Vec::new();
        if let Some(polygon_exp) = sexp.get("polygon") {
            if let Some(pts_exp) = polygon_exp.get("pts") {
                outline = Self::parse_pts(pts_exp);
            }
        }
        
        // Parse filled polygons
        let mut filled_polygons = Vec::new();
        for fp_exp in sexp.get_all("filled_polygon") {
            if let Some(_fp_list) = fp_exp.as_list() {
                let fp_layer = Self::get_string_value(fp_exp, "layer").unwrap_or_default();
                if let Some(pts_exp) = fp_exp.get("pts") {
                    let points = Self::parse_pts(pts_exp);
                    filled_polygons.push(FilledPolygon {
                        layer: fp_layer,
                        points,
                    });
                }
            }
        }
        
        // Parse keepout
        let keepout = if let Some(ko_exp) = sexp.get("keepout") {
            Some(ZoneKeepout {
                tracks: Self::get_string_value(ko_exp, "tracks") == Some("not_allowed".to_string()),
                vias: Self::get_string_value(ko_exp, "vias") == Some("not_allowed".to_string()),
                pads: Self::get_string_value(ko_exp, "pads") == Some("not_allowed".to_string()),
                copperpour: Self::get_string_value(ko_exp, "copperpour") == Some("not_allowed".to_string()),
                footprints: Self::get_string_value(ko_exp, "footprints") == Some("not_allowed".to_string()),
            })
        } else {
            None
        };
        
        Ok(Zone {
            uuid,
            net,
            net_name,
            layer,
            priority,
            connect_pads,
            min_thickness,
            filled,
            outline,
            filled_polygons,
            keepout,
        })
    }

    fn parse_graphic(sexp: &SExp, tag: &str) -> Result<GraphicItem, PcbParseError> {
        let item_type = match tag {
            "gr_line" => GraphicType::Line,
            "gr_arc" => GraphicType::Arc,
            "gr_circle" => GraphicType::Circle,
            "gr_rect" => GraphicType::Rect,
            "gr_poly" => GraphicType::Polygon,
            "gr_text" => GraphicType::Text,
            _ => GraphicType::Line,
        };
        
        let layer = Self::get_string_value(sexp, "layer").unwrap_or_default();
        let start = Self::parse_start(sexp).unwrap_or_default();
        let end = Self::parse_end(sexp).ok();
        let width = Self::get_float_value(sexp, "width").unwrap_or(0.0);
        let fill = Self::get_string_value(sexp, "fill") == Some("solid".to_string());
        
        Ok(GraphicItem {
            item_type,
            layer,
            start,
            end,
            width,
            fill,
        })
    }

    fn parse_at(sexp: &SExp) -> Result<Position3D, PcbParseError> {
        let at_exp = sexp.get("at")
            .ok_or_else(|| PcbParseError::MissingField("at".to_string()))?;
        
        if let Some(list) = at_exp.as_list() {
            if list.len() >= 3 {
                let x = list[1]
                    .as_atom()
                    .and_then(|s| s.parse().ok())
                    .unwrap_or(0.0);
                let y = list[2]
                    .as_atom()
                    .and_then(|s| s.parse().ok())
                    .unwrap_or(0.0);
                return Ok(Position3D::new(x, y));
            }
        }
        
        Err(PcbParseError::InvalidFormat("Invalid 'at' format".to_string()))
    }

    fn parse_start(sexp: &SExp) -> Result<Position3D, PcbParseError> {
        let start_exp = sexp.get("start")
            .ok_or_else(|| PcbParseError::MissingField("start".to_string()))?;
        
        if let Some(list) = start_exp.as_list() {
            if list.len() >= 3 {
                let x = list[1]
                    .as_atom()
                    .and_then(|s| s.parse().ok())
                    .unwrap_or(0.0);
                let y = list[2]
                    .as_atom()
                    .and_then(|s| s.parse().ok())
                    .unwrap_or(0.0);
                return Ok(Position3D::new(x, y));
            }
        }
        
        Err(PcbParseError::InvalidFormat("Invalid 'start' format".to_string()))
    }

    fn parse_end(sexp: &SExp) -> Result<Position3D, PcbParseError> {
        let end_exp = sexp.get("end")
            .ok_or_else(|| PcbParseError::MissingField("end".to_string()))?;
        
        if let Some(list) = end_exp.as_list() {
            if list.len() >= 3 {
                let x = list[1]
                    .as_atom()
                    .and_then(|s| s.parse().ok())
                    .unwrap_or(0.0);
                let y = list[2]
                    .as_atom()
                    .and_then(|s| s.parse().ok())
                    .unwrap_or(0.0);
                return Ok(Position3D::new(x, y));
            }
        }
        
        Err(PcbParseError::InvalidFormat("Invalid 'end' format".to_string()))
    }

    fn parse_size(sexp: &SExp) -> Result<Size2D, PcbParseError> {
        let size_exp = sexp.get("size")
            .ok_or_else(|| PcbParseError::MissingField("size".to_string()))?;
        
        if let Some(list) = size_exp.as_list() {
            if list.len() >= 3 {
                let width = list[1]
                    .as_atom()
                    .and_then(|s| s.parse().ok())
                    .unwrap_or(0.0);
                let height = list[2]
                    .as_atom()
                    .and_then(|s| s.parse().ok())
                    .unwrap_or(0.0);
                return Ok(Size2D { width, height });
            }
        }
        
        Err(PcbParseError::InvalidFormat("Invalid 'size' format".to_string()))
    }

    fn parse_pts(sexp: &SExp) -> Vec<Position3D> {
        let mut points = Vec::new();
        
        if let Some(pts_list) = sexp.as_list() {
            for item in pts_list.iter().skip(1) {
                if let Some(xy_list) = item.as_list() {
                    if let Some(tag) = xy_list.first().and_then(|a| a.as_atom()) {
                        if tag == "xy" && xy_list.len() >= 3 {
                            let x = xy_list[1]
                                .as_atom()
                                .and_then(|s| s.parse().ok())
                                .unwrap_or(0.0);
                            let y = xy_list[2]
                                .as_atom()
                                .and_then(|s| s.parse().ok())
                                .unwrap_or(0.0);
                            points.push(Position3D::new(x, y));
                        }
                    }
                }
            }
        }
        
        points
    }
}

/// Convenience function for parsing PCB files
pub fn parse_pcb(path: &str) -> Result<PcbDesign, Box<dyn std::error::Error>> {
    PcbParser::parse_pcb(Path::new(path))
        .map_err(|e| Box::new(e) as Box<dyn std::error::Error>)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_trace_length() {
        let trace = Trace {
            uuid: "test".to_string(),
            start: Position3D::new(0.0, 0.0),
            end: Position3D::new(3.0, 4.0),
            width: 0.25,
            layer: "F.Cu".to_string(),
            net: 1,
            net_name: Some("VCC".to_string()),
            locked: false,
        };
        assert_eq!(trace.length(), 5.0);
    }

    #[test]
    fn test_trace_cross_section() {
        let trace = Trace {
            uuid: "test".to_string(),
            start: Position3D::new(0.0, 0.0),
            end: Position3D::new(10.0, 0.0),
            width: 0.5,  // 0.5mm width
            layer: "F.Cu".to_string(),
            net: 1,
            net_name: None,
            locked: false,
        };
        // 1oz copper = 0.035mm
        let area = trace.cross_section_area(0.035);
        assert!((area - 0.0175).abs() < 0.0001);
    }

    #[test]
    fn test_copper_thickness_conversion() {
        let ct = CopperThickness {
            outer: 1.0,  // 1oz
            inner: 0.5,  // 0.5oz
        };
        assert!((ct.outer_mm() - 0.035).abs() < 0.0001);
        assert!((ct.inner_mm() - 0.0175).abs() < 0.0001);
    }
}
