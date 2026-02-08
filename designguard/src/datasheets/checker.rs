//! Datasheet-Aware Design Checker
//!
//! Verifies that a schematic meets the requirements specified in component datasheets.

use crate::analyzer::rules::{Issue, Severity};
use crate::datasheets::matcher::DatasheetMatcher;
use crate::datasheets::schema::*;
use crate::parser::schema::{Component, Position, Schematic};

/// Checker that verifies schematics against datasheet requirements
pub struct DatasheetChecker {
    matcher: DatasheetMatcher,
}

impl DatasheetChecker {
    /// Create a new checker with built-in datasheets
    pub fn new() -> Self {
        Self {
            matcher: DatasheetMatcher::with_builtin_datasheets(),
        }
    }
    
    /// Create a checker with a custom matcher
    pub fn with_matcher(matcher: DatasheetMatcher) -> Self {
        Self { matcher }
    }
    
    /// Check a schematic against all applicable datasheet requirements
    pub fn check(&self, schematic: &Schematic) -> Vec<DatasheetIssue> {
        let mut issues = Vec::new();
        
        // Get all components (including power symbols)
        let all_components: Vec<&Component> = schematic
            .components
            .iter()
            .chain(schematic.power_symbols.iter())
            .collect();
        
        // Check each component that has a matching datasheet
        for component in &schematic.components {
            if let Some(datasheet) = self.matcher.match_component(component) {
                // Check decoupling requirements
                issues.extend(self.check_decoupling(
                    component,
                    datasheet,
                    &all_components,
                ));
                
                // Check external component requirements
                issues.extend(self.check_external_components(
                    component,
                    datasheet,
                    &all_components,
                ));
                
                // Check pin requirements
                issues.extend(self.check_pin_requirements(
                    component,
                    datasheet,
                    schematic,
                ));
            }
        }
        
        issues
    }
    
    /// Convert datasheet issues to standard issues
    pub fn check_as_issues(&self, schematic: &Schematic) -> Vec<Issue> {
        self.check(schematic)
            .into_iter()
            .map(|di| di.into())
            .collect()
    }
    
    /// Check decoupling capacitor requirements
    fn check_decoupling(
        &self,
        component: &Component,
        datasheet: &DatasheetRequirements,
        all_components: &[&Component],
    ) -> Vec<DatasheetIssue> {
        let mut issues = Vec::new();
        
        // Group requirements by power pin and role
        for req in &datasheet.decoupling_requirements {
            let found_cap = self.find_matching_capacitor(
                component,
                req,
                all_components,
            );
            
            if found_cap.is_none() {
                issues.push(DatasheetIssue {
                    severity: req.severity.clone().into(),
                    component_ref: component.reference.clone(),
                    component_value: component.value.clone(),
                    requirement_type: "Decoupling Capacitor".to_string(),
                    title: format!(
                        "{} ({}) - {} Decoupling",
                        component.reference,
                        component.value,
                        req.power_pin
                    ),
                    what: format!(
                        "Missing {} {} capacitor on {} pin",
                        req.capacitance.display(),
                        match req.capacitor_role {
                            CapacitorRole::Bypass => "bypass",
                            CapacitorRole::Bulk => "bulk",
                            CapacitorRole::Decoupling => "decoupling",
                            CapacitorRole::Filter => "filter",
                        },
                        req.power_pin
                    ),
                    why: req.reason.clone(),
                    how_to_fix: format!(
                        "Add a {} {} capacitor within {}mm of the {} pin",
                        req.capacitance.display(),
                        req.capacitor_type.as_ref().map(|t| format!("{:?}", t).to_lowercase()).unwrap_or_default(),
                        req.max_distance_mm,
                        req.power_pin
                    ),
                    datasheet_reference: datasheet.datasheet_url.clone(),
                    location: Some(component.position.clone()),
                });
            }
        }
        
        issues
    }
    
