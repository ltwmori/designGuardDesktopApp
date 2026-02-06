//! KiCad Legacy Format Parser (Versions 4-5)
//!
//! This module parses KiCad 4-5 legacy text-based schematic and PCB file formats.
//! Legacy format uses a token-based structure with different syntax than modern S-expression format.
//!
//! Key differences:
//! - Text-based tokens instead of S-expressions
//! - Coordinates in internal units (0.0001 mm per unit)
//! - Different component representation ($Comp/$EndComp blocks)
//! - Different wire representation (Wire Wire Line entries)

use std::collections::HashMap;
use crate::parser::schema::*;
use crate::parser::pcb_schema::*;
use uuid::Uuid;

/// Error type for legacy format parsing
#[derive(Debug, thiserror::Error)]
pub enum LegacyParseError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Parse error: {0}")]
    Parse(String),
    #[error("Invalid format: {0}")]
    InvalidFormat(String),
    #[error("Missing required field: {0}")]
    MissingField(String),
}

/// Parser for KiCad 4-5 legacy formats
pub struct LegacyParser;

impl LegacyParser {
    /// Parse a legacy schematic file (KiCad 4-5)
    pub fn parse_legacy_schematic(content: &str, filename: &str) -> Result<Schematic, LegacyParseError> {
        let lines: Vec<&str> = content.lines().collect();
        let mut line_idx = 0;
        
        // Parse header
        if line_idx >= lines.len() {
            return Err(LegacyParseError::InvalidFormat("File is empty".to_string()));
        }
        let version = Self::parse_header_line(lines[line_idx])?;
        line_idx += 1;
        
        // Skip EELAYER section
        while line_idx < lines.len() {
            if lines[line_idx].trim() == "EELAYER END" {
                line_idx += 1;
                break;
            }
            line_idx += 1;
        }
        
        let mut schematic = Schematic {
            uuid: Uuid::new_v4().to_string(),
            filename: filename.to_string(),
            version: Some(format!("KiCad {}", if version == 4 { "4" } else { "5" })),
            components: Vec::new(),
            wires: Vec::new(),
            labels: Vec::new(),
            nets: Vec::new(),
            power_symbols: Vec::new(),
        };
        
        // Parse $Descr block (page settings)
        while line_idx < lines.len() && !lines[line_idx].trim().starts_with("$EndDescr") {
            line_idx += 1;
        }
        if line_idx < lines.len() {
            line_idx += 1; // Skip $EndDescr
        }
        
        // Parse components, wires, labels, etc.
        while line_idx < lines.len() {
            let line = lines[line_idx].trim();
            
            if line.starts_with("$Comp") {
                line_idx += 1;
                if let Ok(component) = Self::parse_component_inline(&lines, &mut line_idx) {
                    // Check if it's a power symbol (GND, VCC, etc.)
                    let is_power = component.value.to_uppercase().contains("GND") ||
                                   component.value.to_uppercase().contains("VCC") ||
                                   component.value.to_uppercase().contains("VDD") ||
                                   component.reference.starts_with("#PWR");
                    
                    if is_power {
                        schematic.power_symbols.push(component);
                    } else {
                        schematic.components.push(component);
                    }
                }
            } else if line.starts_with("Wire Wire Line") {
                line_idx += 1;
                if line_idx < lines.len() {
                    let coord_line = lines[line_idx].trim();
                    let parts: Vec<&str> = coord_line.split_whitespace().collect();
                    if parts.len() >= 4 {
                        let x1 = parts[0].parse::<f64>().unwrap_or(0.0) * 0.0001; // Convert to mm
                        let y1 = parts[1].parse::<f64>().unwrap_or(0.0) * 0.0001;
                        let x2 = parts[2].parse::<f64>().unwrap_or(0.0) * 0.0001;
                        let y2 = parts[3].parse::<f64>().unwrap_or(0.0) * 0.0001;
                        
                        schematic.wires.push(Wire {
                            uuid: Uuid::new_v4().to_string(),
                            points: vec![
                                Position { x: x1, y: y1 },
                                Position { x: x2, y: y2 },
                            ],
                        });
                    }
                    line_idx += 1;
                }
            } else if line.starts_with("Text Label") {
                // parse_text_label_inline reads the current "Text Label ..." header
                // for coordinates, then the next line for the label text.
                if let Ok(label) = Self::parse_text_label_inline(&lines, &mut line_idx) {
                    schematic.labels.push(label);
                }
            } else if line.starts_with("$Sheet") {
                while line_idx < lines.len() && !lines[line_idx].trim().starts_with("$EndSheet") {
                    line_idx += 1;
                }
                if line_idx < lines.len() {
                    line_idx += 1; // Skip $EndSheet
                }
            } else if line.starts_with("Connection") {
                line_idx += 1;
            } else if line.starts_with("$EndSCHEMATC") {
                break;
            } else {
                line_idx += 1;
            }
        }
        
        // Build nets from connectivity
        schematic.nets = Self::build_nets_legacy(&schematic);
        
        Ok(schematic)
    }
    
