//! IPC-2221 Current Carrying Capacity Calculator
//!
//! Implements the IPC-2221 standard curve fit equations for calculating
//! the current carrying capacity of PCB traces based on:
//! - Trace width (W)
//! - Copper thickness (T)
//! - Allowable temperature rise (ΔT)
//!
//! The formula: I = k × ΔT^b × A^c
//! Where A = W × T (cross-sectional area)
//!
//! Reference: IPC-2221A Generic Standard on Printed Board Design

use serde::{Deserialize, Serialize};
use crate::parser::pcb_schema::{PcbDesign, Trace};

/// IPC-2221 Constants for internal layers
pub const IPC2221_INTERNAL_K: f64 = 0.024;
pub const IPC2221_INTERNAL_B: f64 = 0.44;
pub const IPC2221_INTERNAL_C: f64 = 0.725;

/// IPC-2221 Constants for external layers
pub const IPC2221_EXTERNAL_K: f64 = 0.048;
pub const IPC2221_EXTERNAL_B: f64 = 0.44;
pub const IPC2221_EXTERNAL_C: f64 = 0.725;

/// Standard copper weights and their thicknesses
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum CopperWeight {
    HalfOz,    // 0.5 oz = 17.5 µm = 0.0175 mm
    OneOz,     // 1 oz = 35 µm = 0.035 mm
    TwoOz,     // 2 oz = 70 µm = 0.070 mm
    ThreeOz,   // 3 oz = 105 µm = 0.105 mm
}

impl CopperWeight {
    /// Get thickness in millimeters
    pub fn thickness_mm(&self) -> f64 {
        match self {
            CopperWeight::HalfOz => 0.0175,
            CopperWeight::OneOz => 0.035,
            CopperWeight::TwoOz => 0.070,
            CopperWeight::ThreeOz => 0.105,
        }
    }
    
    /// Get thickness in mils (thousandths of an inch)
    pub fn thickness_mils(&self) -> f64 {
        self.thickness_mm() / 0.0254
    }
    
    /// Get weight in oz/ft²
    pub fn weight_oz(&self) -> f64 {
        match self {
            CopperWeight::HalfOz => 0.5,
            CopperWeight::OneOz => 1.0,
            CopperWeight::TwoOz => 2.0,
            CopperWeight::ThreeOz => 3.0,
        }
    }
    
    /// Create from oz value
    pub fn from_oz(oz: f64) -> Self {
        if oz <= 0.75 {
            CopperWeight::HalfOz
        } else if oz <= 1.5 {
            CopperWeight::OneOz
        } else if oz <= 2.5 {
            CopperWeight::TwoOz
        } else {
            CopperWeight::ThreeOz
        }
    }
}

/// IPC-2221 Calculator for trace current capacity
#[derive(Debug, Clone)]
pub struct Ipc2221Calculator {
    /// Default temperature rise in °C
    pub default_temp_rise: f64,
    /// Default copper weight for outer layers
    pub outer_copper: CopperWeight,
    /// Default copper weight for inner layers
    pub inner_copper: CopperWeight,
}

impl Default for Ipc2221Calculator {
    fn default() -> Self {
        Self {
            default_temp_rise: 10.0,  // 10°C rise is common default
            outer_copper: CopperWeight::OneOz,
            inner_copper: CopperWeight::HalfOz,
        }
    }
}

impl Ipc2221Calculator {
    pub fn new() -> Self {
        Self::default()
    }
    
    pub fn with_copper(outer_oz: f64, inner_oz: f64) -> Self {
        Self {
            default_temp_rise: 10.0,
            outer_copper: CopperWeight::from_oz(outer_oz),
            inner_copper: CopperWeight::from_oz(inner_oz),
        }
    }
    
