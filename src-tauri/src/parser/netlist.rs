//! Geometric Netlist Builder
//!
//! Builds a complete netlist by mapping component pins to nets using
//! geometric analysis of wire connections and labels. Wire segments that
//! share endpoints are grouped into the same electrical net; global labels
//! assign the net name to the whole group.

use crate::parser::schema::*;
use std::collections::{HashMap, HashSet};
use std::f64;

/// Tolerance for geometric intersection (in mm)
const INTERSECTION_TOLERANCE: f64 = 1.0;
/// Tolerance for pin-to-wire matching (mm). Pin positions are estimated from a 2.54mm grid;
/// actual symbol pins can be 5–15mm away, so use a larger tolerance so pins match wires.
const PIN_TO_WIRE_TOLERANCE: f64 = 15.0;

/// Represents a wire segment
#[derive(Debug, Clone)]
struct WireSegment {
    start: Position,
    end: Position,
    net_id: String,
}

/// Geometric netlist builder
pub struct NetlistBuilder;

impl NetlistBuilder {
    /// Build a complete netlist from schematic, mapping all component pins to nets
    pub fn build_netlist(schematic: &Schematic) -> HashMap<String, Vec<PinNetConnection>> {
        let mut pin_to_net = HashMap::new();

        // Step 1: Build wire segments
        let segments = Self::build_connectivity_graph(schematic);
        if segments.is_empty() {
            // No wires: still try to assign nets from labels to nearby pins
            let pin_positions = Self::calculate_pin_positions(schematic);
            Self::add_pins_near_labels(schematic, &pin_positions, &mut pin_to_net);
            return pin_to_net;
        }

        // Step 2: Group segments that are electrically connected (share an endpoint)
        let group_ids = Self::build_segment_groups(&segments);

        // Step 3: Assign net names to each group (from global labels on that net)
        let group_net_names = Self::assign_net_names_to_groups(schematic, &segments, &group_ids);

        // Step 4: Map segment net_id -> resolved net name
        let segment_to_net_name: HashMap<String, String> = segments
            .iter()
            .enumerate()
            .map(|(i, seg)| {
                let g = group_ids[i];
                let name = group_net_names
                    .get(&g)
                    .cloned()
                    .unwrap_or_else(|| format!("Net-{}", g));
                (seg.net_id.clone(), name)
            })
            .collect();

        // Step 5: Calculate pin positions for all components (including power symbols)
        let pin_positions = Self::calculate_pin_positions(schematic);

        // Step 6: Map pins to nets (pin near wire -> segment -> resolved net name)
        for (component_ref, pins) in &pin_positions {
            for (pin_number, pin_pos) in pins {
                if let Some(net_name) =
                    Self::find_net_name_for_position(pin_pos, &segments, &segment_to_net_name)
                {
                    let key = format!("{}:{}", component_ref, pin_number);
                    pin_to_net.insert(key, vec![PinNetConnection {
                        component_ref: component_ref.clone(),
                        pin_number: pin_number.clone(),
                        net_name: net_name.clone(),
                    }]);
                }
            }
        }

        // Step 7: Add pins that are near labels but not on wires (e.g. power symbol at label)
        Self::add_pins_near_labels(schematic, &pin_positions, &mut pin_to_net);

        pin_to_net
    }

    /// Build wire segments from schematic wires
    fn build_connectivity_graph(schematic: &Schematic) -> Vec<WireSegment> {
        let mut segments = Vec::new();

        for wire in &schematic.wires {
            if wire.points.len() < 2 {
                continue;
            }

            for i in 0..(wire.points.len() - 1) {
                let start = wire.points[i].clone();
                let end = wire.points[i + 1].clone();
                let net_id = format!("Net-W{}-{}", wire.uuid, i);
                segments.push(WireSegment {
                    start,
                    end,
                    net_id,
                });
            }
        }

        segments
    }

    /// Group segment indices by electrical connectivity (segments that share an endpoint)
    fn build_segment_groups(segments: &[WireSegment]) -> Vec<usize> {
        let n = segments.len();
        let mut parent: Vec<usize> = (0..n).collect();

        fn find(parent: &mut [usize], i: usize) -> usize {
            if parent[i] != i {
                parent[i] = find(parent, parent[i]);
            }
            parent[i]
        }

        fn unite(parent: &mut [usize], i: usize, j: usize) {
            let pi = find(parent, i);
            let pj = find(parent, j);
            if pi != pj {
                parent[pi] = pj;
            }
        }

        let tol = INTERSECTION_TOLERANCE;
        for i in 0..n {
            for j in (i + 1)..n {
                let a = &segments[i];
                let b = &segments[j];
                let share = Self::distance(&a.start, &b.start) < tol
                    || Self::distance(&a.start, &b.end) < tol
                    || Self::distance(&a.end, &b.start) < tol
                    || Self::distance(&a.end, &b.end) < tol;
                if share {
                    unite(&mut parent, i, j);
                }
            }
        }

        (0..n).map(|i| find(&mut parent, i)).collect()
    }