    /// Parse a legacy PCB file (KiCad 4-5)
    ///
    /// Parses the PCBNEW text-based format into a fully populated `PcbDesign`.
    /// Supports $GENERAL, $SETUP, $EQUIPOT (nets), $MODULE (footprints with $PAD),
    /// $TRACK (traces and vias), $ZONE (copper pours), and $DRAWSEGMENT (graphics).
    pub fn parse_legacy_pcb(content: &str, filename: &str) -> Result<PcbDesign, LegacyParseError> {
        if !content.trim_start().starts_with("PCBNEW") {
            return Err(LegacyParseError::InvalidFormat(
                "Expected PCBNEW header in legacy PCB file".to_string()
            ));
        }

        let unit_factor = Self::detect_pcb_unit_factor(content);
        let lines: Vec<&str> = content.lines().collect();
        let mut line_idx = 1; // Skip PCBNEW header line

        let mut pcb = PcbDesign {
            uuid: Uuid::new_v4().to_string(),
            filename: filename.to_string(),
            version: Some("KiCad 4-5 (Legacy)".to_string()),
            ..Default::default()
        };

        let mut net_map: HashMap<u32, String> = HashMap::new();

        while line_idx < lines.len() {
            let line = lines[line_idx].trim();

            if line.starts_with("$GENERAL") {
                line_idx += 1;
                pcb.general = Self::parse_pcb_general(&lines, &mut line_idx);
            } else if line.starts_with("$SHEETDESCR") {
                while line_idx < lines.len()
                    && !lines[line_idx].trim().starts_with("$EndSHEETDESCR")
                {
                    line_idx += 1;
                }
                if line_idx < lines.len() {
                    line_idx += 1;
                }
            } else if line.starts_with("$SETUP") {
                line_idx += 1;
                pcb.setup = Self::parse_pcb_setup(&lines, &mut line_idx, unit_factor);
            } else if line.starts_with("$EQUIPOT") {
                line_idx += 1;
                if let Some(net) = Self::parse_pcb_equipot(&lines, &mut line_idx) {
                    net_map.insert(net.id, net.name.clone());
                    pcb.nets.push(net);
                }
            } else if line.starts_with("$MODULE") {
                line_idx += 1;
                if let Some(footprint) =
                    Self::parse_pcb_module(&lines, &mut line_idx, unit_factor)
                {
                    pcb.footprints.push(footprint);
                }
            } else if line.starts_with("$DRAWSEGMENT") {
                line_idx += 1;
                if let Some(graphic) =
                    Self::parse_pcb_draw_segment(&lines, &mut line_idx, unit_factor)
                {
                    pcb.graphics.push(graphic);
                }
            } else if line.starts_with("$TRACK") {
                line_idx += 1;
                let (traces, vias) =
                    Self::parse_pcb_track_records(&lines, &mut line_idx, unit_factor);
                pcb.traces.extend(traces);
                pcb.vias.extend(vias);
            } else if line.starts_with("$ZONE") {
                line_idx += 1;
                if let Some(zone) =
                    Self::parse_pcb_zone(&lines, &mut line_idx, unit_factor)
                {
                    pcb.zones.push(zone);
                }
            } else if line.starts_with("$EndBOARD") {
                break;
            } else {
                line_idx += 1;
            }
        }

        // Build default layers if none were parsed from $SETUP
        if pcb.layers.is_empty() {
            pcb.layers = build_default_layers();
        }

        // Post-process: resolve net names on traces, vias, and pads
        for trace in &mut pcb.traces {
            trace.net_name = net_map.get(&trace.net).cloned();
        }
        for via in &mut pcb.vias {
            via.net_name = net_map.get(&via.net).cloned();
        }
        for fp in &mut pcb.footprints {
            for pad in &mut fp.pads {
                if let Some(net_id) = pad.net {
                    pad.net_name = net_map.get(&net_id).cloned();
                }
            }
        }

        // Refresh general counts from actual parsed data
        pcb.general.modules = pcb.footprints.len() as u32;
        pcb.general.tracks = pcb.traces.len() as u32;
        pcb.general.zones = pcb.zones.len() as u32;
        pcb.general.nets = pcb.nets.len() as u32;
        pcb.general.drawings = pcb.graphics.len() as u32;

        Ok(pcb)
    }

    // ========================================================================
    // Legacy PCB block parsers
    // ========================================================================

    /// Detect the unit conversion factor for legacy PCB coordinates.
    /// Returns the factor to multiply raw integer values by to get millimetres.
    fn detect_pcb_unit_factor(content: &str) -> f64 {
        for line in content.lines() {
            let trimmed = line.trim();
            if trimmed.starts_with("InternalUnit") {
                let parts: Vec<&str> = trimmed.split_whitespace().collect();
                if parts.len() >= 3 {
                    if let Ok(factor) = parts[1].parse::<f64>() {
                        if parts[2].eq_ignore_ascii_case("INCH") {
                            return factor * 25.4; // inch-per-unit → mm-per-unit
                        } else if parts[2].eq_ignore_ascii_case("MM") {
                            return factor;
                        }
                    }
                }
            }
        }
        // Default: decimils (0.0001 inch = 0.00254 mm), standard for KiCad 4-5 legacy
        0.00254
    }

    /// Parse the `$GENERAL` block into `PcbGeneral`.
    fn parse_pcb_general(lines: &[&str], idx: &mut usize) -> PcbGeneral {
        let mut general = PcbGeneral::default();
        general.thickness = 1.6; // sensible default

        while *idx < lines.len() {
            let line = lines[*idx].trim();
            if line.starts_with("$EndGENERAL") {
                *idx += 1;
                break;
            }
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 2 {
                match parts[0] {
                    "BoardThickness" => {
                        general.thickness = parts[1].parse::<f64>().unwrap_or(1.6);
                    }
                    "Ntrack" | "Ntracks" => {
                        general.tracks = parts[1].parse::<u32>().unwrap_or(0);
                    }
                    "Nzone" | "Nzones" => {
                        general.zones = parts[1].parse::<u32>().unwrap_or(0);
                    }
                    "Nmodule" | "Nmodules" => {
                        general.modules = parts[1].parse::<u32>().unwrap_or(0);
                    }
                    "Ndraw" | "Ndraws" => {
                        general.drawings = parts[1].parse::<u32>().unwrap_or(0);
                    }
                    "Nnets" => {
                        general.nets = parts[1].parse::<u32>().unwrap_or(0);
                    }
                    _ => {}
                }
            }
            *idx += 1;
        }
        general
    }

    /// Parse the `$SETUP` block into `PcbSetup`.
    fn parse_pcb_setup(lines: &[&str], idx: &mut usize, unit_factor: f64) -> PcbSetup {
        let mut setup = PcbSetup::default();

        while *idx < lines.len() {
            let line = lines[*idx].trim();
            if line.starts_with("$EndSETUP") {
                *idx += 1;
                break;
            }
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 2 {
                if let Ok(val) = parts[1].parse::<f64>() {
                    match parts[0] {
                        "TrackMinWidth" => setup.trace_min = pcb_coord(val, unit_factor),
                        "TrackWidth" => {
                            if setup.track_width == 0.0 {
                                setup.track_width = pcb_coord(val, unit_factor);
                            }
                        }
                        "ViaSize" => setup.via_size = pcb_coord(val, unit_factor),
                        "ViaDrill" => setup.via_drill = pcb_coord(val, unit_factor),
                        "ViaMinSize" => setup.via_min_size = pcb_coord(val, unit_factor),
                        "ViaMinDrill" => setup.via_min_drill = pcb_coord(val, unit_factor),
                        "TrackClearence" | "TrackClearance" => {
                            setup.clearance = pcb_coord(val, unit_factor);
                        }
                        _ => {}
                    }
                }
            }
            *idx += 1;
        }

        // Default copper thickness if not explicitly set
        if setup.copper_thickness.outer == 0.0 {
            setup.copper_thickness.outer = 1.0; // 1 oz default
        }
        if setup.copper_thickness.inner == 0.0 {
            setup.copper_thickness.inner = 1.0;
        }
        setup
    }

