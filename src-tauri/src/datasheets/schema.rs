//! Datasheet Requirements Schema
//!
//! This module defines the data structures for representing component datasheet
//! requirements. These are used to verify that a user's circuit meets the
//! manufacturer's specifications.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Represents requirements extracted from a component datasheet
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatasheetRequirements {
    /// Component identifiers (part numbers this applies to)
    pub part_numbers: Vec<String>,
    
    /// Manufacturer
    pub manufacturer: String,
    
    /// Component category
    pub category: ComponentCategory,
    
    /// Power supply requirements
    #[serde(default)]
    pub power_requirements: Vec<PowerRequirement>,
    
    /// Decoupling capacitor requirements
    #[serde(default)]
    pub decoupling_requirements: Vec<DecouplingRequirement>,
    
    /// External component requirements (crystals, resistors, etc.)
    #[serde(default)]
    pub external_components: Vec<ExternalComponentRequirement>,
    
    /// Pin configuration requirements (pull-ups, pull-downs, defined states)
    #[serde(default)]
    pub pin_requirements: Vec<PinRequirement>,
    
    /// Maximum ratings
    #[serde(default)]
    pub absolute_max_ratings: AbsoluteMaxRatings,
    
    /// Application notes and warnings
    #[serde(default)]
    pub warnings: Vec<String>,
    
    /// Datasheet URL for reference
    pub datasheet_url: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ComponentCategory {
    Microcontroller,
    #[serde(alias = "wireless_module")]
    WirelessModule,
    #[serde(alias = "voltage_regulator")]
    VoltageRegulator,
    #[serde(alias = "usb_interface")]
    UsbInterface,
    Timer,
    #[serde(alias = "op_amp")]
    OpAmp,
    #[serde(alias = "power_management")]
    PowerManagement,
    #[serde(untagged)]
    Other(String),
}

impl Default for ComponentCategory {
    fn default() -> Self {
        ComponentCategory::Other("Unknown".to_string())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PowerRequirement {
    pub pin_name: String,
    pub voltage_min: f64,
    pub voltage_max: f64,
    pub voltage_typical: Option<f64>,
    pub current_max: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DecouplingRequirement {
    /// Which power pin this applies to
    pub power_pin: String,
    
    /// Required capacitor value
    pub capacitance: CapacitorValue,
    
    /// Maximum distance from pin (mm)
    pub max_distance_mm: f64,
    
    /// Maximum loop inductance (nH) - optional
    #[serde(default)]
    pub max_inductance_nh: Option<f64>,
    
    /// Capacitor type requirement
    pub capacitor_type: Option<CapacitorType>,
    
    /// ESR requirement (for stability)
    pub esr_requirement: Option<EsrRequirement>,
    
    /// Is this a bulk cap or bypass cap?
    pub capacitor_role: CapacitorRole,
    
    /// Severity if missing
    pub severity: DatasheetSeverity,
    
    /// Explanation for user
    pub reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CapacitorValue {
    pub min: f64,          // Farads
    pub typical: f64,      // Farads
    #[serde(default)]
    pub max: Option<f64>,  // Farads
}

impl CapacitorValue {
    /// Create a capacitor value in nanofarads
    pub fn nf(value: f64) -> Self {
        Self {
            min: value * 1e-9,
            typical: value * 1e-9,
            max: None,
        }
    }
    
    /// Create a capacitor value in microfarads
    pub fn uf(value: f64) -> Self {
        Self {
            min: value * 1e-6,
            typical: value * 1e-6,
            max: None,
        }
    }
    
    /// Create a capacitor value in picofarads
    pub fn pf(value: f64) -> Self {
        Self {
            min: value * 1e-12,
            typical: value * 1e-12,
            max: None,
        }
    }
    
    /// Get value in nanofarads
    pub fn as_nf(&self) -> f64 {
        self.typical * 1e9
    }
    
    /// Get value in microfarads
    pub fn as_uf(&self) -> f64 {
        self.typical * 1e6
    }
    
    /// Format for display
    pub fn display(&self) -> String {
        let value = self.typical;
        if value >= 1e-6 {
            format!("{:.1}µF", value * 1e6)
        } else if value >= 1e-9 {
            format!("{:.0}nF", value * 1e9)
        } else {
            format!("{:.0}pF", value * 1e12)
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum CapacitorType {
    Ceramic,
    Tantalum,
    Electrolytic,
    FilmOrCeramic,
    Any,
}

impl Default for CapacitorType {
    fn default() -> Self {
        CapacitorType::Any
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum CapacitorRole {
    Bypass,     // High-frequency filtering (typically 100nF)
    Bulk,       // Energy storage (typically 4.7µF-47µF)
    Decoupling, // General term
    Filter,     // Specific filtering application
}

impl Default for CapacitorRole {
    fn default() -> Self {
        CapacitorRole::Decoupling
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EsrRequirement {
    pub min_ohms: Option<f64>,
    pub max_ohms: Option<f64>,
    pub reason: String,  // e.g., "LDO stability requires ESR 0.1-0.5Ω"
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExternalComponentRequirement {
    /// What type of component
    pub component_type: ExternalComponentType,
    
    /// Which pin(s) it connects to
    pub connected_pins: Vec<String>,
    
    /// Value requirement
    pub value_requirement: ValueRequirement,
    
    /// Is this required or optional?
    #[serde(default = "default_true")]
    pub required: bool,
    
    /// Explanation
    pub reason: String,
}

fn default_true() -> bool {
    true
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ExternalComponentType {
    Crystal { 
        frequency_hz: f64, 
        load_capacitance_pf: f64 
    },
    Oscillator { 
        frequency_hz: f64 
    },
    PullUpResistor,
    PullDownResistor,
    CurrentLimitResistor,
    SeriesTerminationResistor,
    FilterCapacitor,
    ProtectionDiode,
    Inductor,
    Ferrite,
    ResetCapacitor,
    BypassCapacitor,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ValueRequirement {
    pub min: Option<f64>,
    pub typical: Option<f64>,
    pub max: Option<f64>,
    pub unit: String,
}

impl ValueRequirement {
    pub fn ohms(min: Option<f64>, typical: Option<f64>, max: Option<f64>) -> Self {
        Self { min, typical, max, unit: "Ω".to_string() }
    }
    
    pub fn farads(min: Option<f64>, typical: Option<f64>, max: Option<f64>) -> Self {
        Self { min, typical, max, unit: "F".to_string() }
    }
    
    pub fn hertz(min: Option<f64>, typical: Option<f64>, max: Option<f64>) -> Self {
        Self { min, typical, max, unit: "Hz".to_string() }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PinRequirement {
    pub pin_name: String,
    pub requirement: PinRequirementType,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum PinRequirementType {
    /// Must be pulled high
    PullUp { 
        resistance_ohms: ValueRequirement 
    },
    
    /// Must be pulled low
    PullDown { 
        resistance_ohms: ValueRequirement 
    },
    
    /// Must have a defined state (high or low), not floating
    DefinedState,
    
    /// Must have capacitor to ground
    CapToGround { 
        capacitance: CapacitorValue 
    },
    
    /// Should have ESD protection
    EsdProtection,
    
    /// Requires RC delay circuit
    RcDelay { 
        r_ohms: f64, 
        c_farads: f64 
    },
    
    /// Must not exceed voltage
    MaxVoltage { 
        voltage: f64 
    },
    
    /// Must be connected to specific net
    ConnectTo {
        net_name: String,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AbsoluteMaxRatings {
    pub vcc_max: Option<f64>,
    pub io_voltage_max: Option<f64>,
    pub io_current_max: Option<f64>,
    pub storage_temp_min: Option<f64>,
    pub storage_temp_max: Option<f64>,
    pub operating_temp_min: Option<f64>,
    pub operating_temp_max: Option<f64>,
    pub esd_hbm: Option<f64>,  // Human Body Model rating
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum DatasheetSeverity {
    Error,      // Will definitely cause problems
    Warning,    // Likely to cause problems
    Info,       // Recommendation
}

impl Default for DatasheetSeverity {
    fn default() -> Self {
        DatasheetSeverity::Warning
    }
}

impl From<DatasheetSeverity> for crate::analyzer::rules::Severity {
    fn from(ds: DatasheetSeverity) -> Self {
        match ds {
            DatasheetSeverity::Error => crate::analyzer::rules::Severity::Error,
            DatasheetSeverity::Warning => crate::analyzer::rules::Severity::Warning,
            DatasheetSeverity::Info => crate::analyzer::rules::Severity::Info,
        }
    }
}

/// Database of all loaded datasheet requirements
#[derive(Debug, Default)]
pub struct DatasheetDatabase {
    /// Map of normalized part number to requirements
    pub datasheets: HashMap<String, DatasheetRequirements>,
    
    /// Aliases for part numbers (maps alias -> canonical part number)
    pub aliases: HashMap<String, String>,
}

impl DatasheetDatabase {
    pub fn new() -> Self {
        Self::default()
    }
    
    /// Add a datasheet to the database
    pub fn add(&mut self, requirements: DatasheetRequirements) {
        // Use first part number as canonical
        if let Some(canonical) = requirements.part_numbers.first() {
            let canonical_normalized = Self::normalize_part_number(canonical);
            
            // Add aliases for all part numbers
            for pn in &requirements.part_numbers {
                let normalized = Self::normalize_part_number(pn);
                if normalized != canonical_normalized {
                    self.aliases.insert(normalized, canonical_normalized.clone());
                }
            }
            
            self.datasheets.insert(canonical_normalized, requirements);
        }
    }
    
    /// Look up requirements by part number
    pub fn get(&self, part_number: &str) -> Option<&DatasheetRequirements> {
        let normalized = Self::normalize_part_number(part_number);
        
        // Direct lookup
        if let Some(req) = self.datasheets.get(&normalized) {
            return Some(req);
        }
        
        // Try alias
        if let Some(canonical) = self.aliases.get(&normalized) {
            return self.datasheets.get(canonical);
        }
        
        // Try fuzzy match
        self.fuzzy_match(part_number)
    }
    
    /// Normalize a part number for comparison
    fn normalize_part_number(pn: &str) -> String {
        pn.to_uppercase()
            .chars()
            .filter(|c| c.is_alphanumeric())
            .collect()
    }
    
    /// Fuzzy match a part number
    fn fuzzy_match(&self, part_number: &str) -> Option<&DatasheetRequirements> {
        let normalized = Self::normalize_part_number(part_number);
        
        // Try prefix matching (e.g., "STM32F411" matches "STM32F411CEU6")
        for (key, req) in &self.datasheets {
            // Check if any part number in the datasheet starts with our query
            for pn in &req.part_numbers {
                let pn_normalized = Self::normalize_part_number(pn);
                if pn_normalized.starts_with(&normalized) || normalized.starts_with(&pn_normalized) {
                    return Some(req);
                }
            }
        }
        
        // Try substring matching for common patterns
        for (key, req) in &self.datasheets {
            for pn in &req.part_numbers {
                let pn_normalized = Self::normalize_part_number(pn);
                // Check for common IC naming patterns
                if normalized.contains(&pn_normalized) || pn_normalized.contains(&normalized) {
                    return Some(req);
                }
            }
        }
        
        None
    }
    
    /// Get all loaded datasheets
    pub fn all(&self) -> impl Iterator<Item = &DatasheetRequirements> {
        self.datasheets.values()
    }
    
    /// Get count of loaded datasheets
    pub fn count(&self) -> usize {
        self.datasheets.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_capacitor_value_display() {
        assert_eq!(CapacitorValue::nf(100.0).display(), "100nF");
        assert_eq!(CapacitorValue::uf(4.7).display(), "4.7µF");
        assert_eq!(CapacitorValue::pf(22.0).display(), "22pF");
    }
    
    #[test]
    fn test_normalize_part_number() {
        assert_eq!(
            DatasheetDatabase::normalize_part_number("STM32F411CEU6"),
            "STM32F411CEU6"
        );
        assert_eq!(
            DatasheetDatabase::normalize_part_number("stm32f411ceu6"),
            "STM32F411CEU6"
        );
        assert_eq!(
            DatasheetDatabase::normalize_part_number("STM32-F411"),
            "STM32F411"
        );
    }
    
    #[test]
    fn test_database_lookup() {
        let mut db = DatasheetDatabase::new();
        
        let req = DatasheetRequirements {
            part_numbers: vec!["STM32F411CEU6".to_string(), "STM32F411CCU6".to_string()],
            manufacturer: "STMicroelectronics".to_string(),
            category: ComponentCategory::Microcontroller,
            power_requirements: vec![],
            decoupling_requirements: vec![],
            external_components: vec![],
            pin_requirements: vec![],
            absolute_max_ratings: AbsoluteMaxRatings::default(),
            warnings: vec![],
            datasheet_url: None,
        };
        
        db.add(req);
        
        // Direct match
        assert!(db.get("STM32F411CEU6").is_some());
        
        // Alias match
        assert!(db.get("STM32F411CCU6").is_some());
        
        // Case insensitive
        assert!(db.get("stm32f411ceu6").is_some());
        
        // Fuzzy match (prefix)
        assert!(db.get("STM32F411").is_some());
    }
}