    /// Assign net name to each group: use global label text if label is on that net, else "Net-{g}"
    fn assign_net_names_to_groups(
        schematic: &Schematic,
        segments: &[WireSegment],
        group_ids: &[usize],
    ) -> HashMap<usize, String> {
        let mut group_net_names: HashMap<usize, String> = HashMap::new();
        let tol = PIN_TO_WIRE_TOLERANCE;

        for label in &schematic.labels {
            let net_name = match &label.label_type {
                LabelType::Global => label.text.clone(),
                LabelType::Local => format!("Net-({})", label.text),
                LabelType::Hierarchical => format!("Hier-{}", label.text),
            };

            for (seg_idx, seg) in segments.iter().enumerate() {
                let on_segment = Self::distance_point_to_segment(&label.position, &seg.start, &seg.end) < tol;
                let near_end = Self::distance(&label.position, &seg.start) < tol
                    || Self::distance(&label.position, &seg.end) < tol;
                if on_segment || near_end {
                    let g = group_ids[seg_idx];
                    group_net_names.insert(g, net_name.clone());
                    break;
                }
            }
        }

        let unique_groups: HashSet<usize> = group_ids.iter().copied().collect();
        for g in unique_groups {
            group_net_names.entry(g).or_insert_with(|| format!("Net-{}", g));
        }

        group_net_names
    }

    /// Distance from point to line segment (in mm)
    fn distance_point_to_segment(point: &Position, start: &Position, end: &Position) -> f64 {
        let dx = end.x - start.x;
        let dy = end.y - start.y;
        let length_sq = dx * dx + dy * dy;

        if length_sq < 1e-12 {
            return Self::distance(point, start);
        }

        let t = ((point.x - start.x) * dx + (point.y - start.y) * dy) / length_sq;
        let t = t.clamp(0.0, 1.0);
        let proj = Position {
            x: start.x + t * dx,
            y: start.y + t * dy,
        };
        Self::distance(point, &proj)
    }

    /// Find resolved net name for a position: nearest segment within tolerance, then lookup name
    fn find_net_name_for_position(
        position: &Position,
        segments: &[WireSegment],
        segment_to_net_name: &HashMap<String, String>,
    ) -> Option<String> {
        let mut best_net_id: Option<String> = None;
        let mut best_dist = PIN_TO_WIRE_TOLERANCE;

        for seg in segments {
            let d = Self::distance_point_to_segment(position, &seg.start, &seg.end);
            if d < best_dist {
                best_dist = d;
                best_net_id = Some(seg.net_id.clone());
            }
        }

        best_net_id.and_then(|id| segment_to_net_name.get(&id).cloned())
    }

    /// Add pin->net entries for pins that are near a label (e.g. power symbol at global label)
    fn add_pins_near_labels(
        schematic: &Schematic,
        pin_positions: &HashMap<String, HashMap<String, Position>>,
        pin_to_net: &mut HashMap<String, Vec<PinNetConnection>>,
    ) {
        let tol = PIN_TO_WIRE_TOLERANCE;
        for label in &schematic.labels {
            let net_name = match &label.label_type {
                LabelType::Global => label.text.clone(),
                LabelType::Local => format!("Net-({})", label.text),
                LabelType::Hierarchical => format!("Hier-{}", label.text),
            };

            for (component_ref, pins) in pin_positions {
                for (pin_number, pin_pos) in pins {
                    if Self::distance(pin_pos, &label.position) < tol {
                        let key = format!("{}:{}", component_ref, pin_number);
                        pin_to_net.entry(key).or_insert_with(Vec::new).push(PinNetConnection {
                            component_ref: component_ref.clone(),
                            pin_number: pin_number.clone(),
                            net_name: net_name.clone(),
                        });
                    }
                }
            }
        }
    }
    