    /// Parse a single `$EQUIPOT` block (net definition).
    fn parse_pcb_equipot(lines: &[&str], idx: &mut usize) -> Option<PcbNet> {
        let mut net_id = 0u32;
        let mut net_name = String::new();

        while *idx < lines.len() {
            let line = lines[*idx].trim();
            if line.starts_with("$EndEQUIPOT") {
                *idx += 1;
                break;
            }
            if line.starts_with("Na ") {
                // Format: Na <id> "<name>"
                let rest = &line[3..];
                let parts: Vec<&str> = rest.splitn(2, ' ').collect();
                if !parts.is_empty() {
                    net_id = parts[0].parse::<u32>().unwrap_or(0);
                }
                if parts.len() >= 2 {
                    net_name = parts[1].trim_matches('"').to_string();
                }
            }
            *idx += 1;
        }
        Some(PcbNet {
            id: net_id,
            name: net_name,
        })
    }

    /// Parse a `$MODULE` block (footprint with nested `$PAD` sub-blocks).
    fn parse_pcb_module(
        lines: &[&str],
        idx: &mut usize,
        unit_factor: f64,
    ) -> Option<Footprint> {
        let mut fp = Footprint {
            uuid: Uuid::new_v4().to_string(),
            reference: String::new(),
            value: String::new(),
            footprint_lib: String::new(),
            layer: "F.Cu".to_string(),
            position: Position3D::default(),
            rotation: 0.0,
            pads: Vec::new(),
            properties: HashMap::new(),
        };

        while *idx < lines.len() {
            let line = lines[*idx].trim();

            if line.starts_with("$EndMODULE") {
                *idx += 1;
                break;
            }

            if line.starts_with("Po ") {
                // Po x y orient layer timestamp attr
                let parts: Vec<&str> = line[3..].split_whitespace().collect();
                if parts.len() >= 4 {
                    let x = parts[0].parse::<f64>().unwrap_or(0.0);
                    let y = parts[1].parse::<f64>().unwrap_or(0.0);
                    fp.position = Position3D::new(
                        pcb_coord(x, unit_factor),
                        pcb_coord(y, unit_factor),
                    );
                    // Legacy stores orientation in tenths of a degree
                    fp.rotation = parts[2].parse::<f64>().unwrap_or(0.0) / 10.0;
                    let layer_num = parts[3].parse::<u32>().unwrap_or(0);
                    fp.layer = legacy_layer_to_name(layer_num);
                }
            } else if line.starts_with("Li ") {
                fp.footprint_lib = line[3..].trim().to_string();
            } else if line.starts_with("T0 ") {
                if let Some(text) = extract_legacy_text_field(line) {
                    fp.reference = text;
                }
            } else if line.starts_with("T1 ") {
                if let Some(text) = extract_legacy_text_field(line) {
                    fp.value = text;
                }
            } else if line.starts_with("Cd ") {
                fp.properties
                    .insert("Description".to_string(), line[3..].trim().to_string());
            } else if line.starts_with("Kw ") {
                fp.properties
                    .insert("Keywords".to_string(), line[3..].trim().to_string());
            } else if line.starts_with("$PAD") {
                *idx += 1;
                if let Some(pad) = Self::parse_pcb_pad(lines, idx, unit_factor) {
                    fp.pads.push(pad);
                }
                continue; // parse_pcb_pad already advanced idx past $EndPAD
            }

            *idx += 1;
        }

        if fp.reference.is_empty() {
            fp.reference = "?".to_string();
        }
        Some(fp)
    }

    /// Parse a `$PAD` sub-block inside a module.
    fn parse_pcb_pad(
        lines: &[&str],
        idx: &mut usize,
        unit_factor: f64,
    ) -> Option<Pad> {
        let mut pad = Pad {
            number: String::new(),
            pad_type: PadType::ThruHole,
            shape: PadShape::Circle,
            position: Position3D::default(),
            size: Size2D::default(),
            drill: None,
            layers: Vec::new(),
            net: None,
            net_name: None,
        };

        while *idx < lines.len() {
            let line = lines[*idx].trim();

            if line.starts_with("$EndPAD") {
                *idx += 1;
                break;
            }

            if line.starts_with("Sh ") {
                // Sh "number" shape width height dx dy orient
                let rest = &line[3..];
                if let Some(start) = rest.find('"') {
                    if let Some(end) = rest[start + 1..].find('"') {
                        pad.number = rest[start + 1..start + 1 + end].to_string();
                    }
                }
                if let Some(after_quote) = rest.rfind('"') {
                    let remaining = rest[after_quote + 1..].trim();
                    let parts: Vec<&str> = remaining.split_whitespace().collect();
                    if !parts.is_empty() {
                        pad.shape = parse_pad_shape(parts[0]);
                    }
                    if parts.len() >= 3 {
                        let w = parts[1].parse::<f64>().unwrap_or(0.0);
                        let h = parts[2].parse::<f64>().unwrap_or(0.0);
                        pad.size = Size2D {
                            width: pcb_coord(w, unit_factor),
                            height: pcb_coord(h, unit_factor),
                        };
                    }
                }
            } else if line.starts_with("Dr ") {
                // Dr diameter offset_x offset_y
                let parts: Vec<&str> = line[3..].split_whitespace().collect();
                if !parts.is_empty() {
                    let dia = parts[0].parse::<f64>().unwrap_or(0.0);
                    if dia > 0.0 {
                        let offset = if parts.len() >= 3 {
                            let ox = parts[1].parse::<f64>().unwrap_or(0.0);
                            let oy = parts[2].parse::<f64>().unwrap_or(0.0);
                            if ox != 0.0 || oy != 0.0 {
                                Some(Position3D::new(
                                    pcb_coord(ox, unit_factor),
                                    pcb_coord(oy, unit_factor),
                                ))
                            } else {
                                None
                            }
                        } else {
                            None
                        };
                        pad.drill = Some(DrillInfo {
                            diameter: pcb_coord(dia, unit_factor),
                            offset,
                        });
                    }
                }
            } else if line.starts_with("At ") {
                // At type N layer_mask
                let parts: Vec<&str> = line[3..].split_whitespace().collect();
                if !parts.is_empty() {
                    pad.pad_type = parse_pad_type(parts[0]);
                }
            } else if line.starts_with("Ne ") {
                // Ne net_number "net_name"
                let rest = &line[3..];
                let parts: Vec<&str> = rest.splitn(2, ' ').collect();
                if !parts.is_empty() {
                    pad.net = parts[0].parse::<u32>().ok();
                }
                if parts.len() >= 2 {
                    pad.net_name = Some(parts[1].trim_matches('"').to_string());
                }
            } else if line.starts_with("Po ") {
                // Po x y (pad position relative to module)
                let parts: Vec<&str> = line[3..].split_whitespace().collect();
                if parts.len() >= 2 {
                    let x = parts[0].parse::<f64>().unwrap_or(0.0);
                    let y = parts[1].parse::<f64>().unwrap_or(0.0);
                    pad.position = Position3D::new(
                        pcb_coord(x, unit_factor),
                        pcb_coord(y, unit_factor),
                    );
                }
            } else if line.starts_with("La ") {
                pad.layers = parse_layer_mask(line[3..].trim());
            }

            *idx += 1;
        }

        // Default layers based on pad type if none parsed from La line
        if pad.layers.is_empty() {
            pad.layers = match pad.pad_type {
                PadType::ThruHole | PadType::NPThruHole => {
                    vec!["F.Cu".to_string(), "B.Cu".to_string()]
                }
                PadType::SMD | PadType::Connect => {
                    vec!["F.Cu".to_string()]
                }
            };
        }

        Some(pad)
    }

