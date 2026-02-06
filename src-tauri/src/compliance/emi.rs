//! EMI Analysis Module
//!
//! Performs geometric analysis of PCB layouts to detect EMI risks:
//! - High-speed traces crossing reference plane gaps
//! - Missing return paths
//! - Impedance discontinuities
//!
//! Uses geometric algorithms to analyze trace-plane relationships.

use serde::{Deserialize, Serialize};
use crate::parser::pcb_schema::{PcbDesign, Trace, Zone, Position3D, NetClassification};
use crate::compliance::net_classifier::NetClassifier;

/// EMI Risk Severity
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum EmiSeverity {
    Critical,   // High-speed signal with no reference plane
    High,       // High-speed signal crossing plane gap
    Medium,     // Clock signal with potential return path issue
    Low,        // Minor impedance discontinuity
    Info,       // Informational finding
}

/// EMI Issue found in the design
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmiIssue {
    pub id: String,
    pub severity: EmiSeverity,
    pub category: EmiCategory,
    pub net_name: String,
    pub layer: String,
    pub location: Option<Position3D>,
    pub message: String,
    pub recommendation: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum EmiCategory {
    PlaneGapCrossing,      // Trace crosses a gap in reference plane
    MissingReferencePlane, // No reference plane under high-speed trace
    ReturnPathDiscontinuity, // Return current path is broken
    LayerTransition,       // Via transition without proper return via
    ParallelHighSpeed,     // High-speed traces running parallel (crosstalk)
    UnshieldedClock,       // Clock signal without guard traces
}

/// EMI Analyzer for PCB designs
pub struct EmiAnalyzer {
    classifier: NetClassifier,
    /// Minimum gap size to consider (mm)
    #[allow(dead_code)]
    min_gap_size: f64,
    /// Distance to check for reference plane (mm)
    #[allow(dead_code)]
    plane_check_distance: f64,
}

impl Default for EmiAnalyzer {
    fn default() -> Self {
        Self {
            classifier: NetClassifier::default(),
            min_gap_size: 0.1,        // 0.1mm minimum gap
            plane_check_distance: 2.0, // Check 2mm around trace
        }
    }
}

impl EmiAnalyzer {
    pub fn new() -> Self {
        Self::default()
    }
    
    /// Analyze PCB for EMI issues
    pub fn analyze(&self, pcb: &PcbDesign) -> Vec<EmiIssue> {
        let mut issues = Vec::new();
        
        // Classify all nets
        let net_classifications = self.classifier.classify_nets(pcb);
        
        // Find high-speed and clock nets
        let critical_nets: Vec<&String> = net_classifications
            .iter()
            .filter(|(_, class)| {
                matches!(class, NetClassification::HighSpeed | NetClassification::Clock)
            })
            .map(|(name, _)| name)
            .collect();
        
        // Find ground/power planes
        let ground_zones = self.find_ground_zones(pcb);
        let power_zones = self.find_power_zones(pcb);
        
        // Check each critical trace
        for trace in &pcb.traces {
            let default_net_name = format!("Net{}", trace.net);
            let net_name = trace.net_name.as_ref()
                .unwrap_or(&default_net_name);
            
            // Skip if not a critical net
            if !critical_nets.contains(&net_name) {
                continue;
            }
            
            let classification = net_classifications.get(net_name)
                .unwrap_or(&NetClassification::Unknown);
            
            // Check for reference plane under trace
            let reference_layer = self.get_reference_layer(&trace.layer, pcb);
            
            if let Some(ref_layer) = reference_layer {
                // Check if there's a plane gap under this trace
                let gap_issues = self.check_plane_gaps(
                    trace,
                    &ref_layer,
                    &ground_zones,
                    &power_zones,
                    classification,
                );
                issues.extend(gap_issues);
            } else {
                // No reference layer found - critical issue
                issues.push(EmiIssue {
                    id: uuid::Uuid::new_v4().to_string(),
                    severity: EmiSeverity::Critical,
                    category: EmiCategory::MissingReferencePlane,
                    net_name: net_name.clone(),
                    layer: trace.layer.clone(),
                    location: Some(trace.start.clone()),
                    message: format!(
                        "High-speed net '{}' on layer {} has no adjacent reference plane",
                        net_name, trace.layer
                    ),
                    recommendation: "Add a ground or power plane on an adjacent layer to provide a return path".to_string(),
                });
            }
        }
        
        // Check for parallel high-speed traces (crosstalk)
        issues.extend(self.check_parallel_traces(pcb, &net_classifications));
        
        // Check via transitions
        issues.extend(self.check_via_transitions(pcb, &net_classifications));
        
        issues
    }
    
    /// Find the reference layer for a given signal layer
    fn get_reference_layer(&self, signal_layer: &str, pcb: &PcbDesign) -> Option<String> {
        // Standard layer stack assumptions:
        // F.Cu -> In1.Cu or B.Cu (for 2-layer)
        // B.Cu -> In(n).Cu or F.Cu (for 2-layer)
        // In1.Cu -> F.Cu or In2.Cu
        
        let layer_count = pcb.layers.iter()
            .filter(|l| l.canonical_name.contains(".Cu"))
            .count();
        
        match signal_layer {
            "F.Cu" => {
                if layer_count > 2 {
                    Some("In1.Cu".to_string())
                } else {
                    Some("B.Cu".to_string())
                }
            }
            "B.Cu" => {
                if layer_count > 2 {
                    Some(format!("In{}.Cu", layer_count - 2))
                } else {
                    Some("F.Cu".to_string())
                }
            }
            layer if layer.starts_with("In") => {
                // For inner layers, prefer the layer above
                let layer_num: Option<u32> = layer
                    .trim_start_matches("In")
                    .trim_end_matches(".Cu")
                    .parse()
                    .ok();
                
                if let Some(num) = layer_num {
                    if num == 1 {
                        Some("F.Cu".to_string())
                    } else {
                        Some(format!("In{}.Cu", num - 1))
                    }
                } else {
                    None
                }
            }
            _ => None,
        }
    }
    
    /// Find ground zones in the PCB
    fn find_ground_zones<'a>(&self, pcb: &'a PcbDesign) -> Vec<&'a Zone> {
        pcb.zones
            .iter()
            .filter(|z| {
                let name_upper = z.net_name.to_uppercase();
                name_upper == "GND" || 
                name_upper == "GROUND" || 
                name_upper == "VSS" ||
                name_upper == "AGND" ||
                name_upper == "DGND"
            })
            .collect()
    }
    
    /// Find power zones in the PCB
    fn find_power_zones<'a>(&self, pcb: &'a PcbDesign) -> Vec<&'a Zone> {
        pcb.zones
            .iter()
            .filter(|z| {
                let name_upper = z.net_name.to_uppercase();
                name_upper.contains("VCC") || 
                name_upper.contains("VDD") || 
                name_upper.contains("3V3") ||
                name_upper.contains("5V") ||
                name_upper.contains("12V") ||
                name_upper.contains("PWR")
            })
            .collect()
    }
    
    /// Check for plane gaps under a trace
    fn check_plane_gaps(
        &self,
        trace: &Trace,
        reference_layer: &str,
        ground_zones: &[&Zone],
        power_zones: &[&Zone],
        classification: &NetClassification,
    ) -> Vec<EmiIssue> {
        let mut issues = Vec::new();
        
        // Get zones on the reference layer
        let ref_zones: Vec<&&Zone> = ground_zones
            .iter()
            .chain(power_zones.iter())
            .filter(|z| z.layer == reference_layer)
            .collect();
        
        if ref_zones.is_empty() {
            // No reference plane at all on this layer
            issues.push(EmiIssue {
                id: uuid::Uuid::new_v4().to_string(),
                severity: EmiSeverity::High,
                category: EmiCategory::MissingReferencePlane,
                net_name: trace.net_name.clone().unwrap_or_default(),
                layer: trace.layer.clone(),
                location: Some(trace.start.clone()),
                message: format!(
                    "No reference plane found on {} under high-speed trace",
                    reference_layer
                ),
                recommendation: "Add a continuous ground plane on the reference layer".to_string(),
            });
            return issues;
        }
        
        // Check if trace crosses any gaps in the reference plane
        // Sample points along the trace
        let sample_points = self.sample_trace_points(trace, 10);
        
        for point in &sample_points {
            let mut has_coverage = false;
            
            for zone in &ref_zones {
                if self.point_in_zone(point, zone) {
                    has_coverage = true;
                    break;
                }
            }
            
            if !has_coverage {
                let severity = match classification {
                    NetClassification::HighSpeed => EmiSeverity::Critical,
                    NetClassification::Clock => EmiSeverity::High,
                    _ => EmiSeverity::Medium,
                };
                
                issues.push(EmiIssue {
                    id: uuid::Uuid::new_v4().to_string(),
                    severity,
                    category: EmiCategory::PlaneGapCrossing,
                    net_name: trace.net_name.clone().unwrap_or_default(),
                    layer: trace.layer.clone(),
                    location: Some(point.clone()),
                    message: format!(
                        "High-speed trace crosses gap in reference plane at ({:.2}, {:.2})",
                        point.x, point.y
                    ),
                    recommendation: "Route trace to avoid plane gaps or add stitching vias".to_string(),
                });
                
                // Only report first gap crossing per trace
                break;
            }
        }
        
        issues
    }
    
    /// Sample points along a trace
    fn sample_trace_points(&self, trace: &Trace, num_samples: usize) -> Vec<Position3D> {
        let mut points = Vec::new();
        
        for i in 0..=num_samples {
            let t = i as f64 / num_samples as f64;
            let x = trace.start.x + t * (trace.end.x - trace.start.x);
            let y = trace.start.y + t * (trace.end.y - trace.start.y);
            points.push(Position3D::new(x, y));
        }
        
        points
    }
    
    /// Check if a point is inside a zone (simplified polygon test)
    fn point_in_zone(&self, point: &Position3D, zone: &Zone) -> bool {
        // First check filled polygons (more accurate)
        for filled in &zone.filled_polygons {
            if self.point_in_polygon(point, &filled.points) {
                return true;
            }
        }
        
        // Fall back to outline
        if !zone.outline.is_empty() {
            return self.point_in_polygon(point, &zone.outline);
        }
        
        false
    }
    
    /// Ray casting algorithm for point-in-polygon test
    fn point_in_polygon(&self, point: &Position3D, polygon: &[Position3D]) -> bool {
        if polygon.len() < 3 {
            return false;
        }
        
        let mut inside = false;
        let n = polygon.len();
        let mut j = n - 1;
        
        for i in 0..n {
            let xi = polygon[i].x;
            let yi = polygon[i].y;
            let xj = polygon[j].x;
            let yj = polygon[j].y;
            
            if ((yi > point.y) != (yj > point.y)) &&
               (point.x < (xj - xi) * (point.y - yi) / (yj - yi) + xi) {
                inside = !inside;
            }
            
            j = i;
        }
        
        inside
    }
    
    /// Check for parallel high-speed traces (crosstalk risk)
    fn check_parallel_traces(
        &self,
        pcb: &PcbDesign,
        classifications: &std::collections::HashMap<String, NetClassification>,
    ) -> Vec<EmiIssue> {
        let mut issues = Vec::new();
        
        // Get high-speed traces
        let hs_traces: Vec<&Trace> = pcb.traces
            .iter()
            .filter(|t| {
                let default_net = format!("Net{}", t.net);
                let net_name = t.net_name.as_ref()
                    .unwrap_or(&default_net);
                matches!(
                    classifications.get(net_name),
                    Some(NetClassification::HighSpeed) | Some(NetClassification::Clock)
                )
            })
            .collect();
        
        // Check each pair for parallel routing
        for i in 0..hs_traces.len() {
            for j in (i + 1)..hs_traces.len() {
                let trace1 = hs_traces[i];
                let trace2 = hs_traces[j];
                
                // Skip if different layers
                if trace1.layer != trace2.layer {
                    continue;
                }
                
                // Check if traces are parallel and close
                if let Some((distance, parallel_length)) = self.check_parallel(trace1, trace2) {
                    // If parallel for more than 5mm and closer than 0.5mm
                    if parallel_length > 5.0 && distance < 0.5 {
                        issues.push(EmiIssue {
                            id: uuid::Uuid::new_v4().to_string(),
                            severity: EmiSeverity::Medium,
                            category: EmiCategory::ParallelHighSpeed,
                            net_name: format!(
                                "{} / {}",
                                trace1.net_name.as_ref().unwrap_or(&"?".to_string()),
                                trace2.net_name.as_ref().unwrap_or(&"?".to_string())
                            ),
                            layer: trace1.layer.clone(),
                            location: Some(trace1.start.clone()),
                            message: format!(
                                "High-speed traces run parallel for {:.1}mm at {:.2}mm spacing (crosstalk risk)",
                                parallel_length, distance
                            ),
                            recommendation: "Increase spacing between parallel high-speed traces or add guard traces".to_string(),
                        });
                    }
                }
            }
        }
        
        issues
    }
    
    /// Check if two traces are parallel and return (distance, parallel_length)
    fn check_parallel(&self, trace1: &Trace, trace2: &Trace) -> Option<(f64, f64)> {
        // Calculate direction vectors
        let dx1 = trace1.end.x - trace1.start.x;
        let dy1 = trace1.end.y - trace1.start.y;
        let dx2 = trace2.end.x - trace2.start.x;
        let dy2 = trace2.end.y - trace2.start.y;
        
        let len1 = (dx1 * dx1 + dy1 * dy1).sqrt();
        let len2 = (dx2 * dx2 + dy2 * dy2).sqrt();
        
        if len1 < 0.1 || len2 < 0.1 {
            return None;
        }
        
        // Normalize
        let nx1 = dx1 / len1;
        let ny1 = dy1 / len1;
        let nx2 = dx2 / len2;
        let ny2 = dy2 / len2;
        
        // Check if parallel (dot product close to 1 or -1)
        let dot = (nx1 * nx2 + ny1 * ny2).abs();
        if dot < 0.95 {
            return None;  // Not parallel
        }
        
        // Calculate perpendicular distance
        let px = trace2.start.x - trace1.start.x;
        let py = trace2.start.y - trace1.start.y;
        let distance = (px * ny1 - py * nx1).abs();
        
        // Parallel length is the minimum of the two trace lengths
        let parallel_length = len1.min(len2);
        
        Some((distance, parallel_length))
    }
    
    /// Check via transitions for high-speed signals
    fn check_via_transitions(
        &self,
        pcb: &PcbDesign,
        classifications: &std::collections::HashMap<String, NetClassification>,
    ) -> Vec<EmiIssue> {
        let mut issues = Vec::new();
        
        // Find vias on high-speed nets
        for via in &pcb.vias {
            let default_net = format!("Net{}", via.net);
            let net_name = via.net_name.as_ref()
                .unwrap_or(&default_net);
            
            let classification = classifications.get(net_name);
            if !matches!(classification, Some(NetClassification::HighSpeed)) {
                continue;
            }
            
            // Check if there's a return via nearby (within 1mm)
            let empty_string = String::new();
            let has_return_via = pcb.vias
                .iter()
                .filter(|v| v.uuid != via.uuid)
                .filter(|v| {
                    let vnet = v.net_name.as_ref().unwrap_or(&empty_string);
                    vnet.to_uppercase().contains("GND") || vnet.to_uppercase() == "VSS"
                })
                .any(|v| {
                    let dx = v.position.x - via.position.x;
                    let dy = v.position.y - via.position.y;
                    (dx * dx + dy * dy).sqrt() < 1.0
                });
            
            if !has_return_via {
                issues.push(EmiIssue {
                    id: uuid::Uuid::new_v4().to_string(),
                    severity: EmiSeverity::Medium,
                    category: EmiCategory::LayerTransition,
                    net_name: net_name.clone(),
                    layer: format!("{} -> {}", via.layers.0, via.layers.1),
                    location: Some(via.position.clone()),
                    message: format!(
                        "High-speed signal '{}' layer transition via at ({:.2}, {:.2}) has no nearby return via",
                        net_name, via.position.x, via.position.y
                    ),
                    recommendation: "Add a ground via within 1mm of high-speed signal vias".to_string(),
                });
            }
        }
        
        issues
    }
}

