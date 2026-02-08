//! Custom Rules Engine
//!
//! Allows companies to define custom design rules in JSON format.
//! Rules can check various aspects of PCB design including:
//! - Trace widths and clearances
//! - Via sizes and placement
//! - Component placement
//! - Net-specific requirements
//! - Manufacturing constraints

use serde::{Deserialize, Serialize};
use std::path::Path;
use crate::parser::pcb_schema::*;

/// Custom rule definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CustomRule {
    /// Unique rule identifier
    pub id: String,
    /// Human-readable rule name
    pub name: String,
    /// Detailed description
    pub description: String,
    /// Rule category
    pub category: RuleCategory,
    /// Severity if rule is violated
    pub severity: RuleSeverity,
    /// Rule condition/check type
    pub check: RuleCheck,
    /// Whether rule is enabled
    #[serde(default = "default_true")]
    pub enabled: bool,
}

fn default_true() -> bool {
    true
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum RuleCategory {
    Manufacturing,
    Signal,
    Power,
    Thermal,
    Mechanical,
    Safety,
    Custom,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum RuleSeverity {
    Error,
    Warning,
    Info,
}

/// Rule check types
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum RuleCheck {
    /// Minimum trace width check
    MinTraceWidth {
        min_width_mm: f64,
        #[serde(default)]
        layer_filter: Option<String>,
        #[serde(default)]
        net_filter: Option<String>,
    },
    
    /// Maximum trace width check
    MaxTraceWidth {
        max_width_mm: f64,
        #[serde(default)]
        layer_filter: Option<String>,
        #[serde(default)]
        net_filter: Option<String>,
    },
    
    /// Minimum via size check
    MinViaSize {
        min_size_mm: f64,
        min_drill_mm: f64,
    },
    
    /// Minimum via-to-via spacing
    MinViaSpacing {
        min_spacing_mm: f64,
    },
    
    /// Minimum trace-to-trace clearance
    MinClearance {
        min_clearance_mm: f64,
        #[serde(default)]
        net_class_filter: Option<String>,
    },
    
    /// Net must have reference plane
    RequireReferencePlane {
        net_pattern: String,
    },
    
    /// Component must exist
    RequireComponent {
        reference_pattern: String,
        value_pattern: Option<String>,
        message: String,
    },
    
    /// Mounting holes must be grounded
    MountingHolesGrounded {
        #[serde(default)]
        min_connections: u32,
    },
    
    /// Fiducials required
    RequireFiducials {
        min_count: u32,
    },
    
    /// Board edge clearance
    BoardEdgeClearance {
        min_clearance_mm: f64,
    },
    
    /// Power trace width based on current
    PowerTraceWidth {
        net_pattern: String,
        expected_current_a: f64,
        temp_rise_c: f64,
    },
    
    /// Differential pair matching
    DifferentialPairMatch {
        net_pattern_p: String,
        net_pattern_n: String,
        max_length_diff_mm: f64,
        max_spacing_diff_mm: f64,
    },
    
    /// Custom expression (advanced)
    Expression {
        expression: String,
        message: String,
    },
}

/// Rule set containing multiple rules
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuleSet {
    /// Rule set name
    pub name: String,
    /// Version
    pub version: String,
    /// Description
    pub description: Option<String>,
    /// Company/author
    pub author: Option<String>,
    /// Rules in this set
    pub rules: Vec<CustomRule>,
    /// Global settings
    #[serde(default)]
    pub settings: RuleSettings,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct RuleSettings {
    /// Default copper thickness (oz)
    #[serde(default = "default_copper")]
    pub copper_oz: f64,
    /// Default temperature rise for current calculations
    #[serde(default = "default_temp_rise")]
    pub temp_rise_c: f64,
    /// Unit system (mm or mil)
    #[serde(default = "default_unit")]
    pub unit: String,
}

fn default_copper() -> f64 {
    1.0
}

fn default_temp_rise() -> f64 {
    10.0
}

fn default_unit() -> String {
    "mm".to_string()
}

/// Rule violation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuleViolation {
    pub rule_id: String,
    pub rule_name: String,
    pub severity: RuleSeverity,
    pub category: RuleCategory,
    pub message: String,
    pub location: Option<Position3D>,
    pub affected_items: Vec<String>,
    pub suggestion: Option<String>,
}

/// Custom rules engine
#[derive(Clone)]
pub struct CustomRulesEngine {
    rule_sets: Vec<RuleSet>,
}

impl CustomRulesEngine {
    pub fn new() -> Self {
        Self {
            rule_sets: Vec::new(),
        }
    }
    
    /// Load rules from JSON file
    pub fn load_rules_file(&mut self, path: &Path) -> Result<(), String> {
        let content = std::fs::read_to_string(path)
            .map_err(|e| format!("Failed to read rules file: {}", e))?;
        
        let rule_set: RuleSet = serde_json::from_str(&content)
            .map_err(|e| format!("Failed to parse rules JSON: {}", e))?;
        
        self.rule_sets.push(rule_set);
        Ok(())
    }
    
    /// Load rules from JSON string
    pub fn load_rules_str(&mut self, json: &str) -> Result<(), String> {
        let rule_set: RuleSet = serde_json::from_str(json)
            .map_err(|e| format!("Failed to parse rules JSON: {}", e))?;
        
        self.rule_sets.push(rule_set);
        Ok(())
    }
    
    /// Add a rule set directly
    pub fn add_rule_set(&mut self, rule_set: RuleSet) {
        self.rule_sets.push(rule_set);
    }
    
    /// Check PCB against all loaded rules
    pub fn check(&self, pcb: &PcbDesign) -> Vec<RuleViolation> {
        let mut violations = Vec::new();
        
        for rule_set in &self.rule_sets {
            for rule in &rule_set.rules {
                if !rule.enabled {
                    continue;
                }
                
                let rule_violations = self.check_rule(pcb, rule, &rule_set.settings);
                violations.extend(rule_violations);
            }
        }
        
        violations
    }
    
    /// Check a single rule
    fn check_rule(&self, pcb: &PcbDesign, rule: &CustomRule, settings: &RuleSettings) -> Vec<RuleViolation> {
        match &rule.check {
            RuleCheck::MinTraceWidth { min_width_mm, layer_filter, net_filter } => {
                self.check_min_trace_width(pcb, rule, *min_width_mm, layer_filter, net_filter)
            }
            RuleCheck::MaxTraceWidth { max_width_mm, layer_filter, net_filter } => {
                self.check_max_trace_width(pcb, rule, *max_width_mm, layer_filter, net_filter)
            }
            RuleCheck::MinViaSize { min_size_mm, min_drill_mm } => {
                self.check_min_via_size(pcb, rule, *min_size_mm, *min_drill_mm)
            }
            RuleCheck::MinViaSpacing { min_spacing_mm } => {
                self.check_min_via_spacing(pcb, rule, *min_spacing_mm)
            }
            RuleCheck::RequireReferencePlane { net_pattern } => {
                self.check_reference_plane(pcb, rule, net_pattern)
            }
            RuleCheck::RequireComponent { reference_pattern, value_pattern, message } => {
                self.check_require_component(pcb, rule, reference_pattern, value_pattern, message)
            }
            RuleCheck::MountingHolesGrounded { min_connections } => {
                self.check_mounting_holes_grounded(pcb, rule, *min_connections)
            }
            RuleCheck::RequireFiducials { min_count } => {
                self.check_fiducials(pcb, rule, *min_count)
            }
            RuleCheck::PowerTraceWidth { net_pattern, expected_current_a, temp_rise_c } => {
                self.check_power_trace_width(pcb, rule, net_pattern, *expected_current_a, *temp_rise_c, settings)
            }
            RuleCheck::DifferentialPairMatch { net_pattern_p, net_pattern_n, max_length_diff_mm, max_spacing_diff_mm: _ } => {
                self.check_differential_pair(pcb, rule, net_pattern_p, net_pattern_n, *max_length_diff_mm)
            }
            _ => Vec::new(), // Other checks not yet implemented
        }
    }
    
    fn check_min_trace_width(
        &self,
        pcb: &PcbDesign,
        rule: &CustomRule,
        min_width: f64,
        layer_filter: &Option<String>,
        net_filter: &Option<String>,
    ) -> Vec<RuleViolation> {
        let mut violations = Vec::new();
        
        for trace in &pcb.traces {
            // Apply filters
            if let Some(layer) = layer_filter {
                if !trace.layer.contains(layer) {
                    continue;
                }
            }
            
            if let Some(net) = net_filter {
                let empty_string = String::new();
                let net_name = trace.net_name.as_ref().unwrap_or(&empty_string);
                if !net_name.to_uppercase().contains(&net.to_uppercase()) {
                    continue;
                }
            }
            
            if trace.width < min_width {
                violations.push(RuleViolation {
                    rule_id: rule.id.clone(),
                    rule_name: rule.name.clone(),
                    severity: rule.severity.clone(),
                    category: rule.category.clone(),
                    message: format!(
                        "Trace width {:.3}mm is below minimum {:.3}mm on layer {}",
                        trace.width, min_width, trace.layer
                    ),
                    location: Some(trace.start.clone()),
                    affected_items: vec![trace.uuid.clone()],
                    suggestion: Some(format!("Increase trace width to at least {:.3}mm", min_width)),
                });
            }
        }
        
        violations
    }
    
    fn check_max_trace_width(
        &self,
        pcb: &PcbDesign,
        rule: &CustomRule,
        max_width: f64,
        layer_filter: &Option<String>,
        net_filter: &Option<String>,
    ) -> Vec<RuleViolation> {
        let mut violations = Vec::new();
        
        for trace in &pcb.traces {
            if let Some(layer) = layer_filter {
                if !trace.layer.contains(layer) {
                    continue;
                }
            }
            
            if let Some(net) = net_filter {
                let empty_string = String::new();
                let net_name = trace.net_name.as_ref().unwrap_or(&empty_string);
                if !net_name.to_uppercase().contains(&net.to_uppercase()) {
                    continue;
                }
            }
            
            if trace.width > max_width {
                violations.push(RuleViolation {
                    rule_id: rule.id.clone(),
                    rule_name: rule.name.clone(),
                    severity: rule.severity.clone(),
                    category: rule.category.clone(),
                    message: format!(
                        "Trace width {:.3}mm exceeds maximum {:.3}mm on layer {}",
                        trace.width, max_width, trace.layer
                    ),
                    location: Some(trace.start.clone()),
                    affected_items: vec![trace.uuid.clone()],
                    suggestion: Some(format!("Reduce trace width to at most {:.3}mm", max_width)),
                });
            }
        }
        
        violations
    }
    
    fn check_min_via_size(
        &self,
        pcb: &PcbDesign,
        rule: &CustomRule,
        min_size: f64,
        min_drill: f64,
    ) -> Vec<RuleViolation> {
        let mut violations = Vec::new();
        
        for via in &pcb.vias {
            if via.size < min_size {
                violations.push(RuleViolation {
                    rule_id: rule.id.clone(),
                    rule_name: rule.name.clone(),
                    severity: rule.severity.clone(),
                    category: rule.category.clone(),
                    message: format!(
                        "Via size {:.3}mm is below minimum {:.3}mm",
                        via.size, min_size
                    ),
                    location: Some(via.position.clone()),
                    affected_items: vec![via.uuid.clone()],
                    suggestion: Some(format!("Increase via size to at least {:.3}mm", min_size)),
                });
            }
            
            if via.drill < min_drill {
                violations.push(RuleViolation {
                    rule_id: rule.id.clone(),
                    rule_name: rule.name.clone(),
                    severity: rule.severity.clone(),
                    category: rule.category.clone(),
                    message: format!(
                        "Via drill {:.3}mm is below minimum {:.3}mm",
                        via.drill, min_drill
                    ),
                    location: Some(via.position.clone()),
                    affected_items: vec![via.uuid.clone()],
                    suggestion: Some(format!("Increase via drill to at least {:.3}mm", min_drill)),
                });
            }
        }
        
        violations
    }
    
    fn check_min_via_spacing(
        &self,
        pcb: &PcbDesign,
        rule: &CustomRule,
        min_spacing: f64,
    ) -> Vec<RuleViolation> {
        let mut violations = Vec::new();
        
        for i in 0..pcb.vias.len() {
            for j in (i + 1)..pcb.vias.len() {
                let via1 = &pcb.vias[i];
                let via2 = &pcb.vias[j];
                
                let dx = via2.position.x - via1.position.x;
                let dy = via2.position.y - via1.position.y;
                let distance = (dx * dx + dy * dy).sqrt();
                
                // Edge-to-edge spacing
                let edge_spacing = distance - (via1.size + via2.size) / 2.0;
                
                if edge_spacing < min_spacing {
                    violations.push(RuleViolation {
                        rule_id: rule.id.clone(),
                        rule_name: rule.name.clone(),
                        severity: rule.severity.clone(),
                        category: rule.category.clone(),
                        message: format!(
                            "Via spacing {:.3}mm is below minimum {:.3}mm",
                            edge_spacing, min_spacing
                        ),
                        location: Some(via1.position.clone()),
                        affected_items: vec![via1.uuid.clone(), via2.uuid.clone()],
                        suggestion: Some(format!("Increase via spacing to at least {:.3}mm", min_spacing)),
                    });
                }
            }
        }
        
        violations
    }
    
    fn check_reference_plane(
        &self,
        pcb: &PcbDesign,
        rule: &CustomRule,
        net_pattern: &str,
    ) -> Vec<RuleViolation> {
        let mut violations = Vec::new();
        let pattern_upper = net_pattern.to_uppercase();
        
        // Find traces matching the pattern
        let matching_traces: Vec<&Trace> = pcb.traces
            .iter()
            .filter(|t| {
                t.net_name.as_ref()
                    .map(|n| n.to_uppercase().contains(&pattern_upper))
                    .unwrap_or(false)
            })
            .collect();
        
        if matching_traces.is_empty() {
            return violations;
        }
        
        // Check if there's a ground plane
        let has_ground_plane = pcb.zones
            .iter()
            .any(|z| {
                let name_upper = z.net_name.to_uppercase();
                name_upper == "GND" || name_upper == "GROUND" || name_upper == "VSS"
            });
        
        if !has_ground_plane {
            violations.push(RuleViolation {
                rule_id: rule.id.clone(),
                rule_name: rule.name.clone(),
                severity: rule.severity.clone(),
                category: rule.category.clone(),
                message: format!(
                    "Nets matching '{}' require a reference plane, but no ground plane found",
                    net_pattern
                ),
                location: matching_traces.first().map(|t| t.start.clone()),
                affected_items: matching_traces.iter().map(|t| t.uuid.clone()).collect(),
                suggestion: Some("Add a ground plane on an adjacent layer".to_string()),
            });
        }
        
        violations
    }
    
    fn check_require_component(
        &self,
        pcb: &PcbDesign,
        rule: &CustomRule,
        reference_pattern: &str,
        value_pattern: &Option<String>,
        message: &str,
    ) -> Vec<RuleViolation> {
        let mut violations = Vec::new();
        let ref_pattern_upper = reference_pattern.to_uppercase();
        
        let found = pcb.footprints.iter().any(|fp| {
            let ref_matches = fp.reference.to_uppercase().contains(&ref_pattern_upper);
            
            if let Some(val_pattern) = value_pattern {
                let val_pattern_upper = val_pattern.to_uppercase();
                ref_matches && fp.value.to_uppercase().contains(&val_pattern_upper)
            } else {
                ref_matches
            }
        });
        
        if !found {
            violations.push(RuleViolation {
                rule_id: rule.id.clone(),
                rule_name: rule.name.clone(),
                severity: rule.severity.clone(),
                category: rule.category.clone(),
                message: message.to_string(),
                location: None,
                affected_items: Vec::new(),
                suggestion: Some(format!("Add component matching reference '{}'", reference_pattern)),
            });
        }
        
        violations
    }
    
    fn check_mounting_holes_grounded(
        &self,
        pcb: &PcbDesign,
        rule: &CustomRule,
        _min_connections: u32,
    ) -> Vec<RuleViolation> {
        let mut violations = Vec::new();
        
        // Find mounting holes (typically reference starts with 'H' or 'MH')
        let mounting_holes: Vec<&Footprint> = pcb.footprints
            .iter()
            .filter(|fp| {
                let ref_upper = fp.reference.to_uppercase();
                ref_upper.starts_with('H') || 
                ref_upper.starts_with("MH") ||
                fp.footprint_lib.to_uppercase().contains("MOUNTING")
            })
            .collect();
        
        for hole in mounting_holes {
            // Check if any pad is connected to ground
            let grounded = hole.pads.iter().any(|pad| {
                pad.net_name.as_ref()
                    .map(|n| {
                        let upper = n.to_uppercase();
                        upper == "GND" || upper == "GROUND" || upper == "VSS"
                    })
                    .unwrap_or(false)
            });
            
            if !grounded {
                violations.push(RuleViolation {
                    rule_id: rule.id.clone(),
                    rule_name: rule.name.clone(),
                    severity: rule.severity.clone(),
                    category: rule.category.clone(),
                    message: format!(
                        "Mounting hole {} is not connected to ground",
                        hole.reference
                    ),
                    location: Some(hole.position.clone()),
                    affected_items: vec![hole.uuid.clone()],
                    suggestion: Some("Connect mounting hole to GND for EMI shielding".to_string()),
                });
            }
        }
        
        violations
    }
    
    fn check_fiducials(
        &self,
        pcb: &PcbDesign,
        rule: &CustomRule,
        min_count: u32,
    ) -> Vec<RuleViolation> {
        let mut violations = Vec::new();
        
        // Count fiducials (typically reference starts with 'FID')
        let fiducial_count = pcb.footprints
            .iter()
            .filter(|fp| {
                let ref_upper = fp.reference.to_uppercase();
                ref_upper.starts_with("FID") ||
                fp.footprint_lib.to_uppercase().contains("FIDUCIAL")
            })
            .count() as u32;
        
        if fiducial_count < min_count {
            violations.push(RuleViolation {
                rule_id: rule.id.clone(),
                rule_name: rule.name.clone(),
                severity: rule.severity.clone(),
                category: rule.category.clone(),
                message: format!(
                    "Found {} fiducials, but {} required for assembly",
                    fiducial_count, min_count
                ),
                location: None,
                affected_items: Vec::new(),
                suggestion: Some(format!("Add at least {} fiducial markers", min_count)),
            });
        }
        
        violations
    }
    
    fn check_power_trace_width(
        &self,
        pcb: &PcbDesign,
        rule: &CustomRule,
        net_pattern: &str,
        expected_current: f64,
        temp_rise: f64,
        settings: &RuleSettings,
    ) -> Vec<RuleViolation> {
        use crate::compliance::ipc2221::Ipc2221Calculator;
        
        let mut violations = Vec::new();
        let pattern_upper = net_pattern.to_uppercase();
        let calculator = Ipc2221Calculator::with_copper(settings.copper_oz, settings.copper_oz * 0.5);
        
        for trace in &pcb.traces {
            let empty_string = String::new();
            let net_name = trace.net_name.as_ref().unwrap_or(&empty_string);
            if !net_name.to_uppercase().contains(&pattern_upper) {
                continue;
            }
            
            let is_external = trace.layer == "F.Cu" || trace.layer == "B.Cu";
            let copper_thickness = if is_external {
                settings.copper_oz * 0.035
            } else {
                settings.copper_oz * 0.5 * 0.035
            };
            
            let max_current = calculator.calculate_max_current(
                trace.width,
                copper_thickness,
                temp_rise,
                is_external,
            );
            
            if max_current < expected_current {
                let required_width = calculator.calculate_required_width(
                    expected_current,
                    copper_thickness,
                    temp_rise,
                    is_external,
                );
                
                violations.push(RuleViolation {
                    rule_id: rule.id.clone(),
                    rule_name: rule.name.clone(),
                    severity: rule.severity.clone(),
                    category: rule.category.clone(),
                    message: format!(
                        "Power trace '{}' width {:.3}mm can only handle {:.2}A, but {:.2}A expected",
                        net_name, trace.width, max_current, expected_current
                    ),
                    location: Some(trace.start.clone()),
                    affected_items: vec![trace.uuid.clone()],
                    suggestion: Some(format!(
                        "Increase trace width to at least {:.3}mm for {}A at {}°C rise",
                        required_width, expected_current, temp_rise
                    )),
                });
            }
        }
        
        violations
    }
    
    fn check_differential_pair(
        &self,
        pcb: &PcbDesign,
        rule: &CustomRule,
        net_pattern_p: &str,
        net_pattern_n: &str,
        max_length_diff: f64,
    ) -> Vec<RuleViolation> {
        let mut violations = Vec::new();
        let pattern_p_upper = net_pattern_p.to_uppercase();
        let pattern_n_upper = net_pattern_n.to_uppercase();
        
        // Calculate total length for P net
        let p_length: f64 = pcb.traces
            .iter()
            .filter(|t| {
                t.net_name.as_ref()
                    .map(|n| n.to_uppercase().contains(&pattern_p_upper))
                    .unwrap_or(false)
            })
            .map(|t| t.length())
            .sum();
        
        // Calculate total length for N net
        let n_length: f64 = pcb.traces
            .iter()
            .filter(|t| {
                t.net_name.as_ref()
                    .map(|n| n.to_uppercase().contains(&pattern_n_upper))
                    .unwrap_or(false)
            })
            .map(|t| t.length())
            .sum();
        
        let length_diff = (p_length - n_length).abs();
        
        if length_diff > max_length_diff {
            violations.push(RuleViolation {
                rule_id: rule.id.clone(),
                rule_name: rule.name.clone(),
                severity: rule.severity.clone(),
                category: rule.category.clone(),
                message: format!(
                    "Differential pair length mismatch: {} ({:.2}mm) vs {} ({:.2}mm), diff={:.2}mm",
                    net_pattern_p, p_length, net_pattern_n, n_length, length_diff
                ),
                location: None,
                affected_items: Vec::new(),
                suggestion: Some(format!(
                    "Match trace lengths to within {:.2}mm using serpentine routing",
                    max_length_diff
                )),
            });
        }
        
        violations
    }
}

impl Default for CustomRulesEngine {
    fn default() -> Self {
        Self::new()
    }
}

/// Generate a sample rules.json template
pub fn generate_sample_rules() -> RuleSet {
    RuleSet {
        name: "Company Standard PCB Rules".to_string(),
        version: "1.0.0".to_string(),
        description: Some("Standard design rules for production PCBs".to_string()),
        author: Some("Engineering Team".to_string()),
        settings: RuleSettings {
            copper_oz: 1.0,
            temp_rise_c: 10.0,
            unit: "mm".to_string(),
        },
        rules: vec![
            CustomRule {
                id: "MIN_TRACE_WIDTH".to_string(),
                name: "Minimum Trace Width".to_string(),
                description: "All traces must be at least 0.15mm wide for manufacturability".to_string(),
                category: RuleCategory::Manufacturing,
                severity: RuleSeverity::Error,
                check: RuleCheck::MinTraceWidth {
                    min_width_mm: 0.15,
                    layer_filter: None,
                    net_filter: None,
                },
                enabled: true,
            },
            CustomRule {
                id: "MIN_VIA_SIZE".to_string(),
                name: "Minimum Via Size".to_string(),
                description: "Vias must meet minimum size requirements".to_string(),
                category: RuleCategory::Manufacturing,
                severity: RuleSeverity::Error,
                check: RuleCheck::MinViaSize {
                    min_size_mm: 0.6,
                    min_drill_mm: 0.3,
                },
                enabled: true,
            },
            CustomRule {
                id: "MOUNTING_HOLES_GND".to_string(),
                name: "Mounting Holes Grounded".to_string(),
                description: "All mounting holes must be connected to ground".to_string(),
                category: RuleCategory::Safety,
                severity: RuleSeverity::Warning,
                check: RuleCheck::MountingHolesGrounded {
                    min_connections: 1,
                },
                enabled: true,
            },
            CustomRule {
                id: "FIDUCIALS".to_string(),
                name: "Assembly Fiducials".to_string(),
                description: "Board must have at least 3 fiducials for assembly".to_string(),
                category: RuleCategory::Manufacturing,
                severity: RuleSeverity::Warning,
                check: RuleCheck::RequireFiducials {
                    min_count: 3,
                },
                enabled: true,
            },
            CustomRule {
                id: "USB_REF_PLANE".to_string(),
                name: "USB Reference Plane".to_string(),
                description: "USB signals require a continuous reference plane".to_string(),
                category: RuleCategory::Signal,
                severity: RuleSeverity::Error,
                check: RuleCheck::RequireReferencePlane {
                    net_pattern: "USB".to_string(),
                },
                enabled: true,
            },
            CustomRule {
                id: "POWER_5V_WIDTH".to_string(),
                name: "5V Power Trace Width".to_string(),
                description: "5V power traces must handle expected current".to_string(),
                category: RuleCategory::Power,
                severity: RuleSeverity::Error,
                check: RuleCheck::PowerTraceWidth {
                    net_pattern: "5V".to_string(),
                    expected_current_a: 2.0,
                    temp_rise_c: 10.0,
                },
                enabled: true,
            },
            CustomRule {
                id: "USB_DIFF_PAIR".to_string(),
                name: "USB Differential Pair Matching".to_string(),
                description: "USB D+/D- must be length matched".to_string(),
                category: RuleCategory::Signal,
                severity: RuleSeverity::Warning,
                check: RuleCheck::DifferentialPairMatch {
                    net_pattern_p: "USB_D+".to_string(),
                    net_pattern_n: "USB_D-".to_string(),
                    max_length_diff_mm: 0.5,
                    max_spacing_diff_mm: 0.1,
                },
                enabled: true,
            },
        ],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sample_rules_serialization() {
        let rules = generate_sample_rules();
        let json = serde_json::to_string_pretty(&rules).unwrap();
        
        // Should be valid JSON
        let parsed: RuleSet = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.name, "Company Standard PCB Rules");
        assert_eq!(parsed.rules.len(), 7);
    }

    #[test]
    fn test_rule_check_serialization() {
        let check = RuleCheck::MinTraceWidth {
            min_width_mm: 0.15,
            layer_filter: Some("F.Cu".to_string()),
            net_filter: None,
        };
        
        let json = serde_json::to_string(&check).unwrap();
        assert!(json.contains("MinTraceWidth"));
        assert!(json.contains("0.15"));
    }
}