    /// Parse the `$TRACK` block which contains multiple trace/via records.
    ///
    /// Each record is two consecutive lines (Po + De). Shape code 0 = trace
    /// segment, shape code 1 = via.
    fn parse_pcb_track_records(
        lines: &[&str],
        idx: &mut usize,
        unit_factor: f64,
    ) -> (Vec<Trace>, Vec<Via>) {
        let mut traces = Vec::new();
        let mut vias = Vec::new();

        while *idx < lines.len() {
            let line = lines[*idx].trim();

            if line.starts_with("$EndTRACK") {
                *idx += 1;
                break;
            }

            if line.starts_with("Po ") {
                let po_parts: Vec<&str> = line[3..].split_whitespace().collect();
                *idx += 1;

                if *idx < lines.len() {
                    let de_line = lines[*idx].trim();
                    if de_line.starts_with("De ") {
                        let de_parts: Vec<&str> = de_line[3..].split_whitespace().collect();

                        if po_parts.len() >= 6 && de_parts.len() >= 3 {
                            let shape = po_parts[0].parse::<u32>().unwrap_or(0);
                            let x1 = po_parts[1].parse::<f64>().unwrap_or(0.0);
                            let y1 = po_parts[2].parse::<f64>().unwrap_or(0.0);
                            let x2 = po_parts[3].parse::<f64>().unwrap_or(0.0);
                            let y2 = po_parts[4].parse::<f64>().unwrap_or(0.0);
                            let width = po_parts[5].parse::<f64>().unwrap_or(0.0);

                            let layer_num = de_parts[0].parse::<u32>().unwrap_or(0);
                            let track_type = de_parts[1].parse::<u32>().unwrap_or(0);
                            let net_id = de_parts[2].parse::<u32>().unwrap_or(0);
                            let status = de_parts
                                .get(4)
                                .and_then(|s| s.parse::<u32>().ok())
                                .unwrap_or(0);
                            let locked = (status & 0x01) != 0;
                            let layer_name = legacy_layer_to_name(layer_num);

                            if shape == 1 {
                                let via_type = match track_type {
                                    1 => ViaType::Blind,
                                    2 => ViaType::Buried,
                                    3 => ViaType::Micro,
                                    _ => ViaType::Through,
                                };
                                vias.push(Via {
                                    uuid: Uuid::new_v4().to_string(),
                                    position: Position3D::new(
                                        pcb_coord(x1, unit_factor),
                                        pcb_coord(y1, unit_factor),
                                    ),
                                    size: pcb_coord(width, unit_factor),
                                    drill: pcb_coord(width * 0.5, unit_factor),
                                    layers: ("F.Cu".to_string(), "B.Cu".to_string()),
                                    net: net_id,
                                    net_name: None,
                                    via_type,
                                    locked,
                                });
                            } else {
                                traces.push(Trace {
                                    uuid: Uuid::new_v4().to_string(),
                                    start: Position3D::new(
                                        pcb_coord(x1, unit_factor),
                                        pcb_coord(y1, unit_factor),
                                    ),
                                    end: Position3D::new(
                                        pcb_coord(x2, unit_factor),
                                        pcb_coord(y2, unit_factor),
                                    ),
                                    width: pcb_coord(width, unit_factor),
                                    layer: layer_name,
                                    net: net_id,
                                    net_name: None,
                                    locked,
                                });
                            }
                        }
                    }
                }
            }

            *idx += 1;
        }

        (traces, vias)
    }