/// Generate EMI analysis report
pub fn generate_emi_report(pcb: &PcbDesign) -> EmiReport {
    let analyzer = EmiAnalyzer::default();
    let issues = analyzer.analyze(pcb);
    
    let critical_count = issues.iter().filter(|i| i.severity == EmiSeverity::Critical).count();
    let high_count = issues.iter().filter(|i| i.severity == EmiSeverity::High).count();
    let medium_count = issues.iter().filter(|i| i.severity == EmiSeverity::Medium).count();
    
    EmiReport {
        total_issues: issues.len(),
        critical_count,
        high_count,
        medium_count,
        issues,
        recommendations: generate_recommendations(critical_count, high_count),
    }
}

fn generate_recommendations(critical: usize, high: usize) -> Vec<String> {
    let mut recs = Vec::new();
    
    if critical > 0 {
        recs.push("CRITICAL: Address missing reference planes immediately - these will cause EMI failures".to_string());
    }
    
    if high > 0 {
        recs.push("HIGH: Review plane gap crossings and consider rerouting high-speed signals".to_string());
    }
    
    recs.push("Consider running a full SI/PI simulation for high-speed interfaces".to_string());
    recs.push("Verify impedance control requirements with your PCB manufacturer".to_string());
    
    recs
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmiReport {
    pub total_issues: usize,
    pub critical_count: usize,
    pub high_count: usize,
    pub medium_count: usize,
    pub issues: Vec<EmiIssue>,
    pub recommendations: Vec<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_point_in_polygon() {
        let analyzer = EmiAnalyzer::default();
        
        // Square polygon
        let polygon = vec![
            Position3D::new(0.0, 0.0),
            Position3D::new(10.0, 0.0),
            Position3D::new(10.0, 10.0),
            Position3D::new(0.0, 10.0),
        ];
        
        // Point inside
        assert!(analyzer.point_in_polygon(&Position3D::new(5.0, 5.0), &polygon));
        
        // Point outside
        assert!(!analyzer.point_in_polygon(&Position3D::new(15.0, 5.0), &polygon));
    }

    #[test]
    fn test_parallel_detection() {
        let analyzer = EmiAnalyzer::default();
        
        // Two parallel horizontal traces
        let trace1 = Trace {
            uuid: "t1".to_string(),
            start: Position3D::new(0.0, 0.0),
            end: Position3D::new(10.0, 0.0),
            width: 0.2,
            layer: "F.Cu".to_string(),
            net: 1,
            net_name: Some("NET1".to_string()),
            locked: false,
        };
        
        let trace2 = Trace {
            uuid: "t2".to_string(),
            start: Position3D::new(0.0, 0.3),
            end: Position3D::new(10.0, 0.3),
            width: 0.2,
            layer: "F.Cu".to_string(),
            net: 2,
            net_name: Some("NET2".to_string()),
            locked: false,
        };
        
        let result = analyzer.check_parallel(&trace1, &trace2);
        assert!(result.is_some());
        
        let (distance, length) = result.unwrap();
        assert!((distance - 0.3).abs() < 0.01);
        assert!((length - 10.0).abs() < 0.01);
    }
}