    /// Calculate maximum current for a trace
    ///
    /// # Arguments
    /// * `width_mm` - Trace width in millimeters
    /// * `thickness_mm` - Copper thickness in millimeters
    /// * `temp_rise_c` - Allowable temperature rise in °C
    /// * `is_external` - True for outer layers, false for internal
    ///
    /// # Returns
    /// Maximum current in Amperes
    pub fn calculate_max_current(
        &self,
        width_mm: f64,
        thickness_mm: f64,
        temp_rise_c: f64,
        is_external: bool,
    ) -> f64 {
        // Convert to mils² for the formula
        let width_mils = width_mm / 0.0254;
        let thickness_mils = thickness_mm / 0.0254;
        let area_mils2 = width_mils * thickness_mils;
        
        let (k, b, c) = if is_external {
            (IPC2221_EXTERNAL_K, IPC2221_EXTERNAL_B, IPC2221_EXTERNAL_C)
        } else {
            (IPC2221_INTERNAL_K, IPC2221_INTERNAL_B, IPC2221_INTERNAL_C)
        };
        
        // I = k × ΔT^b × A^c
        k * temp_rise_c.powf(b) * area_mils2.powf(c)
    }
    
    /// Calculate required trace width for a given current
    ///
    /// # Arguments
    /// * `current_a` - Required current in Amperes
    /// * `thickness_mm` - Copper thickness in millimeters
    /// * `temp_rise_c` - Allowable temperature rise in °C
    /// * `is_external` - True for outer layers, false for internal
    ///
    /// # Returns
    /// Required trace width in millimeters
    pub fn calculate_required_width(
        &self,
        current_a: f64,
        thickness_mm: f64,
        temp_rise_c: f64,
        is_external: bool,
    ) -> f64 {
        let (k, b, c) = if is_external {
            (IPC2221_EXTERNAL_K, IPC2221_EXTERNAL_B, IPC2221_EXTERNAL_C)
        } else {
            (IPC2221_INTERNAL_K, IPC2221_INTERNAL_B, IPC2221_INTERNAL_C)
        };
        
        // Rearrange: A = (I / (k × ΔT^b))^(1/c)
        let area_mils2 = (current_a / (k * temp_rise_c.powf(b))).powf(1.0 / c);
        
        // Convert thickness to mils
        let thickness_mils = thickness_mm / 0.0254;
        
        // Width = Area / Thickness
        let width_mils = area_mils2 / thickness_mils;
        
        // Convert back to mm
        width_mils * 0.0254
    }
    
    /// Calculate temperature rise for a given current and trace
    ///
    /// # Arguments
    /// * `current_a` - Current in Amperes
    /// * `width_mm` - Trace width in millimeters
    /// * `thickness_mm` - Copper thickness in millimeters
    /// * `is_external` - True for outer layers, false for internal
    ///
    /// # Returns
    /// Temperature rise in °C
    pub fn calculate_temp_rise(
        &self,
        current_a: f64,
        width_mm: f64,
        thickness_mm: f64,
        is_external: bool,
    ) -> f64 {
        let (k, b, c) = if is_external {
            (IPC2221_EXTERNAL_K, IPC2221_EXTERNAL_B, IPC2221_EXTERNAL_C)
        } else {
            (IPC2221_INTERNAL_K, IPC2221_INTERNAL_B, IPC2221_INTERNAL_C)
        };
        
        // Convert to mils²
        let width_mils = width_mm / 0.0254;
        let thickness_mils = thickness_mm / 0.0254;
        let area_mils2 = width_mils * thickness_mils;
        
        // Rearrange: ΔT = (I / (k × A^c))^(1/b)
        (current_a / (k * area_mils2.powf(c))).powf(1.0 / b)
    }
    
    /// Analyze all traces in a PCB design
    pub fn analyze_pcb(&self, pcb: &PcbDesign) -> Vec<TraceCurrentAnalysis> {
        let mut results = Vec::new();
        
        for trace in &pcb.traces {
            let is_external = Self::is_external_layer(&trace.layer);
            let copper_thickness = if is_external {
                pcb.setup.copper_thickness.outer_mm()
            } else {
                pcb.setup.copper_thickness.inner_mm()
            };
            
            let max_current = self.calculate_max_current(
                trace.width,
                copper_thickness,
                self.default_temp_rise,
                is_external,
            );
            
            results.push(TraceCurrentAnalysis {
                trace_uuid: trace.uuid.clone(),
                net_name: trace.net_name.clone().unwrap_or_else(|| format!("Net{}", trace.net)),
                layer: trace.layer.clone(),
                width_mm: trace.width,
                length_mm: trace.length(),
                copper_thickness_mm: copper_thickness,
                is_external,
                max_current_a: max_current,
                temp_rise_c: self.default_temp_rise,
            });
        }
        
        results
    }
    