    /// Parse a `$ZONE` block (copper pour).
    fn parse_pcb_zone(
        lines: &[&str],
        idx: &mut usize,
        unit_factor: f64,
    ) -> Option<Zone> {
        let mut zone = Zone {
            uuid: Uuid::new_v4().to_string(),
            net: 0,
            net_name: String::new(),
            layer: "F.Cu".to_string(),
            priority: 0,
            connect_pads: ZoneConnectType::ThermalRelief,
            min_thickness: 0.0,
            filled: false,
            outline: Vec::new(),
            filled_polygons: Vec::new(),
            keepout: None,
        };

        while *idx < lines.len() {
            let line = lines[*idx].trim();

            if line.starts_with("$EndZONE") || line.starts_with("$endZONE") {
                *idx += 1;
                break;
            }

            if line.starts_with("ZInfo") {
                let rest = line.get(6..).unwrap_or("").trim();
                let parts: Vec<&str> = rest.splitn(3, ' ').collect();
                if parts.len() >= 2 {
                    zone.net = parts[1].parse::<u32>().unwrap_or(0);
                }
                if parts.len() >= 3 {
                    zone.net_name = parts[2].trim_matches('"').to_string();
                }
            } else if line.starts_with("ZLayer ") {
                let layer_num = line[7..].trim().parse::<u32>().unwrap_or(0);
                zone.layer = legacy_layer_to_name(layer_num);
            } else if line.starts_with("ZMinThickness ") {
                let val = line[14..].trim().parse::<f64>().unwrap_or(0.0);
                zone.min_thickness = pcb_coord(val, unit_factor);
            } else if line.starts_with("ZPriority ") {
                zone.priority = line[10..].trim().parse::<u32>().unwrap_or(0);
            } else if line.starts_with("ZConnectPadsType ") {
                zone.connect_pads = match line[17..].trim() {
                    "0" => ZoneConnectType::Solid,
                    "1" => ZoneConnectType::ThermalRelief,
                    _ => ZoneConnectType::None,
                };
            } else if line.starts_with("ZCorner ") {
                let parts: Vec<&str> = line[8..].split_whitespace().collect();
                if parts.len() >= 2 {
                    let x = parts[0].parse::<f64>().unwrap_or(0.0);
                    let y = parts[1].parse::<f64>().unwrap_or(0.0);
                    zone.outline.push(Position3D::new(
                        pcb_coord(x, unit_factor),
                        pcb_coord(y, unit_factor),
                    ));
                }
            } else if line.starts_with("$POLYSCORNERS") {
                *idx += 1;
                let mut poly_points = Vec::new();
                while *idx < lines.len() {
                    let pline = lines[*idx].trim();
                    if pline.starts_with("$endPOLYSCORNERS") {
                        break;
                    }
                    let parts: Vec<&str> = pline.split_whitespace().collect();
                    if parts.len() >= 2 {
                        let x = parts[0].parse::<f64>().unwrap_or(0.0);
                        let y = parts[1].parse::<f64>().unwrap_or(0.0);
                        poly_points.push(Position3D::new(
                            pcb_coord(x, unit_factor),
                            pcb_coord(y, unit_factor),
                        ));
                        // Flag value 1 in 3rd field signals end of current polygon
                        let is_end =
                            parts.get(2).and_then(|s| s.parse::<u32>().ok()).unwrap_or(0) == 1;
                        if is_end && !poly_points.is_empty() {
                            zone.filled_polygons.push(FilledPolygon {
                                layer: zone.layer.clone(),
                                points: std::mem::take(&mut poly_points),
                            });
                            zone.filled = true;
                        }
                    }
                    *idx += 1;
                }
            }

            *idx += 1;
        }

        Some(zone)
    }

    /// Parse a `$DRAWSEGMENT` block (graphic item).
    fn parse_pcb_draw_segment(
        lines: &[&str],
        idx: &mut usize,
        unit_factor: f64,
    ) -> Option<GraphicItem> {
        let mut item = GraphicItem {
            item_type: GraphicType::Line,
            layer: "Edge.Cuts".to_string(),
            start: Position3D::default(),
            end: None,
            width: 0.0,
            fill: false,
        };

        while *idx < lines.len() {
            let line = lines[*idx].trim();

            if line.starts_with("$EndDRAWSEGMENT") {
                *idx += 1;
                break;
            }

            if line.starts_with("Po ") {
                // Po shape x1 y1 x2 y2 width
                let parts: Vec<&str> = line[3..].split_whitespace().collect();
                if parts.len() >= 6 {
                    let shape = parts[0].parse::<u32>().unwrap_or(0);
                    item.item_type = match shape {
                        0 => GraphicType::Line,
                        1 => GraphicType::Rect,
                        2 => GraphicType::Arc,
                        3 => GraphicType::Circle,
                        _ => GraphicType::Line,
                    };
                    let x1 = parts[1].parse::<f64>().unwrap_or(0.0);
                    let y1 = parts[2].parse::<f64>().unwrap_or(0.0);
                    let x2 = parts[3].parse::<f64>().unwrap_or(0.0);
                    let y2 = parts[4].parse::<f64>().unwrap_or(0.0);
                    let w = parts[5].parse::<f64>().unwrap_or(0.0);

                    item.start = Position3D::new(
                        pcb_coord(x1, unit_factor),
                        pcb_coord(y1, unit_factor),
                    );
                    item.end = Some(Position3D::new(
                        pcb_coord(x2, unit_factor),
                        pcb_coord(y2, unit_factor),
                    ));
                    item.width = pcb_coord(w, unit_factor);
                }
            } else if line.starts_with("De ") {
                let parts: Vec<&str> = line[3..].split_whitespace().collect();
                if !parts.is_empty() {
                    let layer_num = parts[0].parse::<u32>().unwrap_or(44);
                    item.layer = legacy_layer_to_name(layer_num);
                }
            }

            *idx += 1;
        }

        Some(item)
    }
    
    /// Parse file header line and extract version
    fn parse_header_line(line: &str) -> Result<u32, LegacyParseError> {
        if line.starts_with("EESchema Schematic File Version") {
            // Extract version number
            if line.contains("Version 4") {
                Ok(4)
            } else if line.contains("Version 5") {
                Ok(5)
            } else {
                // Default to 5 if version string unclear
                Ok(5)
            }
        } else {
            Err(LegacyParseError::InvalidFormat(
                "Expected EESchema header".to_string()
            ))
        }
    }
    