    /// Find a capacitor that matches the requirement
    fn find_matching_capacitor<'a>(
        &self,
        ic: &Component,
        req: &DecouplingRequirement,
        all_components: &'a [&'a Component],
    ) -> Option<&'a Component> {
        for component in all_components {
            // Check if it's a capacitor
            if !component.reference.starts_with('C') && !component.reference.starts_with('c') {
                continue;
            }
            
            // Parse capacitor value
            if let Some(cap_value) = parse_capacitor_value(&component.value) {
                // Check if value is within acceptable range
                let min_acceptable = req.capacitance.min * 0.8; // 20% tolerance
                let max_acceptable = req.capacitance.max.unwrap_or(req.capacitance.typical * 10.0);
                
                if cap_value >= min_acceptable && cap_value <= max_acceptable {
                    // Check distance
                    let distance = calculate_distance(&ic.position, &component.position);
                    if distance <= req.max_distance_mm {
                        return Some(*component);
                    }
                }
            }
        }
        
        None
    }
    
    /// Check external component requirements
    fn check_external_components(
        &self,
        component: &Component,
        datasheet: &DatasheetRequirements,
        all_components: &[&Component],
    ) -> Vec<DatasheetIssue> {
        let mut issues = Vec::new();
        
        for req in &datasheet.external_components {
            if !req.required {
                continue; // Skip optional components
            }
            
            let found = self.find_external_component(component, req, all_components);
            
            if !found {
                let component_desc = match &req.component_type {
                    ExternalComponentType::Crystal { frequency_hz, .. } => {
                        format!("{}MHz crystal", frequency_hz / 1_000_000.0)
                    }
                    ExternalComponentType::PullUpResistor => "pull-up resistor".to_string(),
                    ExternalComponentType::PullDownResistor => "pull-down resistor".to_string(),
                    ExternalComponentType::FilterCapacitor => "filter capacitor".to_string(),
                    ExternalComponentType::SeriesTerminationResistor => "series termination resistor".to_string(),
                    ExternalComponentType::ProtectionDiode => "protection diode".to_string(),
                    _ => "external component".to_string(),
                };
                
                issues.push(DatasheetIssue {
                    severity: Severity::Warning,
                    component_ref: component.reference.clone(),
                    component_value: component.value.clone(),
                    requirement_type: "External Component".to_string(),
                    title: format!(
                        "{} ({}) - Missing {}",
                        component.reference,
                        component.value,
                        component_desc
                    ),
                    what: format!(
                        "Missing required {} on pins: {}",
                        component_desc,
                        req.connected_pins.join(", ")
                    ),
                    why: req.reason.clone(),
                    how_to_fix: format!(
                        "Add {} connected to {} pins",
                        component_desc,
                        req.connected_pins.join(", ")
                    ),
                    datasheet_reference: datasheet.datasheet_url.clone(),
                    location: Some(component.position.clone()),
                });
            }
        }
        
        issues
    }
    
    /// Find an external component that matches the requirement
    fn find_external_component(
        &self,
        ic: &Component,
        req: &ExternalComponentRequirement,
        all_components: &[&Component],
    ) -> bool {
        let search_radius = 50.0; // mm - generous search radius
        
        for component in all_components {
            let distance = calculate_distance(&ic.position, &component.position);
            if distance > search_radius {
                continue;
            }
            
            match &req.component_type {
                ExternalComponentType::Crystal { .. } => {
                    if component.reference.starts_with('Y') || 
                       component.reference.starts_with('X') ||
                       component.value.to_lowercase().contains("mhz") ||
                       component.value.to_lowercase().contains("crystal") {
                        return true;
                    }
                }
                ExternalComponentType::PullUpResistor | ExternalComponentType::PullDownResistor => {
                    if component.reference.starts_with('R') || component.reference.starts_with('r') {
                        if let Some(value) = parse_resistor_value(&component.value) {
                            // Check if it's a reasonable pull-up/down value (1k-100k)
                            if value >= 1000.0 && value <= 100_000.0 {
                                return true;
                            }
                        }
                    }
                }
                ExternalComponentType::FilterCapacitor | ExternalComponentType::BypassCapacitor => {
                    if component.reference.starts_with('C') || component.reference.starts_with('c') {
                        return true;
                    }
                }
                ExternalComponentType::SeriesTerminationResistor => {
                    if component.reference.starts_with('R') || component.reference.starts_with('r') {
                        if let Some(value) = parse_resistor_value(&component.value) {
                            // Series termination typically 22-33 ohms
                            if value >= 20.0 && value <= 50.0 {
                                return true;
                            }
                        }
                    }
                }
                ExternalComponentType::ProtectionDiode => {
                    if component.reference.starts_with('D') ||
                       component.value.to_lowercase().contains("tvs") ||
                       component.value.to_lowercase().contains("esd") {
                        return true;
                    }
                }
                _ => {}
            }
        }
        
        false
    }
    
    /// Check pin requirements
    fn check_pin_requirements(
        &self,
        component: &Component,
        datasheet: &DatasheetRequirements,
        schematic: &Schematic,
    ) -> Vec<DatasheetIssue> {
        let mut issues = Vec::new();
        
        // Get all components for checking
        let all_components: Vec<&Component> = schematic
            .components
            .iter()
            .chain(schematic.power_symbols.iter())
            .collect();
        
        for req in &datasheet.pin_requirements {
            let issue = match &req.requirement {
                PinRequirementType::DefinedState => {
                    // Check if pin has a pull-up, pull-down, or direct connection
                    let has_defined_state = self.check_pin_has_defined_state(
                        component,
                        &req.pin_name,
                        &all_components,
                    );
                    
                    if !has_defined_state {
                        Some(DatasheetIssue {
                            severity: Severity::Warning,
                            component_ref: component.reference.clone(),
                            component_value: component.value.clone(),
                            requirement_type: "Pin Configuration".to_string(),
                            title: format!(
                                "{} ({}) - {} Pin State",
                                component.reference,
                                component.value,
                                req.pin_name
                            ),
                            what: format!(
                                "{} pin may be floating (undefined state)",
                                req.pin_name
                            ),
                            why: req.reason.clone(),
                            how_to_fix: format!(
                                "Add a pull-up or pull-down resistor to {} pin, or connect directly to VCC/GND",
                                req.pin_name
                            ),
                            datasheet_reference: datasheet.datasheet_url.clone(),
                            location: Some(component.position.clone()),
                        })
                    } else {
                        None
                    }
                }
                PinRequirementType::CapToGround { capacitance } => {
                    let has_cap = self.find_nearby_capacitor(
                        component,
                        capacitance.typical,
                        &all_components,
                    );
                    
                    if !has_cap {
                        Some(DatasheetIssue {
                            severity: Severity::Warning,
                            component_ref: component.reference.clone(),
                            component_value: component.value.clone(),
                            requirement_type: "Pin Configuration".to_string(),
                            title: format!(
                                "{} ({}) - {} Capacitor",
                                component.reference,
                                component.value,
                                req.pin_name
                            ),
                            what: format!(
                                "Missing capacitor on {} pin",
                                req.pin_name
                            ),
                            why: req.reason.clone(),
                            how_to_fix: format!(
                                "Add {} capacitor from {} pin to GND",
                                CapacitorValue { min: capacitance.min, typical: capacitance.typical, max: capacitance.max }.display(),
                                req.pin_name
                            ),
                            datasheet_reference: datasheet.datasheet_url.clone(),
                            location: Some(component.position.clone()),
                        })
                    } else {
                        None
                    }
                }
                PinRequirementType::PullUp { resistance_ohms } => {
                    let has_pullup = self.find_nearby_resistor(
                        component,
                        resistance_ohms.typical.unwrap_or(10_000.0),
                        &all_components,
                    );
                    
                    if !has_pullup {
                        Some(DatasheetIssue {
                            severity: Severity::Warning,
                            component_ref: component.reference.clone(),
                            component_value: component.value.clone(),
                            requirement_type: "Pin Configuration".to_string(),
                            title: format!(
                                "{} ({}) - {} Pull-up",
                                component.reference,
                                component.value,
                                req.pin_name
                            ),
                            what: format!(
                                "Missing pull-up resistor on {} pin",
                                req.pin_name
                            ),
                            why: req.reason.clone(),
                            how_to_fix: format!(
                                "Add {}Ω pull-up resistor from {} pin to VCC",
                                resistance_ohms.typical.unwrap_or(10_000.0),
                                req.pin_name
                            ),
                            datasheet_reference: datasheet.datasheet_url.clone(),
                            location: Some(component.position.clone()),
                        })
                    } else {
                        None
                    }
                }
                PinRequirementType::RcDelay { r_ohms, c_farads } => {
                    // Check for both resistor and capacitor nearby
                    let has_resistor = self.find_nearby_resistor(component, *r_ohms, &all_components);
                    let has_cap = self.find_nearby_capacitor(component, *c_farads, &all_components);
                    
                    if !has_resistor || !has_cap {
                        Some(DatasheetIssue {
                            severity: Severity::Warning,
                            component_ref: component.reference.clone(),
                            component_value: component.value.clone(),
                            requirement_type: "Pin Configuration".to_string(),
                            title: format!(
                                "{} ({}) - {} RC Delay",
                                component.reference,
                                component.value,
                                req.pin_name
                            ),
                            what: format!(
                                "Missing RC delay circuit on {} pin",
                                req.pin_name
                            ),
                            why: req.reason.clone(),
                            how_to_fix: format!(
                                "Add RC delay: {}Ω resistor to VCC, {}nF capacitor to GND",
                                r_ohms,
                                c_farads * 1e9
                            ),
                            datasheet_reference: datasheet.datasheet_url.clone(),
                            location: Some(component.position.clone()),
                        })
                    } else {
                        None
                    }
                }
                _ => None,
            };
            
            if let Some(issue) = issue {
                issues.push(issue);
            }
        }
        
        issues
    }
    
    /// Check if a pin has a defined state (pull-up, pull-down, or direct connection)
    fn check_pin_has_defined_state(
        &self,
        ic: &Component,
        _pin_name: &str,
        all_components: &[&Component],
    ) -> bool {
        // Simplified check: look for resistors nearby that could be pull-ups/downs
        for component in all_components {
            if component.reference.starts_with('R') || component.reference.starts_with('r') {
                let distance = calculate_distance(&ic.position, &component.position);
                if distance <= 30.0 {
                    if let Some(value) = parse_resistor_value(&component.value) {
                        // Typical pull-up/down range
                        if value >= 1000.0 && value <= 100_000.0 {
                            return true;
                        }
                    }
                }
            }
        }
        
        // Also check for direct connections to power symbols
        for component in all_components {
            let value_upper = component.value.to_uppercase();
            if value_upper == "GND" || value_upper == "VCC" || value_upper == "VDD" || value_upper == "3V3" {
                let distance = calculate_distance(&ic.position, &component.position);
                if distance <= 20.0 {
                    return true;
                }
            }
        }
        
        false
    }
    
    /// Find a nearby capacitor with approximately the specified value
    fn find_nearby_capacitor(
        &self,
        ic: &Component,
        target_value: f64,
        all_components: &[&Component],
    ) -> bool {
        for component in all_components {
            if component.reference.starts_with('C') || component.reference.starts_with('c') {
                let distance = calculate_distance(&ic.position, &component.position);
                if distance <= 30.0 {
                    if let Some(value) = parse_capacitor_value(&component.value) {
                        // Allow 50% tolerance
                        if value >= target_value * 0.5 && value <= target_value * 2.0 {
                            return true;
                        }
                    }
                }
            }
        }
        false
    }
    
    /// Find a nearby resistor with approximately the specified value
    fn find_nearby_resistor(
        &self,
        ic: &Component,
        target_value: f64,
        all_components: &[&Component],
    ) -> bool {
        for component in all_components {
            if component.reference.starts_with('R') || component.reference.starts_with('r') {
                let distance = calculate_distance(&ic.position, &component.position);
                if distance <= 30.0 {
                    if let Some(value) = parse_resistor_value(&component.value) {
                        // Allow 50% tolerance
                        if value >= target_value * 0.5 && value <= target_value * 2.0 {
                            return true;
                        }
                    }
                }
            }
        }
        false
    }
    
    /// Get the matcher for direct access
    pub fn matcher(&self) -> &DatasheetMatcher {
        &self.matcher
    }
}