    /// Check if a layer is external (F.Cu or B.Cu)
    fn is_external_layer(layer: &str) -> bool {
        layer == "F.Cu" || layer == "B.Cu"
    }
}

/// Result of trace current analysis
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TraceCurrentAnalysis {
    pub trace_uuid: String,
    pub net_name: String,
    pub layer: String,
    pub width_mm: f64,
    pub length_mm: f64,
    pub copper_thickness_mm: f64,
    pub is_external: bool,
    pub max_current_a: f64,
    pub temp_rise_c: f64,
}

impl TraceCurrentAnalysis {
    /// Check if trace can handle a specific current
    pub fn can_handle_current(&self, current_a: f64) -> bool {
        current_a <= self.max_current_a
    }
    
    /// Get safety margin percentage
    pub fn safety_margin(&self, actual_current_a: f64) -> f64 {
        if actual_current_a <= 0.0 {
            return 100.0;
        }
        ((self.max_current_a - actual_current_a) / actual_current_a) * 100.0
    }
}

/// IPC-2221 compliance issue
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CurrentCapacityIssue {
    pub trace_uuid: String,
    pub net_name: String,
    pub layer: String,
    pub current_width_mm: f64,
    pub required_width_mm: f64,
    pub max_current_a: f64,
    pub expected_current_a: f64,
    pub severity: IssueSeverity,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum IssueSeverity {
    Critical,  // Trace will likely fail
    Warning,   // Trace is undersized but may work
    Info,      // Informational
}

/// Check power traces against expected currents
pub fn check_power_traces(
    pcb: &PcbDesign,
    power_net_currents: &[(String, f64)],  // (net_name, expected_current_a)
    temp_rise_c: f64,
) -> Vec<CurrentCapacityIssue> {
    let calculator = Ipc2221Calculator::default();
    let mut issues = Vec::new();
    
    for (net_name, expected_current) in power_net_currents {
        // Find all traces for this net
        let net_traces: Vec<&Trace> = pcb.traces
            .iter()
            .filter(|t| t.net_name.as_ref() == Some(net_name))
            .collect();
        
        for trace in net_traces {
            let is_external = Ipc2221Calculator::is_external_layer(&trace.layer);
            let copper_thickness = if is_external {
                pcb.setup.copper_thickness.outer_mm()
            } else {
                pcb.setup.copper_thickness.inner_mm()
            };
            
            let max_current = calculator.calculate_max_current(
                trace.width,
                copper_thickness,
                temp_rise_c,
                is_external,
            );
            
            if max_current < *expected_current {
                let required_width = calculator.calculate_required_width(
                    *expected_current,
                    copper_thickness,
                    temp_rise_c,
                    is_external,
                );
                
                let deficit_percent = ((*expected_current - max_current) / *expected_current) * 100.0;
                
                let severity = if deficit_percent > 50.0 {
                    IssueSeverity::Critical
                } else if deficit_percent > 20.0 {
                    IssueSeverity::Warning
                } else {
                    IssueSeverity::Info
                };
                
                issues.push(CurrentCapacityIssue {
                    trace_uuid: trace.uuid.clone(),
                    net_name: net_name.clone(),
                    layer: trace.layer.clone(),
                    current_width_mm: trace.width,
                    required_width_mm: required_width,
                    max_current_a: max_current,
                    expected_current_a: *expected_current,
                    severity,
                    message: format!(
                        "Trace on {} for net '{}' is undersized: {:.3}mm width can handle {:.2}A, \
                         but {:.2}A expected. Recommend {:.3}mm width for {}°C rise.",
                        trace.layer,
                        net_name,
                        trace.width,
                        max_current,
                        expected_current,
                        required_width,
                        temp_rise_c
                    ),
                });
            }
        }
    }
    
    issues
}

/// Generate a current capacity report for all traces
pub fn generate_current_report(pcb: &PcbDesign, temp_rise_c: f64) -> CurrentCapacityReport {
    let calculator = Ipc2221Calculator {
        default_temp_rise: temp_rise_c,
        ..Default::default()
    };
    
    let analyses = calculator.analyze_pcb(pcb);
    
    // Group by net
    let mut net_summaries: std::collections::HashMap<String, NetCurrentSummary> = 
        std::collections::HashMap::new();
    
    for analysis in &analyses {
        let entry = net_summaries
            .entry(analysis.net_name.clone())
            .or_insert_with(|| NetCurrentSummary {
                net_name: analysis.net_name.clone(),
                min_width_mm: f64::MAX,
                max_width_mm: 0.0,
                min_current_capacity_a: f64::MAX,
                total_length_mm: 0.0,
                trace_count: 0,
            });
        
        entry.min_width_mm = entry.min_width_mm.min(analysis.width_mm);
        entry.max_width_mm = entry.max_width_mm.max(analysis.width_mm);
        entry.min_current_capacity_a = entry.min_current_capacity_a.min(analysis.max_current_a);
        entry.total_length_mm += analysis.length_mm;
        entry.trace_count += 1;
    }
    
    CurrentCapacityReport {
        temp_rise_c,
        outer_copper_oz: calculator.outer_copper.weight_oz(),
        inner_copper_oz: calculator.inner_copper.weight_oz(),
        trace_analyses: analyses,
        net_summaries: net_summaries.into_values().collect(),
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CurrentCapacityReport {
    pub temp_rise_c: f64,
    pub outer_copper_oz: f64,
    pub inner_copper_oz: f64,
    pub trace_analyses: Vec<TraceCurrentAnalysis>,
    pub net_summaries: Vec<NetCurrentSummary>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetCurrentSummary {
    pub net_name: String,
    pub min_width_mm: f64,
    pub max_width_mm: f64,
    pub min_current_capacity_a: f64,
    pub total_length_mm: f64,
    pub trace_count: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_copper_weight_thickness() {
        assert!((CopperWeight::OneOz.thickness_mm() - 0.035).abs() < 0.0001);
        assert!((CopperWeight::TwoOz.thickness_mm() - 0.070).abs() < 0.0001);
    }

    #[test]
    fn test_max_current_calculation() {
        let calc = Ipc2221Calculator::default();
        
        // 10 mil (0.254mm) trace, 1oz copper, 10°C rise, external
        let current = calc.calculate_max_current(0.254, 0.035, 10.0, true);
        
        // Should be approximately 0.5-1A for this configuration
        assert!(current > 0.3 && current < 2.0, "Current: {}", current);
    }

    #[test]
    fn test_required_width_calculation() {
        let calc = Ipc2221Calculator::default();
        
        // Need 1A, 1oz copper, 10°C rise, external
        let width = calc.calculate_required_width(1.0, 0.035, 10.0, true);
        
        // Should be reasonable trace width
        assert!(width > 0.1 && width < 5.0, "Width: {}", width);
    }

    #[test]
    fn test_temp_rise_calculation() {
        let calc = Ipc2221Calculator::default();
        
        // 0.5mm trace, 1oz copper, 1A current, external
        let temp_rise = calc.calculate_temp_rise(1.0, 0.5, 0.035, true);
        
        // Should be reasonable temperature rise
        assert!(temp_rise > 0.0 && temp_rise < 100.0, "Temp rise: {}", temp_rise);
    }

    #[test]
    fn test_internal_vs_external() {
        let calc = Ipc2221Calculator::default();
        
        // Same trace parameters
        let external_current = calc.calculate_max_current(0.5, 0.035, 10.0, true);
        let internal_current = calc.calculate_max_current(0.5, 0.035, 10.0, false);
        
        // External should handle more current due to better heat dissipation
        assert!(external_current > internal_current);
    }
}