    /// Calculate pin positions for all components
    /// Uses default pin offsets (typical 2.54mm grid for through-hole, 1.27mm for SMD)
    fn calculate_pin_positions(schematic: &Schematic) -> HashMap<String, HashMap<String, Position>> {
        let mut pin_positions = HashMap::new();
        
        for component in &schematic.components {
            let mut component_pins = HashMap::new();
            if component.pins.is_empty() {
                // Symbol instances often have no (pin ...) in file — pins come from library.
                // Use component position as single connection point so we can still match wires/labels.
                component_pins.insert("1".to_string(), component.position.clone());
            } else {
            // Default pin spacing (2.54mm typical for through-hole)
            let pin_spacing = 2.54;
            // Calculate pin positions based on component rotation and position
            let cos_r = component.rotation.to_radians().cos();
            let sin_r = component.rotation.to_radians().sin();
            for (idx, pin) in component.pins.iter().enumerate() {
                // Default pin layout: assume pins are arranged in a grid or line
                // For simplicity, use index-based positioning
                // In a real implementation, we'd need symbol library data
                let pin_offset_x = (idx as f64 % 2.0) * pin_spacing - pin_spacing / 2.0;
                let pin_offset_y = (idx as f64 / 2.0).floor() * pin_spacing - pin_spacing / 2.0;
                
                // Rotate pin offset
                let rotated_x = pin_offset_x * cos_r - pin_offset_y * sin_r;
                let rotated_y = pin_offset_x * sin_r + pin_offset_y * cos_r;
                
                // Add to component position
                let pin_pos = Position {
                    x: component.position.x + rotated_x,
                    y: component.position.y + rotated_y,
                };
                
                component_pins.insert(pin.number.clone(), pin_pos);
            }
            }
            pin_positions.insert(component.reference.clone(), component_pins);
        }
        // Also process power symbols
        for component in &schematic.power_symbols {
            let mut component_pins = HashMap::new();
            if component.pins.is_empty() {
                component_pins.insert("1".to_string(), component.position.clone());
            } else {
            for pin in &component.pins {
                component_pins.insert(pin.number.clone(), component.position.clone());
            }
            }
            pin_positions.insert(component.reference.clone(), component_pins);
        }
        
        pin_positions
    }
    
    /// Check if a point lies on a line segment (within tolerance)
    #[allow(dead_code)]
    fn point_on_segment(point: &Position, start: &Position, end: &Position) -> bool {
        let dx = end.x - start.x;
        let dy = end.y - start.y;
        let length_sq = dx * dx + dy * dy;
        
        if length_sq < 1e-6 {
            // Degenerate segment, check if point is at start/end
            return Self::distance(point, start) < INTERSECTION_TOLERANCE;
        }
        
        // Parameter t: 0 = start, 1 = end
        let t = ((point.x - start.x) * dx + (point.y - start.y) * dy) / length_sq;
        
        if t < 0.0 || t > 1.0 {
            return false;
        }
        
        // Projected point on segment
        let proj_x = start.x + t * dx;
        let proj_y = start.y + t * dy;
        
        // Distance from point to projected point
        let dist = ((point.x - proj_x).powi(2) + (point.y - proj_y).powi(2)).sqrt();
        
        dist < INTERSECTION_TOLERANCE
    }
    
    /// Calculate distance between two points
    fn distance(p1: &Position, p2: &Position) -> f64 {
        let dx = p1.x - p2.x;
        let dy = p1.y - p2.y;
        (dx * dx + dy * dy).sqrt()
    }
}

/// Connection from a pin to a net
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct PinNetConnection {
    pub component_ref: String,
    pub pin_number: String,
    pub net_name: String,
}

/// Enhanced net with pin connections
#[derive(Debug, Clone)]
pub struct EnhancedNet {
    pub name: String,
    pub pin_connections: Vec<PinNetConnection>,
}

impl EnhancedNet {
    /// Get all component references connected to this net
    pub fn connected_components(&self) -> Vec<&str> {
        self.pin_connections
            .iter()
            .map(|c| c.component_ref.as_str())
            .collect()
    }
    
    /// Check if a specific component is connected to this net
    pub fn has_component(&self, component_ref: &str) -> bool {
        self.pin_connections
            .iter()
            .any(|c| c.component_ref == component_ref)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_point_on_segment() {
        let start = Position { x: 0.0, y: 0.0 };
        let end = Position { x: 10.0, y: 0.0 };
        let point = Position { x: 5.0, y: 0.1 };
        
        assert!(NetlistBuilder::point_on_segment(&point, &start, &end));
        
        let point_far = Position { x: 5.0, y: 2.0 };
        assert!(!NetlistBuilder::point_on_segment(&point_far, &start, &end));
    }
    
    #[test]
    fn test_distance() {
        let p1 = Position { x: 0.0, y: 0.0 };
        let p2 = Position { x: 3.0, y: 4.0 };
        
        assert!((NetlistBuilder::distance(&p1, &p2) - 5.0).abs() < 1e-6);
    }
}