impl Default for DatasheetChecker {
    fn default() -> Self {
        Self::new()
    }
}

/// Issue generated from datasheet checking
#[derive(Debug, Clone)]
pub struct DatasheetIssue {
    pub severity: Severity,
    pub component_ref: String,
    pub component_value: String,
    pub requirement_type: String,
    pub title: String,
    pub what: String,
    pub why: String,
    pub how_to_fix: String,
    pub datasheet_reference: Option<String>,
    pub location: Option<Position>,
}

impl From<DatasheetIssue> for Issue {
    fn from(di: DatasheetIssue) -> Self {
        Issue {
            id: uuid::Uuid::new_v4().to_string(),
            rule_id: format!("datasheet_{}", di.requirement_type.to_lowercase().replace(' ', "_")),
            severity: di.severity,
            message: format!("{}: {}", di.title, di.what),
            risk_score: None,
            component: Some(di.component_ref),
            location: di.location,
            suggestion: Some(di.how_to_fix),
        }
    }
}

/// Calculate distance between two positions
fn calculate_distance(p1: &Position, p2: &Position) -> f64 {
    let dx = p1.x - p2.x;
    let dy = p1.y - p2.y;
    (dx * dx + dy * dy).sqrt()
}

/// Parse a capacitor value string to Farads
fn parse_capacitor_value(value: &str) -> Option<f64> {
    let value = value.trim().to_lowercase();
    
    // Extract number and unit
    let mut num_str = String::new();
    let mut unit = String::new();
    let mut found_digit = false;
    
    for ch in value.chars() {
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
    
    // Convert to Farads
    let multiplier = match unit.trim() {
        "pf" | "p" => 1e-12,
        "nf" | "n" => 1e-9,
        "uf" | "u" | "µf" | "µ" => 1e-6,
        "mf" | "m" => 1e-3,
        "f" => 1.0,
        _ => {
            // Try to guess from value magnitude
            if num >= 1.0 && num <= 1000.0 {
                1e-12 // Assume pF for small values
            } else {
                1e-9 // Assume nF
            }
        }
    };
    
    Some(num * multiplier)
}

/// Parse a resistor value string to Ohms
fn parse_resistor_value(value: &str) -> Option<f64> {
    let value = value.trim().to_lowercase();
    
    // Extract number and unit
    let mut num_str = String::new();
    let mut unit = String::new();
    let mut found_digit = false;
    
    for ch in value.chars() {
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
    
    // Convert to Ohms
    let multiplier = match unit.trim() {
        "k" | "kohm" | "kω" => 1000.0,
        "m" | "mohm" | "mω" | "meg" => 1_000_000.0,
        "ohm" | "ω" | "r" | "" => 1.0,
        _ => 1.0,
    };
    
    Some(num * multiplier)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    
    fn create_test_component(reference: &str, value: &str, x: f64, y: f64) -> Component {
        Component {
            uuid: uuid::Uuid::new_v4().to_string(),
            reference: reference.to_string(),
            value: value.to_string(),
            lib_id: "Test:Component".to_string(),
            footprint: None,
            position: Position { x, y },
            rotation: 0.0,
            properties: HashMap::new(),
            pins: vec![],
        }
    }
    
    #[test]
    fn test_parse_capacitor_value() {
        assert!((parse_capacitor_value("100nF").unwrap() - 100e-9).abs() < 1e-12);
        assert!((parse_capacitor_value("4.7uF").unwrap() - 4.7e-6).abs() < 1e-9);
        assert!((parse_capacitor_value("22pF").unwrap() - 22e-12).abs() < 1e-15);
        assert!((parse_capacitor_value("10µF").unwrap() - 10e-6).abs() < 1e-9);
    }
    
    #[test]
    fn test_parse_resistor_value() {
        assert!((parse_resistor_value("10k").unwrap() - 10_000.0).abs() < 1.0);
        assert!((parse_resistor_value("4.7k").unwrap() - 4_700.0).abs() < 1.0);
        assert!((parse_resistor_value("100").unwrap() - 100.0).abs() < 0.1);
        assert!((parse_resistor_value("1M").unwrap() - 1_000_000.0).abs() < 1.0);
    }
    
    #[test]
    fn test_checker_finds_missing_decoupling() {
        let checker = DatasheetChecker::new();
        
        let schematic = Schematic {
            uuid: "test".to_string(),
            filename: "test.kicad_sch".to_string(),
            version: None,
            components: vec![
                create_test_component("U1", "STM32F411CEU6", 100.0, 100.0),
            ],
            wires: vec![],
            labels: vec![],
            nets: vec![],
            power_symbols: vec![],
        };
        
        let issues = checker.check(&schematic);
        
        // Should find missing decoupling capacitors
        assert!(!issues.is_empty());
        assert!(issues.iter().any(|i| i.requirement_type == "Decoupling Capacitor"));
    }
    
    #[test]
    fn test_checker_passes_with_decoupling() {
        let checker = DatasheetChecker::new();
        
        let schematic = Schematic {
            uuid: "test".to_string(),
            filename: "test.kicad_sch".to_string(),
            version: None,
            components: vec![
                create_test_component("U1", "STM32F411CEU6", 100.0, 100.0),
                create_test_component("C1", "100nF", 105.0, 100.0), // Bypass cap
                create_test_component("C2", "10uF", 110.0, 100.0),  // Bulk cap
                create_test_component("C3", "1uF", 103.0, 100.0),   // VDDA cap
                create_test_component("C4", "10nF", 102.0, 100.0),  // VDDA HF cap
                create_test_component("R1", "10k", 95.0, 100.0),    // Pull-up for BOOT0
            ],
            wires: vec![],
            labels: vec![],
            nets: vec![],
            power_symbols: vec![],
        };
        
        let issues = checker.check(&schematic);
        
        // Should have fewer issues (decoupling requirements met)
        let decoupling_issues: Vec<_> = issues
            .iter()
            .filter(|i| i.requirement_type == "Decoupling Capacitor")
            .collect();
        
        // May still have some issues for specific pins, but bulk decoupling should be satisfied
        assert!(decoupling_issues.len() < 4);
    }
}
