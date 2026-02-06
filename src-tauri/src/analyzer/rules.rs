use crate::parser::schema::*;
use crate::analyzer::capacitor_classifier::{CapacitorClassification, CapacitorFunction};
use crate::analyzer::decoupling_groups::DecouplingGroup;
use crate::analyzer::drs::{DRSAnalyzer, ICRiskScore};
use crate::compliance::power_net_registry::PowerNetRegistry;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum Severity {
    Error,
    Warning,
    Info,
    Suggestion,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RiskScore {
    pub value: f64,  // 0-100 risk index
    pub inductance_nh: Option<f64>,
    pub limit_nh: Option<f64>,
    pub metric: Option<String>,
    pub details: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Issue {
    pub id: String,
    pub rule_id: String,
    pub severity: Severity,
    pub message: String,
    pub component: Option<String>,
    pub location: Option<Position>,
    pub suggestion: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub risk_score: Option<RiskScore>,
}

/// Enhanced context for rule checking
pub struct RuleContext {
    pub capacitor_classifications: Vec<CapacitorClassification>,
    pub decoupling_groups: Vec<DecouplingGroup>,
    pub power_registry: PowerNetRegistry,
    pub pcb: Option<crate::parser::pcb_schema::PcbDesign>,  // Optional PCB for inductance analysis
}

pub trait Rule: Send + Sync {
    fn id(&self) -> &str;
    fn name(&self) -> &str;
    fn severity(&self) -> Severity;
    fn check(&self, schematic: &Schematic) -> Vec<Issue>;
}

pub struct RulesEngine {
    rules: Vec<Arc<dyn Rule>>,
}

impl RulesEngine {
    pub fn new() -> Self {
        Self {
            rules: Vec::new(),
        }
    }

    pub fn with_default_rules() -> Self {
        let mut engine = Self::new();
        engine.add_rule(Arc::new(DecouplingCapacitorRule));
        engine.add_rule(Arc::new(I2CPullResistorRule));
        engine.add_rule(Arc::new(CrystalLoadCapacitorRule));
        engine.add_rule(Arc::new(PowerPinRule));
        engine.add_rule(Arc::new(ESDProtectionRule));
        engine.add_rule(Arc::new(BulkCapacitorRule));
        engine
    }

    pub fn add_rule(&mut self, rule: Arc<dyn Rule>) {
        self.rules.push(rule);
    }

    pub fn analyze(&self, schematic: &Schematic) -> Vec<Issue> {
        let mut issues = Vec::new();
        for rule in &self.rules {
            issues.extend(rule.check(schematic));
        }
        issues
    }
    
    /// Enhanced analyze with capacitor classifications and decoupling groups
    pub fn analyze_enhanced(&self, schematic: &Schematic, context: Option<&RuleContext>) -> Vec<Issue> {
        let mut issues = Vec::new();
        
        // Use enhanced rules if context is available
        if let Some(ctx) = context {
            // Enhanced DecouplingCapacitorRule
            issues.extend(DecouplingCapacitorRule::check_enhanced(schematic, ctx));
            // Enhanced BulkCapacitorRule
            issues.extend(BulkCapacitorRule::check_enhanced(schematic, ctx));
            // Enhanced CrystalLoadCapacitorRule
            issues.extend(CrystalLoadCapacitorRule::check_enhanced(schematic, ctx));
            
            // Run other rules normally
            for rule in &self.rules {
                if rule.id() != "decoupling_capacitor" && 
                   rule.id() != "bulk_capacitor" && 
                   rule.id() != "crystal_load_capacitor" {
                    issues.extend(rule.check(schematic));
                }
            }
        } else {
            // Fall back to standard rules
            for rule in &self.rules {
                issues.extend(rule.check(schematic));
            }
        }
        
        issues
    }
}

impl Default for RulesEngine {
    fn default() -> Self {
        Self::with_default_rules()
    }
}

// Helper functions

fn distance(p1: &Position, p2: &Position) -> f64 {
    let dx = p1.x - p2.x;
    let dy = p1.y - p2.y;
    (dx * dx + dy * dy).sqrt()
}

fn is_nearby(component: &Component, position: &Position, radius_mm: f64) -> bool {
    distance(&component.position, position) <= radius_mm
}

fn parse_value(value: &str) -> Option<(f64, String)> {
    let value = value.trim();
    
    // Try to parse patterns like "10k", "100nF", "2.2uF", "33pF"
    let value_lower = value.to_lowercase();
    
    // Extract number and unit
    let mut num_str = String::new();
    let mut unit = String::new();
    let mut found_digit = false;
    
    for ch in value_lower.chars() {
        if ch.is_ascii_digit() || ch == '.' || ch == '-' {
            num_str.push(ch);
            found_digit = true;
        } else if found_digit {
            unit.push(ch);
        }
    }
    
    if num_str.is_empty() {
        return None;
    }
    
    let num = num_str.parse::<f64>().ok()?;
    
    // Normalize unit
    let normalized_unit = match unit.as_str() {
        "pf" | "p" => "pF",
        "nf" | "n" => "nF",
        "uf" | "u" | "µf" | "µ" => "uF",
        "mf" | "m" => "mF",
        "f" => "F",
        "k" | "kohm" | "kω" => "kΩ",
        "m" | "mohm" | "mω" => "mΩ",
        "ohm" | "ω" | "r" => "Ω",
        _ => &unit,
    };
    
    Some((num, normalized_unit.to_string()))
}

fn value_matches_pattern(value: &str, patterns: &[&str]) -> bool {
    let value_lower = value.to_lowercase();
    patterns.iter().any(|pattern| value_lower.contains(&pattern.to_lowercase()))
}

fn get_all_components(schematic: &Schematic) -> Vec<&Component> {
    let mut all = Vec::new();
    all.extend(&schematic.components);
    all.extend(&schematic.power_symbols);
    all
}

// Rule implementations

pub struct DecouplingCapacitorRule;

impl Rule for DecouplingCapacitorRule {
    fn id(&self) -> &str {
        "decoupling_capacitor"
    }

    fn name(&self) -> &str {
        "Decoupling Capacitor Check"
    }

    fn severity(&self) -> Severity {
        Severity::Warning
    }

    fn check(&self, schematic: &Schematic) -> Vec<Issue> {
        let mut issues = Vec::new();
        let all_components = get_all_components(schematic);
        
        // Find ICs (reference starts with 'U')
        let ics: Vec<&Component> = all_components
            .iter()
            .filter(|c| c.reference.starts_with('U') || c.reference.starts_with('u'))
            .copied()
            .collect();

        for ic in ics {
            // Look for 100nF capacitor within 20mm
            let mut found_cap = false;
            for component in &all_components {
                if component.reference.starts_with('C') || component.reference.starts_with('c') {
                    if let Some((value, unit)) = parse_value(&component.value) {
                        // Check for ~100nF (80nF to 120nF)
                        if unit == "nF" && value >= 80.0 && value <= 120.0 {
                            if is_nearby(component, &ic.position, 20.0) {
                                found_cap = true;
                                break;
                            }
                        }
                    }
                }
            }

            if !found_cap {
                issues.push(Issue {
                    id: uuid::Uuid::new_v4().to_string(),
                    rule_id: self.id().to_string(),
                    severity: self.severity(),
                    message: format!(
                        "IC {} ({}) may need a decoupling capacitor (100nF) within 20mm",
                        ic.reference, ic.value
                    ),
                    component: Some(ic.reference.clone()),
                    location: Some(ic.position.clone()),
                    suggestion: Some("Add a 100nF ceramic capacitor close to the IC power pins".to_string()),
                    risk_score: None,
                });
            }
        }

        issues
    }
}

impl DecouplingCapacitorRule {
    /// Enhanced check using capacitor classifications and decoupling groups
    pub fn check_enhanced(schematic: &Schematic, context: &RuleContext) -> Vec<Issue> {
        let mut issues = Vec::new();
        
        // Check each decoupling group
        for group in &context.decoupling_groups {
            // Check for missing High-Frequency Bypass
            if !group.has_hf_bypass {
                let message = if group.has_bulk {
                    format!(
                        "Missing High-Frequency Bypass: IC {} ({}) has {}µF bulk capacitor but no 100nF local decoupling within 5mm of power pins",
                        group.ic_ref,
                        group.ic_value,
                        group.capacitors.iter()
                            .find(|c| c.is_bulk)
                            .map(|c| c.value.clone())
                            .unwrap_or_else(|| "?".to_string())
                    )
                } else {
                    format!(
                        "Missing Decoupling: IC {} ({}) has no decoupling capacitor within 20mm",
                        group.ic_ref, group.ic_value
                    )
                };
                
                // Find IC component for location
                let ic_location = schematic.components
                    .iter()
                    .find(|c| c.reference == group.ic_ref)
                    .map(|c| c.position.clone());
                
                // Calculate risk score if PCB is available
                let risk_score = if let Some(ref pcb) = context.pcb {
                    DecouplingCapacitorRule::calculate_risk_score_for_ic(&group.ic_ref, &group.ic_value, pcb)
                } else {
                    None
                };
                
                issues.push(Issue {
                    id: uuid::Uuid::new_v4().to_string(),
                    rule_id: "decoupling_capacitor".to_string(),
                    severity: Severity::Warning,
                    message,
                    component: Some(group.ic_ref.clone()),
                    location: ic_location,
                    suggestion: Some("Add a 100nF ceramic capacitor (0402 or 0603) within 5mm of the IC power pins".to_string()),
                    risk_score,
                });
            } else {
                // Check if HF bypass is too far (>5mm)
                if let Some(distance) = group.hf_bypass_distance_mm {
                    if distance > 5.0 {
                        // Calculate risk score if PCB is available
                        let risk_score = if let Some(ref pcb) = context.pcb {
                            DecouplingCapacitorRule::calculate_risk_score_for_ic(&group.ic_ref, &group.ic_value, pcb)
                        } else {
                            None
                        };
                        
                        issues.push(Issue {
                            id: uuid::Uuid::new_v4().to_string(),
                            rule_id: "decoupling_capacitor".to_string(),
                            severity: Severity::Warning,
                            message: format!(
                                "Decoupling capacitor too far: IC {} has HF bypass at {:.1}mm (recommended <5mm)",
                                group.ic_ref, distance
                            ),
                            component: Some(group.ic_ref.clone()),
                            location: schematic.components
                                .iter()
                                .find(|c| c.reference == group.ic_ref)
                                .map(|c| c.position.clone()),
                            suggestion: Some("Move decoupling capacitor closer to IC power pins (<5mm)".to_string()),
                            risk_score,
                        });
                    }
                }
            }
        }
        
        issues
    }
}

pub struct I2CPullResistorRule;

impl Rule for I2CPullResistorRule {
    fn id(&self) -> &str {
        "i2c_pull_resistors"
    }

    fn name(&self) -> &str {
        "I2C Pull-up Resistor Check"
    }

    fn severity(&self) -> Severity {
        Severity::Warning
    }

    fn check(&self, schematic: &Schematic) -> Vec<Issue> {
        let mut issues = Vec::new();
        
        // Detect I2C buses (SDA, SCL labels)
        let mut has_sda = false;
        let mut has_scl = false;
        
        for label in &schematic.labels {
            let label_upper = label.text.to_uppercase();
            if label_upper == "SDA" || label_upper.contains("SDA") {
                has_sda = true;
            }
            if label_upper == "SCL" || label_upper.contains("SCL") {
                has_scl = true;
            }
        }

        if has_sda && has_scl {
            // Check for pull-up resistors (2.2k to 10k)
            let all_components = get_all_components(schematic);
            let mut found_pullup = false;

            for component in &all_components {
                if component.reference.starts_with('R') || component.reference.starts_with('r') {
                    if let Some((value, unit)) = parse_value(&component.value) {
                        // Check for 2.2k to 10k
                        if (unit == "kΩ" || unit == "k" || unit == "Ω") {
                            let value_ohm = if unit == "kΩ" || unit == "k" {
                                value * 1000.0
                            } else {
                                value
                            };
                            
                            if value_ohm >= 2200.0 && value_ohm <= 10000.0 {
                                found_pullup = true;
                                break;
                            }
                        }
                    }
                }
            }

            if !found_pullup {
                issues.push(Issue {
                    id: uuid::Uuid::new_v4().to_string(),
                    rule_id: self.id().to_string(),
                    severity: self.severity(),
                    message: "I2C bus detected (SDA/SCL) but no pull-up resistors found (2.2k-10k)".to_string(),
                    component: None,
                    location: None,
                    suggestion: Some("Add pull-up resistors (typically 4.7kΩ) to SDA and SCL lines".to_string()),
                    risk_score: None,
                });
            }
        }

        issues
    }
}

pub struct CrystalLoadCapacitorRule;

impl Rule for CrystalLoadCapacitorRule {
    fn id(&self) -> &str {
        "crystal_load_capacitors"
    }

    fn name(&self) -> &str {
        "Crystal Load Capacitor Check"
    }

    fn severity(&self) -> Severity {
        Severity::Warning
    }

    fn check(&self, schematic: &Schematic) -> Vec<Issue> {
        let mut issues = Vec::new();
        let all_components = get_all_components(schematic);
        
        // Find crystals (reference 'Y' or value contains "MHz")
        let crystals: Vec<&Component> = all_components
            .iter()
            .filter(|c| {
                c.reference.starts_with('Y') || c.reference.starts_with('y') ||
                value_matches_pattern(&c.value, &["mhz", "khz", "crystal", "oscillator"])
            })
            .copied()
            .collect();

        for crystal in crystals {
            // Check for two capacitors nearby (10pF-33pF)
            let mut capacitor_count = 0;
            let mut capacitors_found = Vec::new();

            for component in &all_components {
                if component.reference.starts_with('C') || component.reference.starts_with('c') {
                    if let Some((value, unit)) = parse_value(&component.value) {
                        // Check for 10pF to 33pF
                        if unit == "pF" && value >= 10.0 && value <= 33.0 {
                            if is_nearby(component, &crystal.position, 30.0) {
                                capacitor_count += 1;
                                capacitors_found.push(component.reference.clone());
                            }
                        }
                    }
                }
            }

            if capacitor_count < 2 {
                issues.push(Issue {
                    id: uuid::Uuid::new_v4().to_string(),
                    rule_id: self.id().to_string(),
                    severity: self.severity(),
                    message: format!(
                        "Crystal {} ({}) should have two load capacitors (10-33pF) nearby. Found: {}",
                        crystal.reference,
                        crystal.value,
                        capacitor_count
                    ),
                    component: Some(crystal.reference.clone()),
                    location: Some(crystal.position.clone()),
                    suggestion: Some(format!(
                        "Add two load capacitors (typically 22pF) near the crystal. Found: {:?}",
                        capacitors_found
                    )),
                    risk_score: None,
                });
            }
        }

        issues
    }
    
}

impl BulkCapacitorRule {
    /// Calculate risk score for an IC using DRS (same as DecouplingCapacitorRule)
    fn calculate_risk_score_for_ic(
        ic_ref: &str,
        ic_value: &str,
        pcb: &crate::parser::pcb_schema::PcbDesign,
    ) -> Option<RiskScore> {
        let drs_analyzer = DRSAnalyzer::new();
        let max_inductance_nh = drs_analyzer.get_max_inductance(ic_value);
        
        Some(RiskScore {
            value: 50.0,
            inductance_nh: None,
            limit_nh: max_inductance_nh,
            metric: Some("loop_inductance".to_string()),
            details: max_inductance_nh.map(|limit| {
                format!("Recommended max inductance: {:.1} nH for {}", limit, ic_value)
            }),
        })
    }
}

impl CrystalLoadCapacitorRule {
    /// Enhanced check using capacitor classifications (only timing caps)
    pub fn check_enhanced(schematic: &Schematic, context: &RuleContext) -> Vec<Issue> {
        let mut issues = Vec::new();
        let all_components = get_all_components(schematic);
        
        // Find crystals (reference 'Y' or value contains "MHz")
        let crystals: Vec<&Component> = all_components
            .iter()
            .filter(|c| {
                c.reference.starts_with('Y') || c.reference.starts_with('y') ||
                value_matches_pattern(&c.value, &["mhz", "khz", "crystal", "oscillator"])
            })
            .copied()
            .collect();

        for crystal in crystals {
            // Find timing capacitors (classified as Timing, 1pF-47pF, near crystal)
            let timing_caps: Vec<&CapacitorClassification> = context.capacitor_classifications
                .iter()
                .filter(|c| {
                    c.function == CapacitorFunction::Timing &&
                    c.confidence > 0.5
                })
                .collect();
            
            // Check if we have a pair (two timing caps)
            let nearby_timing_caps: Vec<&CapacitorClassification> = timing_caps
                .iter()
                .filter(|c| {
                    // Find component and check distance
                    if let Some(cap_comp) = schematic.components.iter().find(|comp| comp.reference == c.component_ref) {
                        let distance = distance(&cap_comp.position, &crystal.position);
                        distance < 30.0  // Within 30mm
                    } else {
                        false
                    }
                })
                .copied()
                .collect();
            
            if nearby_timing_caps.len() < 2 {
                let message = if nearby_timing_caps.is_empty() {
                    format!(
                        "Crystal {} ({}) missing load capacitors (need 2 timing caps, 10pF-33pF)",
                        crystal.reference, crystal.value
                    )
                } else {
                    format!(
                        "Crystal {} ({}) has incomplete timing capacitor pair (found 1, need 2)",
                        crystal.reference, crystal.value
                    )
                };
                
                issues.push(Issue {
                    id: uuid::Uuid::new_v4().to_string(),
                    rule_id: "crystal_load_capacitors".to_string(),
                    severity: Severity::Warning,
                    message,
                    component: Some(crystal.reference.clone()),
                    location: Some(crystal.position.clone()),
                    suggestion: Some("Add two load capacitors (10pF-33pF each) near the crystal, one on each pin to GND".to_string()),
                    risk_score: None,
                });
            }
        }
        
        issues
    }
}

pub struct PowerPinRule;

impl Rule for PowerPinRule {
    fn id(&self) -> &str {
        "power_pins"
    }

    fn name(&self) -> &str {
        "Power Pin Check"
    }

    fn severity(&self) -> Severity {
        Severity::Error
    }

    fn check(&self, schematic: &Schematic) -> Vec<Issue> {
        let mut issues = Vec::new();
        
        // Check for GND symbol
        let mut has_gnd = false;
        for component in &schematic.power_symbols {
            let value_upper = component.value.to_uppercase();
            if value_upper == "GND" || value_upper == "GROUND" || value_upper == "VSS" {
                has_gnd = true;
                break;
            }
        }

        // Also check labels
        if !has_gnd {
            for label in &schematic.labels {
                let label_upper = label.text.to_uppercase();
                if label_upper == "GND" || label_upper == "GROUND" || label_upper == "VSS" {
                    has_gnd = true;
                    break;
                }
            }
        }

        if !has_gnd {
            issues.push(Issue {
                id: uuid::Uuid::new_v4().to_string(),
                rule_id: self.id().to_string(),
                severity: self.severity(),
                message: "No GND symbol or label found in schematic".to_string(),
                component: None,
                location: None,
                suggestion: Some("Add a GND power symbol to the schematic".to_string()),
                risk_score: None,
            });
        }

        // Flag ICs without visible power connections
        // This is simplified - real check would analyze net connectivity
        let all_components = get_all_components(schematic);
        let ics: Vec<&Component> = all_components
            .iter()
            .filter(|c| c.reference.starts_with('U') || c.reference.starts_with('u'))
            .copied()
            .collect();

        // Check if ICs have power-related properties or are connected to power nets
        // This is a simplified check - real implementation would analyze net connectivity
        for ic in ics {
            let has_power_prop = ic.properties.values().any(|v| {
                let v_upper = v.to_uppercase();
                v_upper.contains("VDD") || v_upper.contains("VCC") || 
                v_upper.contains("VSS") || v_upper.contains("GND") ||
                v_upper.contains("POWER")
            });

            if !has_power_prop && ic.pins.is_empty() {
                // If no pins defined, we can't verify power connections
                issues.push(Issue {
                    id: uuid::Uuid::new_v4().to_string(),
                    rule_id: self.id().to_string(),
                    severity: Severity::Warning,
                    message: format!(
                        "IC {} ({}) has no visible power connections. Verify VDD/VCC and GND are connected",
                        ic.reference, ic.value
                    ),
                    component: Some(ic.reference.clone()),
                    location: Some(ic.position.clone()),
                    suggestion: Some("Ensure all power and ground pins are properly connected".to_string()),
                    risk_score: None,
                });
            }
        }

        issues
    }
    
}

impl DecouplingCapacitorRule {
    /// Calculate risk score for an IC using DRS
    fn calculate_risk_score_for_ic(
        ic_ref: &str,
        ic_value: &str,
        pcb: &crate::parser::pcb_schema::PcbDesign,
    ) -> Option<RiskScore> {
        // Get max inductance limit
        let drs_analyzer = DRSAnalyzer::new();
        let max_inductance_nh = drs_analyzer.get_max_inductance(ic_value);
        
        // For now, return a placeholder risk score
        // Full DRS integration would require running full DRS analysis
        // This is a simplified version that can be enhanced later
        Some(RiskScore {
            value: 50.0,  // Placeholder - would come from DRS risk_index
            inductance_nh: None,  // Would be calculated from DRS CapacitorAnalysis
            limit_nh: max_inductance_nh,
            metric: Some("loop_inductance".to_string()),
            details: max_inductance_nh.map(|limit| {
                format!("Recommended max inductance: {:.1} nH for {}", limit, ic_value)
            }),
        })
    }
}

impl BulkCapacitorRule {
    /// Enhanced check using decoupling groups
    pub fn check_enhanced(schematic: &Schematic, context: &RuleContext) -> Vec<Issue> {
        let mut issues = Vec::new();
        
        // Check each decoupling group for missing bulk caps when required
        for group in &context.decoupling_groups {
            // Check if IC requires bulk cap (high-speed ICs, processors, etc.)
            let requires_bulk = group.ic_value.to_uppercase().contains("STM32") ||
                               group.ic_value.to_uppercase().contains("ESP32") ||
                               group.ic_value.to_uppercase().contains("FPGA") ||
                               group.ic_value.to_uppercase().contains("CPU");
            
            if requires_bulk && !group.has_bulk {
                let ic_location = schematic.components
                    .iter()
                    .find(|c| c.reference == group.ic_ref)
                    .map(|c| c.position.clone());
                
                // Calculate risk score if PCB is available
                let risk_score = if let Some(ref pcb) = context.pcb {
                    BulkCapacitorRule::calculate_risk_score_for_ic(&group.ic_ref, &group.ic_value, pcb)
                } else {
                    None
                };
                
                issues.push(Issue {
                    id: uuid::Uuid::new_v4().to_string(),
                    rule_id: "bulk_capacitor".to_string(),
                    severity: Severity::Warning,
                    message: format!(
                        "Missing Bulk Capacitor: IC {} ({}) requires a bulk capacitor (>4.7µF) for energy storage",
                        group.ic_ref, group.ic_value
                    ),
                    component: Some(group.ic_ref.clone()),
                    location: ic_location,
                    suggestion: Some("Add a 10µF to 47µF bulk capacitor (0805, 1206, or tantalum) on the power rail".to_string()),
                    risk_score,
                });
            }
        }
        
        issues
    }
}

pub struct ESDProtectionRule;

impl Rule for ESDProtectionRule {
    fn id(&self) -> &str {
        "esd_protection"
    }

    fn name(&self) -> &str {
        "ESD Protection Check"
    }

    fn severity(&self) -> Severity {
        Severity::Info
    }

    fn check(&self, schematic: &Schematic) -> Vec<Issue> {
        let mut issues = Vec::new();
        
        // Detect external interfaces (USB, Ethernet)
        let mut has_usb = false;
        let mut has_ethernet = false;
        
        let all_components = get_all_components(schematic);
        for component in &all_components {
            let ref_upper = component.reference.to_uppercase();
            let value_upper = component.value.to_uppercase();
            let lib_upper = component.lib_id.to_uppercase();
            
            if value_matches_pattern(&value_upper, &["usb", "usb-a", "usb-b", "usb-c", "micro-usb"]) ||
               value_matches_pattern(&lib_upper, &["usb"]) ||
               ref_upper.contains("USB") {
                has_usb = true;
            }
            
            if value_matches_pattern(&value_upper, &["ethernet", "rj45", "lan"]) ||
               value_matches_pattern(&lib_upper, &["ethernet", "rj45"]) ||
               ref_upper.contains("ETH") || ref_upper.contains("RJ45") {
                has_ethernet = true;
            }
        }

        // Check labels too
        for label in &schematic.labels {
            let label_upper = label.text.to_uppercase();
            if label_upper.contains("USB") || label_upper.contains("D+") || label_upper.contains("D-") {
                has_usb = true;
            }
            if label_upper.contains("ETH") || label_upper.contains("RJ45") {
                has_ethernet = true;
            }
        }

        // Check for TVS diodes
        let mut has_tvs = false;
        for component in &all_components {
            let value_upper = component.value.to_uppercase();
            let lib_upper = component.lib_id.to_uppercase();
            
            if value_matches_pattern(&value_upper, &["tvs", "esd", "transient"]) ||
               value_matches_pattern(&lib_upper, &["tvs", "esd"]) ||
               component.reference.starts_with('D') && value_upper.contains("ESD") {
                has_tvs = true;
                break;
            }
        }

        if (has_usb || has_ethernet) && !has_tvs {
            let interface = if has_usb && has_ethernet {
                "USB and Ethernet"
            } else if has_usb {
                "USB"
            } else {
                "Ethernet"
            };

            issues.push(Issue {
                id: uuid::Uuid::new_v4().to_string(),
                rule_id: self.id().to_string(),
                severity: self.severity(),
                message: format!("{} interface detected but no ESD protection (TVS diodes) found", interface),
                component: None,
                location: None,
                suggestion: Some("Consider adding TVS diodes for ESD protection on external interface lines".to_string()),
                risk_score: None,
            });
        }

        issues
    }
}

pub struct BulkCapacitorRule;

impl Rule for BulkCapacitorRule {
    fn id(&self) -> &str {
        "bulk_capacitor"
    }

    fn name(&self) -> &str {
        "Bulk Capacitor Check"
    }

    fn severity(&self) -> Severity {
        Severity::Warning
    }

    fn check(&self, schematic: &Schematic) -> Vec<Issue> {
        let mut issues = Vec::new();
        let all_components = get_all_components(schematic);
        
        // Find voltage regulators
        let regulators: Vec<&Component> = all_components
            .iter()
            .filter(|c| {
                let ref_upper = c.reference.to_uppercase();
                let value_upper = c.value.to_uppercase();
                let lib_upper = c.lib_id.to_uppercase();
                
                ref_upper.starts_with('U') && (
                    value_matches_pattern(&value_upper, &["lm", "ldo", "regulator", "7805", "1117", "ams1117"]) ||
                    value_matches_pattern(&lib_upper, &["regulator", "ldo", "linear"])
                )
            })
            .copied()
            .collect();

        for regulator in regulators {
            // Check for 10µF-100µF capacitors
            let mut found_bulk_cap = false;
            
            for component in &all_components {
                if component.reference.starts_with('C') || component.reference.starts_with('c') {
                    if let Some((value, unit)) = parse_value(&component.value) {
                        // Check for 10uF to 100uF
                        let value_uF = if unit == "uF" || unit == "µF" {
                            value
                        } else if unit == "nF" {
                            value / 1000.0
                        } else if unit == "mF" {
                            value * 1000.0
                        } else {
                            continue;
                        };
                        
                        if value_uF >= 10.0 && value_uF <= 100.0 {
                            if is_nearby(component, &regulator.position, 30.0) {
                                found_bulk_cap = true;
                                break;
                            }
                        }
                    }
                }
            }

            if !found_bulk_cap {
                issues.push(Issue {
                    id: uuid::Uuid::new_v4().to_string(),
                    rule_id: self.id().to_string(),
                    severity: self.severity(),
                    message: format!(
                        "Voltage regulator {} ({}) should have a bulk capacitor (10-100µF) nearby",
                        regulator.reference, regulator.value
                    ),
                    component: Some(regulator.reference.clone()),
                    location: Some(regulator.position.clone()),
                    suggestion: Some("Add a bulk capacitor (typically 22µF or 47µF) near the regulator output".to_string()),
                    risk_score: None,
                });
            }
        }

        issues
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn create_test_schematic() -> Schematic {
        Schematic {
            uuid: "test".to_string(),
            filename: "test.kicad_sch".to_string(),
            version: None,
            components: Vec::new(),
            wires: Vec::new(),
            labels: Vec::new(),
            nets: Vec::new(),
            power_symbols: Vec::new(),
        }
    }

    #[test]
    fn test_distance() {
        let p1 = Position { x: 0.0, y: 0.0 };
        let p2 = Position { x: 3.0, y: 4.0 };
        assert_eq!(distance(&p1, &p2), 5.0);
    }

    #[test]
    fn test_is_nearby() {
        let component = Component {
            uuid: "test".to_string(),
            reference: "C1".to_string(),
            value: "100nF".to_string(),
            lib_id: "Device:C".to_string(),
            footprint: None,
            position: Position { x: 10.0, y: 10.0 },
            rotation: 0.0,
            properties: HashMap::new(),
            pins: Vec::new(),
        };
        let position = Position { x: 15.0, y: 10.0 };
        assert!(is_nearby(&component, &position, 10.0));
        assert!(!is_nearby(&component, &position, 4.0));
    }

    #[test]
    fn test_parse_value() {
        assert_eq!(parse_value("100nF"), Some((100.0, "nF".to_string())));
        assert_eq!(parse_value("10k"), Some((10.0, "kΩ".to_string())));
        assert_eq!(parse_value("2.2uF"), Some((2.2, "uF".to_string())));
        assert_eq!(parse_value("33pF"), Some((33.0, "pF".to_string())));
        assert_eq!(parse_value("47µF"), Some((47.0, "uF".to_string())));
    }

    #[test]
    fn test_decoupling_capacitor_rule() {
        let mut schematic = create_test_schematic();
        
        // Add an IC without decoupling capacitor
        schematic.components.push(Component {
            uuid: "ic1".to_string(),
            reference: "U1".to_string(),
            value: "STM32F4".to_string(),
            lib_id: "MCU:STM32".to_string(),
            footprint: None,
            position: Position { x: 100.0, y: 100.0 },
            rotation: 0.0,
            properties: HashMap::new(),
            pins: Vec::new(),
        });

        let rule = DecouplingCapacitorRule;
        let issues = rule.check(&schematic);
        assert!(!issues.is_empty());
        assert_eq!(issues[0].rule_id, "decoupling_capacitor");
    }

    #[test]
    fn test_i2c_pull_resistor_rule() {
        let mut schematic = create_test_schematic();
        
        // Add I2C labels
        schematic.labels.push(Label {
            uuid: "l1".to_string(),
            text: "SDA".to_string(),
            position: Position { x: 50.0, y: 50.0 },
            rotation: 0.0,
            label_type: LabelType::Global,
        });
        schematic.labels.push(Label {
            uuid: "l2".to_string(),
            text: "SCL".to_string(),
            position: Position { x: 60.0, y: 50.0 },
            rotation: 0.0,
            label_type: LabelType::Global,
        });

        let rule = I2CPullResistorRule;
        let issues = rule.check(&schematic);
        assert!(!issues.is_empty());
    }

    #[test]
    fn test_crystal_load_capacitor_rule() {
        let mut schematic = create_test_schematic();
        
        // Add a crystal
        schematic.components.push(Component {
            uuid: "y1".to_string(),
            reference: "Y1".to_string(),
            value: "8MHz".to_string(),
            lib_id: "Device:Crystal".to_string(),
            footprint: None,
            position: Position { x: 100.0, y: 100.0 },
            rotation: 0.0,
            properties: HashMap::new(),
            pins: Vec::new(),
        });

        let rule = CrystalLoadCapacitorRule;
        let issues = rule.check(&schematic);
        assert!(!issues.is_empty());
    }

    #[test]
    fn test_power_pin_rule() {
        let schematic = create_test_schematic();
        
        let rule = PowerPinRule;
        let issues = rule.check(&schematic);
        // Should flag missing GND
        assert!(!issues.is_empty());
    }

    #[test]
    fn test_esd_protection_rule() {
        let mut schematic = create_test_schematic();
        
        // Add USB component
        schematic.components.push(Component {
            uuid: "usb1".to_string(),
            reference: "J1".to_string(),
            value: "USB-A".to_string(),
            lib_id: "Connector:USB".to_string(),
            footprint: None,
            position: Position { x: 50.0, y: 50.0 },
            rotation: 0.0,
            properties: HashMap::new(),
            pins: Vec::new(),
        });

        let rule = ESDProtectionRule;
        let issues = rule.check(&schematic);
        assert!(!issues.is_empty());
    }

    #[test]
    fn test_bulk_capacitor_rule() {
        let mut schematic = create_test_schematic();
        
        // Add a regulator
        schematic.components.push(Component {
            uuid: "reg1".to_string(),
            reference: "U1".to_string(),
            value: "AMS1117-3.3".to_string(),
            lib_id: "Regulator:Linear".to_string(),
            footprint: None,
            position: Position { x: 100.0, y: 100.0 },
            rotation: 0.0,
            properties: HashMap::new(),
            pins: Vec::new(),
        });

        let rule = BulkCapacitorRule;
        let issues = rule.check(&schematic);
        assert!(!issues.is_empty());
    }

    #[test]
    fn test_rules_engine() {
        let engine = RulesEngine::with_default_rules();
        let schematic = create_test_schematic();
        let issues = engine.analyze(&schematic);
        
        // Should have at least one issue (missing GND)
        assert!(!issues.is_empty());
    }
}