    /// Parse a component from $Comp block (inline version)
    fn parse_component_inline(lines: &[&str], line_idx: &mut usize) -> Result<Component, LegacyParseError>
    {
        let mut lib_id = String::new();
        let mut reference = String::new();
        let mut value = String::new();
        let mut footprint = None;
        let mut position = Position { x: 0.0, y: 0.0 };
        let rotation = 0.0;
        let mut uuid = Uuid::new_v4().to_string();
        let mut properties = HashMap::new();
        let mut pins = Vec::new();
        
        while *line_idx < lines.len() {
            let line = lines[*line_idx].trim();
            
            if line == "$EndComp" {
                *line_idx += 1;
                break;
            }
            
            if line.starts_with("L ") {
                // L Library:Component Reference
                let parts: Vec<&str> = line[2..].split_whitespace().collect();
                if !parts.is_empty() {
                    let full_lib_id = parts[0].to_string();
                    // Reference might be in the lib_id (after colon) or separate
                    if let Some((lib, ref_part)) = full_lib_id.split_once(':') {
                        lib_id = lib.to_string();
                        if !ref_part.is_empty() && !ref_part.starts_with('#') {
                            reference = ref_part.to_string();
                        }
                    } else {
                        lib_id = full_lib_id;
                    }
                }
            } else if line.starts_with("U ") {
                // U unit convert timestamp
                let parts: Vec<&str> = line[2..].split_whitespace().collect();
                if parts.len() >= 3 {
                    // Use timestamp as UUID seed
                    uuid = format!("{:08X}", parts[2].chars().take(8).fold(0u64, |acc, c| {
                        acc * 16 + c.to_digit(16).unwrap_or(0) as u64
                    }));
                }
            } else if line.starts_with("P ") {
                // P X Y
                let parts: Vec<&str> = line[2..].split_whitespace().collect();
                if parts.len() >= 2 {
                    let x = parts[0].parse::<f64>().unwrap_or(0.0) * 0.0001; // Convert to mm
                    let y = parts[1].parse::<f64>().unwrap_or(0.0) * 0.0001;
                    position = Position { x, y };
                }
            } else if line.starts_with("F ") {
                // F field_num "text" orientation X Y size flags hjust vjust style "fieldname"
                // Parse more carefully - text may contain spaces
                let rest = &line[2..];
                let mut parts = rest.split_whitespace();
                
                if let Some(field_num_str) = parts.next() {
                    if let Ok(field_num) = field_num_str.parse::<usize>() {
                        // Find the quoted text (may contain spaces)
                        let text_start = rest.find('"');
                        let text_end = rest[text_start.map(|i| i + 1).unwrap_or(0)..].find('"');
                        
                        let text = if let (Some(start), Some(end)) = (text_start, text_end) {
                            let start_idx = start + 1;
                            let end_idx = start_idx + end;
                            rest[start_idx..end_idx].to_string()
                        } else {
                            // Fallback: take next token
                            parts.next().unwrap_or("").trim_matches('"').to_string()
                        };
                        
                        match field_num {
                            0 => reference = text.clone(),
                            1 => value = text.clone(),
                            2 => footprint = Some(text.clone()),
                            3 => {
                                // Datasheet
                                properties.insert("Datasheet".to_string(), text.clone());
                            }
                            _ => {
                                // Additional fields
                                properties.insert(format!("Field{}", field_num), text.clone());
                            }
                        }
                    }
                }
            } else if line.starts_with('\t') || line.starts_with("    ") {
                // Pin information (tab-indented or space-indented)
                // Format: pin_num X Y rotation
                let parts: Vec<&str> = line.trim().split_whitespace().collect();
                if parts.len() >= 1 {
                    let pin_number = parts[0].to_string();
                    pins.push(Pin {
                        number: pin_number,
                        uuid: Uuid::new_v4().to_string(),
                    });
                }
            }
            
            *line_idx += 1;
        }
        
        // Ensure we have at least a reference
        if reference.is_empty() && !lib_id.is_empty() {
            // Try to extract from lib_id
            if let Some((_, ref_part)) = lib_id.split_once(':') {
                reference = ref_part.split_whitespace().next().unwrap_or("").to_string();
            }
        }
        
        if reference.is_empty() {
            reference = "?".to_string();
        }
        
        Ok(Component {
            uuid,
            reference,
            value: if value.is_empty() { "?".to_string() } else { value },
            lib_id: if lib_id.is_empty() { "Unknown".to_string() } else { lib_id },
            footprint,
            position,
            rotation,
            properties,
            pins,
        })
    }
    
    
    /// Parse a text label from legacy format (inline version)
    fn parse_text_label_inline(lines: &[&str], line_idx: &mut usize) -> Result<Label, LegacyParseError> {
        // Text Label X Y rotation size ~ shape
        // Label text on next line
        if *line_idx >= lines.len() {
            return Err(LegacyParseError::InvalidFormat("Missing label line".to_string()));
        }
        
        let line = lines[*line_idx].trim();
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() < 4 {
            return Err(LegacyParseError::InvalidFormat("Invalid label format".to_string()));
        }
        
        let x = parts[2].parse::<f64>().unwrap_or(0.0) * 0.0001; // Convert to mm
        let y = parts[3].parse::<f64>().unwrap_or(0.0) * 0.0001;
        let rotation = parts.get(4).and_then(|s| s.parse::<f64>().ok()).unwrap_or(0.0);
        
        // Get label text from next line
        *line_idx += 1;
        let text = if *line_idx < lines.len() {
            lines[*line_idx].trim().to_string()
        } else {
            String::new()
        };
        *line_idx += 1;
        
        // Determine label type based on text pattern
        // Global labels typically don't have special prefixes in legacy format
        // Hierarchical labels might have special syntax
        let label_type = if text.starts_with("~") || text.contains(":") {
            LabelType::Hierarchical
        } else if text.len() > 0 && text.chars().next().unwrap().is_uppercase() {
            // Common power/net labels
            LabelType::Global
        } else {
            LabelType::Local
        };
        
        Ok(Label {
            uuid: Uuid::new_v4().to_string(),
            text,
            position: Position { x, y },
            rotation,
            label_type,
        })
    }
    
    /// Build nets from legacy schematic connectivity
    fn build_nets_legacy(schematic: &Schematic) -> Vec<Net> {
        use std::collections::{HashMap, HashSet};
        
        let mut net_map: HashMap<String, HashSet<Connection>> = HashMap::new();
        
        // Process labels - they define nets
        for label in &schematic.labels {
            let net_name = match label.label_type {
                LabelType::Global => label.text.clone(),
                LabelType::Local => format!("Net-({})", label.text),
                LabelType::Hierarchical => format!("Hier-{}", label.text),
            };
            
            net_map.entry(net_name).or_insert_with(HashSet::new);
        }
        
        // Process wires - create connections
        for (wire_idx, wire) in schematic.wires.iter().enumerate() {
            if wire.points.len() >= 2 {
                let net_name = format!("Net-(W{})", wire_idx);
                let mut connections = HashSet::new();
                
                // Wire endpoints connect to components via geometric proximity
                // This is simplified - real implementation needs geometric analysis
                connections.insert(Connection {
                    component_ref: "WIRE".to_string(),
                    pin_number: format!("{}", wire_idx),
                });
                
                net_map.insert(net_name, connections);
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

// ============================================================================
// Legacy PCB standalone helper functions
// ============================================================================

/// Map a legacy numeric layer ID to the canonical KiCad layer name.
fn legacy_layer_to_name(layer_num: u32) -> String {
    match layer_num {
        0 => "F.Cu".to_string(),
        1..=30 => format!("In{}.Cu", layer_num),
        31 => "B.Cu".to_string(),
        32 => "B.Adhes".to_string(),
        33 => "F.Adhes".to_string(),
        34 => "B.Paste".to_string(),
        35 => "F.Paste".to_string(),
        36 => "B.SilkS".to_string(),
        37 => "F.SilkS".to_string(),
        38 => "B.Mask".to_string(),
        39 => "F.Mask".to_string(),
        40 => "Dwgs.User".to_string(),
        41 => "Cmts.User".to_string(),
        42 => "Eco1.User".to_string(),
        43 => "Eco2.User".to_string(),
        44 => "Edge.Cuts".to_string(),
        45 => "Margin".to_string(),
        46 => "B.CrtYd".to_string(),
        47 => "F.CrtYd".to_string(),
        48 => "B.Fab".to_string(),
        49 => "F.Fab".to_string(),
        _ => format!("User.{}", layer_num),
    }
}

/// Convert a raw coordinate value to mm using the given unit factor.
#[inline]
fn pcb_coord(val: f64, factor: f64) -> f64 {
    val * factor
}

/// Parse a legacy pad shape character to `PadShape`.
fn parse_pad_shape(shape_char: &str) -> PadShape {
    match shape_char {
        "C" => PadShape::Circle,
        "R" => PadShape::Rect,
        "O" => PadShape::Oval,
        "T" => PadShape::Trapezoid,
        _ => PadShape::Circle,
    }
}

/// Parse a legacy pad type string to `PadType`.
fn parse_pad_type(type_str: &str) -> PadType {
    match type_str {
        "STD" => PadType::ThruHole,
        "SMD" => PadType::SMD,
        "CONN" => PadType::Connect,
        "HOLE" => PadType::NPThruHole,
        _ => PadType::ThruHole,
    }
}

/// Extract the quoted text from a legacy T0/T1 field line.
fn extract_legacy_text_field(line: &str) -> Option<String> {
    // Format: T0 x y xsize ysize rot penwidth N visible layer "text"
    // The text is the last quoted string on the line.
    let end = line.rfind('"')?;
    let start = line[..end].rfind('"')?;
    Some(line[start + 1..end].to_string())
}

/// Parse a hex layer mask string into a list of canonical layer names.
fn parse_layer_mask(mask_str: &str) -> Vec<String> {
    let cleaned = mask_str
        .trim()
        .trim_start_matches("0x")
        .trim_start_matches("0X");
    if let Ok(mask) = u64::from_str_radix(cleaned, 16) {
        let mut layers = Vec::new();
        for bit in 0..50u32 {
            if mask & (1u64 << bit) != 0 {
                layers.push(legacy_layer_to_name(bit));
            }
        }
        if !layers.is_empty() {
            return layers;
        }
    }
    // Fallback: space-separated numeric layer IDs
    mask_str
        .split_whitespace()
        .filter_map(|p| p.parse::<u32>().ok())
        .map(legacy_layer_to_name)
        .collect()
}

/// Build default two-layer copper stack for a legacy PCB.
fn build_default_layers() -> Vec<PcbLayer> {
    vec![
        PcbLayer {
            ordinal: 0,
            canonical_name: "F.Cu".to_string(),
            layer_type: LayerType::Signal,
            user_name: None,
        },
        PcbLayer {
            ordinal: 31,
            canonical_name: "B.Cu".to_string(),
            layer_type: LayerType::Signal,
            user_name: None,
        },
    ]
}

// Conversion from LegacyParseError to standard errors
impl From<LegacyParseError> for String {
    fn from(err: LegacyParseError) -> Self {
        err.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ====================================================================
    // Schematic tests (existing, preserved)
    // ====================================================================

    #[test]
    fn test_parse_legacy_schematic_basic() {
        let content = r#"EESchema Schematic File Version 4
EELAYER 30 0
EELAYER END
$Descr A3 16535 11693
encoding utf-8
Sheet 1 5
Title "Test"
$EndDescr
$Comp
L Device:R R1
U 1 1 561E4EB0
P 1200 8900
F 0 "R1" H 1200 8650 50  0001 C CNN
F 1 "10k" H 1200 8750 50  0000 C CNN
$EndComp
Wire Wire Line
	800  6800 2200 6800
Text Label 5000 3300 0    60   ~ 0
VCC
$EndSCHEMATC"#;

        let result = LegacyParser::parse_legacy_schematic(content, "test.sch");
        assert!(result.is_ok());

        let schematic = result.unwrap();
        assert_eq!(schematic.components.len(), 1);
        assert_eq!(schematic.components[0].reference, "R1");
        assert_eq!(schematic.components[0].value, "10k");
        assert_eq!(schematic.wires.len(), 1);
        assert!(!schematic.labels.is_empty());
    }

    // ====================================================================
    // Standalone helper tests
    // ====================================================================

    #[test]
    fn test_legacy_layer_to_name() {
        assert_eq!(legacy_layer_to_name(0), "F.Cu");
        assert_eq!(legacy_layer_to_name(31), "B.Cu");
        assert_eq!(legacy_layer_to_name(1), "In1.Cu");
        assert_eq!(legacy_layer_to_name(15), "In15.Cu");
        assert_eq!(legacy_layer_to_name(37), "F.SilkS");
        assert_eq!(legacy_layer_to_name(44), "Edge.Cuts");
    }

    #[test]
    fn test_pcb_coord_conversion() {
        // Default decimil factor: 0.00254 mm per unit
        let factor = 0.00254;
        let val = 10000.0; // 10000 decimils = 1 inch = 25.4 mm
        let mm = pcb_coord(val, factor);
        assert!((mm - 25.4).abs() < 1e-6);
    }

    #[test]
    fn test_parse_pad_shape_mapping() {
        assert_eq!(parse_pad_shape("C"), PadShape::Circle);
        assert_eq!(parse_pad_shape("R"), PadShape::Rect);
        assert_eq!(parse_pad_shape("O"), PadShape::Oval);
        assert_eq!(parse_pad_shape("T"), PadShape::Trapezoid);
        assert_eq!(parse_pad_shape("Z"), PadShape::Circle); // unknown defaults
    }

    #[test]
    fn test_parse_pad_type_mapping() {
        assert_eq!(parse_pad_type("STD"), PadType::ThruHole);
        assert_eq!(parse_pad_type("SMD"), PadType::SMD);
        assert_eq!(parse_pad_type("CONN"), PadType::Connect);
        assert_eq!(parse_pad_type("HOLE"), PadType::NPThruHole);
    }

    #[test]
    fn test_extract_legacy_text_field() {
        let line = r#"T0 1000 -2000 600 600 0 120 N V 21 "R1""#;
        assert_eq!(extract_legacy_text_field(line), Some("R1".to_string()));

        let line2 = r#"T1 1000 2000 600 600 0 120 N V 21 "10k""#;
        assert_eq!(extract_legacy_text_field(line2), Some("10k".to_string()));
    }

    #[test]
    fn test_parse_layer_mask_hex() {
        // 0x00000001 = bit 0 = F.Cu
        let layers = parse_layer_mask("00000001");
        assert!(layers.contains(&"F.Cu".to_string()));

        // 0x80000001 = bit 0 (F.Cu) + bit 31 (B.Cu)
        let layers = parse_layer_mask("80000001");
        assert!(layers.contains(&"F.Cu".to_string()));
        assert!(layers.contains(&"B.Cu".to_string()));
    }

    #[test]
    fn test_detect_pcb_unit_factor_decimil() {
        let content = "PCBNEW\n$SETUP\nInternalUnit 0.000100 INCH\n$EndSETUP\n";
        let factor = LegacyParser::detect_pcb_unit_factor(content);
        assert!((factor - 0.00254).abs() < 1e-9);
    }

    #[test]
    fn test_detect_pcb_unit_factor_default() {
        let content = "PCBNEW\nnothing special\n";
        let factor = LegacyParser::detect_pcb_unit_factor(content);
        assert!((factor - 0.00254).abs() < 1e-9);
    }

    // ====================================================================
    // PCB parsing integration tests
    // ====================================================================

    #[test]
    fn test_parse_legacy_pcb_rejects_bad_header() {
        let content = "NOT_A_PCB_FILE\n";
        let result = LegacyParser::parse_legacy_pcb(content, "bad.brd");
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_legacy_pcb_minimal() {
        let content = r#"PCBNEW-BOARD Version 1 date 2024/01/01
$GENERAL
BoardThickness 1.6
Nnets 3
$EndGENERAL
$SETUP
InternalUnit 0.000100 INCH
TrackWidth 250
ViaSize 600
ViaDrill 300
TrackClearence 200
$EndSETUP
$EQUIPOT
Na 0 ""
St ~
$EndEQUIPOT
$EQUIPOT
Na 1 "VCC"
St ~
$EndEQUIPOT
$EQUIPOT
Na 2 "GND"
St ~
$EndEQUIPOT
$MODULE R_0402
Po 5000 3000 0 0 00000000 00000000
Li Resistor_SMD:R_0402
T0 0 -1000 600 600 0 120 N V 21 "R1"
T1 0 1000 600 600 0 120 N V 21 "10k"
$PAD
Sh "1" R 400 500 0 0 0
Dr 0 0 0
At SMD N 00888000
Ne 1 "VCC"
Po -450 0
$EndPAD
$PAD
Sh "2" R 400 500 0 0 0
Dr 0 0 0
At SMD N 00888000
Ne 2 "GND"
Po 450 0
$EndPAD
$EndMODULE R_0402
$DRAWSEGMENT
Po 0 0 0 10000 10000 150
De 44 0 0 00000000 0
$EndDRAWSEGMENT
$TRACK
Po 0 5000 3000 7000 3000 250
De 0 0 1 00000000 0
Po 1 7000 3000 7000 3000 600
De 0 0 2 00000000 0
$EndTRACK
$ZONE
ZInfo 00000000 1 "VCC"
ZLayer 0
ZMinThickness 250
ZPriority 0
ZCorner 0 0 0
ZCorner 10000 0 0
ZCorner 10000 10000 0
ZCorner 0 10000 1
$EndZONE
$EndBOARD
"#;

        let result = LegacyParser::parse_legacy_pcb(content, "test.brd");
        assert!(result.is_ok(), "Parse failed: {:?}", result.err());

        let pcb = result.unwrap();

        // General
        assert!((pcb.general.thickness - 1.6).abs() < 1e-6);

        // Nets
        assert_eq!(pcb.nets.len(), 3);
        assert_eq!(pcb.nets[1].name, "VCC");
        assert_eq!(pcb.nets[2].name, "GND");

        // Footprints
        assert_eq!(pcb.footprints.len(), 1);
        let fp = &pcb.footprints[0];
        assert_eq!(fp.reference, "R1");
        assert_eq!(fp.value, "10k");
        assert_eq!(fp.pads.len(), 2);
        assert_eq!(fp.pads[0].number, "1");
        assert_eq!(fp.pads[0].pad_type, PadType::SMD);
        assert_eq!(fp.pads[0].shape, PadShape::Rect);
        assert_eq!(fp.pads[0].net_name, Some("VCC".to_string()));
        assert_eq!(fp.pads[1].net_name, Some("GND".to_string()));

        // Traces
        assert_eq!(pcb.traces.len(), 1);
        assert_eq!(pcb.traces[0].layer, "F.Cu");
        assert_eq!(pcb.traces[0].net_name, Some("VCC".to_string()));

        // Vias
        assert_eq!(pcb.vias.len(), 1);
        assert_eq!(pcb.vias[0].via_type, ViaType::Through);
        assert_eq!(pcb.vias[0].net_name, Some("GND".to_string()));

        // Graphics
        assert_eq!(pcb.graphics.len(), 1);
        assert_eq!(pcb.graphics[0].layer, "Edge.Cuts");
        assert_eq!(pcb.graphics[0].item_type, GraphicType::Line);

        // Zones
        assert_eq!(pcb.zones.len(), 1);
        assert_eq!(pcb.zones[0].net_name, "VCC");
        assert_eq!(pcb.zones[0].layer, "F.Cu");
        assert_eq!(pcb.zones[0].outline.len(), 4);

        // Setup
        let factor = 0.00254;
        assert!((pcb.setup.track_width - pcb_coord(250.0, factor)).abs() < 1e-6);
        assert!((pcb.setup.via_size - pcb_coord(600.0, factor)).abs() < 1e-6);
    }

    #[test]
    fn test_parse_legacy_pcb_empty_board() {
        let content = "PCBNEW-BOARD Version 1\n$EndBOARD\n";
        let result = LegacyParser::parse_legacy_pcb(content, "empty.brd");
        assert!(result.is_ok());

        let pcb = result.unwrap();
        assert!(pcb.footprints.is_empty());
        assert!(pcb.traces.is_empty());
        assert!(pcb.vias.is_empty());
        assert!(!pcb.layers.is_empty()); // default layers added
    }

    #[test]
    fn test_parse_legacy_pcb_missing_blocks() {
        // Only nets, no modules or tracks -- should parse gracefully
        let content = r#"PCBNEW-BOARD Version 1
$EQUIPOT
Na 1 "NET1"
St ~
$EndEQUIPOT
$EndBOARD
"#;
        let result = LegacyParser::parse_legacy_pcb(content, "partial.brd");
        assert!(result.is_ok());

        let pcb = result.unwrap();
        assert_eq!(pcb.nets.len(), 1);
        assert_eq!(pcb.nets[0].name, "NET1");
        assert!(pcb.footprints.is_empty());
    }
}
