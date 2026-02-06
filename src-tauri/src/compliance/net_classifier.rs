//! Net Classification Module
//!
//! Classifies PCB nets by their signal type using pattern matching
//! and heuristics. Identifies high-speed signals (USB, HDMI, Ethernet),
//! clocks, power rails, and analog signals.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use crate::parser::pcb_schema::{PcbDesign, NetClassification};

/// High-speed signal patterns
const HIGH_SPEED_PATTERNS: &[&str] = &[
    // USB
    "USB", "D+", "D-", "DP", "DM", "USBDP", "USBDM",
    "USB_D+", "USB_D-", "USB_DP", "USB_DM",
    "USB2", "USB3", "SSRX", "SSTX", "SS_RX", "SS_TX",
    // HDMI
    "HDMI", "TMDS", "HPD", "CEC", "DDC",
    "HDMI_D0", "HDMI_D1", "HDMI_D2", "HDMI_CLK",
    // DisplayPort
    "DP_", "AUX", "LANE0", "LANE1", "LANE2", "LANE3",
    // Ethernet
    "ETH", "RGMII", "RMII", "MII", "MDIO", "MDC",
    "TX+", "TX-", "RX+", "RX-", "TXP", "TXN", "RXP", "RXN",
    "TD+", "TD-", "RD+", "RD-",
    // PCIe
    "PCIE", "PCI_", "PERST", "CLKREQ", "REFCLK",
    "PERN", "PERP", "PETN", "PETP",
    // SATA
    "SATA", "SATA_TX", "SATA_RX",
    // DDR Memory
    "DDR", "DQ", "DQS", "DQM", "CAS", "RAS", "WE",
    "A0", "A1", "A2", "A3", "A4", "A5", "A6", "A7",
    "A8", "A9", "A10", "A11", "A12", "A13", "A14", "A15",
    "BA0", "BA1", "BA2",
    // LVDS
    "LVDS", "_P", "_N",
    // MIPI
    "MIPI", "CSI", "DSI", "DPHY",
    // QSPI/SPI High Speed
    "QSPI", "OSPI",
];

/// Clock signal patterns
const CLOCK_PATTERNS: &[&str] = &[
    "CLK", "CLOCK", "CK", "XTAL", "OSC",
    "MCLK", "BCLK", "LRCLK", "SCLK", "PCLK",
    "SYSCLK", "REFCLK", "FCLK", "HCLK",
    "CLK_IN", "CLK_OUT", "CLKIN", "CLKOUT",
    "32K", "24M", "25M", "48M", "100M", "125M",
    "PLL", "DPLL",
];

/// Power net patterns
const POWER_PATTERNS: &[&str] = &[
    "VCC", "VDD", "VBAT", "VBUS", "VIN", "VOUT",
    "3V3", "3.3V", "5V", "12V", "1V8", "1.8V", "2V5", "2.5V",
    "AVCC", "AVDD", "DVCC", "DVDD", "PVCC", "PVDD",
    "VCCA", "VCCD", "VCCIO", "VDDIO",
    "V+", "V-", "+5V", "+3.3V", "+12V", "-5V", "-12V",
    "PWR", "POWER", "SUPPLY",
    "VCORE", "VREF", "VDDA", "VSSA",
];

/// Ground net patterns
const GROUND_PATTERNS: &[&str] = &[
    "GND", "GROUND", "VSS", "GNDA", "GNDD",
    "AGND", "DGND", "PGND", "SGND",
    "AVSS", "DVSS", "PVSS",
    "0V", "COM", "COMMON",
];

/// Analog signal patterns
const ANALOG_PATTERNS: &[&str] = &[
    "AIN", "AOUT", "ANALOG", "ADC", "DAC",
    "AUDIO", "MIC", "SPK", "LINE",
    "VREF", "SENSE", "TEMP",
    "AN0", "AN1", "AN2", "AN3", "AN4", "AN5", "AN6", "AN7",
];

/// Net classifier using pattern matching
pub struct NetClassifier {
    /// Custom patterns for high-speed nets
    custom_high_speed: Vec<String>,
    /// Custom patterns for clock nets
    custom_clock: Vec<String>,
    /// Custom patterns for power nets
    custom_power: Vec<String>,
    /// Custom patterns for ground nets
    custom_ground: Vec<String>,
    /// Custom patterns for analog nets
    custom_analog: Vec<String>,
}

impl Default for NetClassifier {
    fn default() -> Self {
        Self {
            custom_high_speed: Vec::new(),
            custom_clock: Vec::new(),
            custom_power: Vec::new(),
            custom_ground: Vec::new(),
            custom_analog: Vec::new(),
        }
    }
}

impl NetClassifier {
    pub fn new() -> Self {
        Self::default()
    }
    
    /// Add custom high-speed pattern
    pub fn add_high_speed_pattern(&mut self, pattern: &str) {
        self.custom_high_speed.push(pattern.to_uppercase());
    }
    
    /// Add custom clock pattern
    pub fn add_clock_pattern(&mut self, pattern: &str) {
        self.custom_clock.push(pattern.to_uppercase());
    }
    
    /// Classify a single net by name
    pub fn classify_net(&self, net_name: &str) -> NetClassification {
        let name_upper = net_name.to_uppercase();
        
        // Check ground first (highest priority)
        if self.matches_patterns(&name_upper, GROUND_PATTERNS, &self.custom_ground) {
            return NetClassification::Ground;
        }
        
        // Check power
        if self.matches_patterns(&name_upper, POWER_PATTERNS, &self.custom_power) {
            return NetClassification::Power;
        }
        
        // Check high-speed
        if self.matches_patterns(&name_upper, HIGH_SPEED_PATTERNS, &self.custom_high_speed) {
            return NetClassification::HighSpeed;
        }
        
        // Check clock
        if self.matches_patterns(&name_upper, CLOCK_PATTERNS, &self.custom_clock) {
            return NetClassification::Clock;
        }
        
        // Check analog
        if self.matches_patterns(&name_upper, ANALOG_PATTERNS, &self.custom_analog) {
            return NetClassification::Analog;
        }
        
        // Default to digital
        NetClassification::Digital
    }
    
    /// Classify all nets in a PCB design
    pub fn classify_nets(&self, pcb: &PcbDesign) -> HashMap<String, NetClassification> {
        let mut classifications = HashMap::new();
        
        for net in &pcb.nets {
            let classification = self.classify_net(&net.name);
            classifications.insert(net.name.clone(), classification);
        }
        
        classifications
    }
    
    /// Check if name matches any pattern
    fn matches_patterns(&self, name: &str, builtin: &[&str], custom: &[String]) -> bool {
        // Check builtin patterns
        for pattern in builtin {
            if name.contains(pattern) {
                return true;
            }
        }
        
        // Check custom patterns
        for pattern in custom {
            if name.contains(pattern) {
                return true;
            }
        }
        
        false
    }
    
    /// Get classification details
    pub fn get_classification_info(classification: &NetClassification) -> ClassificationInfo {
        match classification {
            NetClassification::HighSpeed => ClassificationInfo {
                name: "High-Speed".to_string(),
                description: "High-speed digital signals requiring controlled impedance".to_string(),
                typical_impedance: Some("50Ω single-ended, 90-100Ω differential".to_string()),
                routing_requirements: vec![
                    "Maintain continuous reference plane".to_string(),
                    "Match trace lengths for differential pairs".to_string(),
                    "Avoid vias and layer transitions".to_string(),
                    "Use appropriate termination".to_string(),
                ],
            },
            NetClassification::Clock => ClassificationInfo {
                name: "Clock".to_string(),
                description: "Clock signals with strict timing requirements".to_string(),
                typical_impedance: Some("50Ω typical".to_string()),
                routing_requirements: vec![
                    "Keep traces short and direct".to_string(),
                    "Avoid routing near sensitive analog circuits".to_string(),
                    "Consider guard traces for isolation".to_string(),
                    "Use series termination at source".to_string(),
                ],
            },
            NetClassification::Power => ClassificationInfo {
                name: "Power".to_string(),
                description: "Power distribution nets".to_string(),
                typical_impedance: None,
                routing_requirements: vec![
                    "Use wide traces or planes".to_string(),
                    "Add decoupling capacitors near loads".to_string(),
                    "Consider current carrying capacity".to_string(),
                    "Minimize inductance with short, wide paths".to_string(),
                ],
            },
            NetClassification::Ground => ClassificationInfo {
                name: "Ground".to_string(),
                description: "Ground reference nets".to_string(),
                typical_impedance: None,
                routing_requirements: vec![
                    "Use solid ground planes when possible".to_string(),
                    "Avoid splits under high-speed signals".to_string(),
                    "Provide adequate return paths".to_string(),
                    "Use multiple vias for plane connections".to_string(),
                ],
            },
            NetClassification::Analog => ClassificationInfo {
                name: "Analog".to_string(),
                description: "Analog signals sensitive to noise".to_string(),
                typical_impedance: None,
                routing_requirements: vec![
                    "Keep away from digital signals".to_string(),
                    "Use guard traces or ground shields".to_string(),
                    "Minimize trace length".to_string(),
                    "Consider separate analog ground".to_string(),
                ],
            },
            NetClassification::Digital => ClassificationInfo {
                name: "Digital".to_string(),
                description: "Standard digital signals".to_string(),
                typical_impedance: None,
                routing_requirements: vec![
                    "Follow standard design rules".to_string(),
                    "Maintain adequate clearance".to_string(),
                ],
            },
            NetClassification::Unknown => ClassificationInfo {
                name: "Unknown".to_string(),
                description: "Unclassified net".to_string(),
                typical_impedance: None,
                routing_requirements: vec![],
            },
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClassificationInfo {
    pub name: String,
    pub description: String,
    pub typical_impedance: Option<String>,
    pub routing_requirements: Vec<String>,
}

/// Summary of net classifications for a design
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetClassificationSummary {
    pub total_nets: usize,
    pub high_speed_count: usize,
    pub clock_count: usize,
    pub power_count: usize,
    pub ground_count: usize,
    pub analog_count: usize,
    pub digital_count: usize,
    pub high_speed_nets: Vec<String>,
    pub clock_nets: Vec<String>,
}

/// Generate classification summary for a PCB
pub fn generate_classification_summary(pcb: &PcbDesign) -> NetClassificationSummary {
    let classifier = NetClassifier::default();
    let classifications = classifier.classify_nets(pcb);
    
    let mut summary = NetClassificationSummary {
        total_nets: classifications.len(),
        high_speed_count: 0,
        clock_count: 0,
        power_count: 0,
        ground_count: 0,
        analog_count: 0,
        digital_count: 0,
        high_speed_nets: Vec::new(),
        clock_nets: Vec::new(),
    };
    
    for (name, class) in &classifications {
        match class {
            NetClassification::HighSpeed => {
                summary.high_speed_count += 1;
                summary.high_speed_nets.push(name.clone());
            }
            NetClassification::Clock => {
                summary.clock_count += 1;
                summary.clock_nets.push(name.clone());
            }
            NetClassification::Power => summary.power_count += 1,
            NetClassification::Ground => summary.ground_count += 1,
            NetClassification::Analog => summary.analog_count += 1,
            NetClassification::Digital => summary.digital_count += 1,
            NetClassification::Unknown => {}
        }
    }
    
    summary.high_speed_nets.sort();
    summary.clock_nets.sort();
    
    summary
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_usb_classification() {
        let classifier = NetClassifier::default();
        
        assert_eq!(classifier.classify_net("USB_D+"), NetClassification::HighSpeed);
        assert_eq!(classifier.classify_net("USB_D-"), NetClassification::HighSpeed);
        assert_eq!(classifier.classify_net("USBDP"), NetClassification::HighSpeed);
        assert_eq!(classifier.classify_net("USBDM"), NetClassification::HighSpeed);
    }

    #[test]
    fn test_clock_classification() {
        let classifier = NetClassifier::default();
        
        assert_eq!(classifier.classify_net("CLK_48M"), NetClassification::Clock);
        assert_eq!(classifier.classify_net("SYSCLK"), NetClassification::Clock);
        assert_eq!(classifier.classify_net("XTAL_IN"), NetClassification::Clock);
    }

    #[test]
    fn test_power_classification() {
        let classifier = NetClassifier::default();
        
        assert_eq!(classifier.classify_net("VCC"), NetClassification::Power);
        assert_eq!(classifier.classify_net("3V3"), NetClassification::Power);
        assert_eq!(classifier.classify_net("+5V"), NetClassification::Power);
    }

    #[test]
    fn test_ground_classification() {
        let classifier = NetClassifier::default();
        
        assert_eq!(classifier.classify_net("GND"), NetClassification::Ground);
        assert_eq!(classifier.classify_net("AGND"), NetClassification::Ground);
        assert_eq!(classifier.classify_net("VSS"), NetClassification::Ground);
    }

    #[test]
    fn test_hdmi_classification() {
        let classifier = NetClassifier::default();
        
        assert_eq!(classifier.classify_net("HDMI_D0+"), NetClassification::HighSpeed);
        assert_eq!(classifier.classify_net("TMDS_CLK"), NetClassification::HighSpeed);
    }

    #[test]
    fn test_ethernet_classification() {
        let classifier = NetClassifier::default();
        
        assert_eq!(classifier.classify_net("ETH_TX+"), NetClassification::HighSpeed);
        assert_eq!(classifier.classify_net("RGMII_TXD0"), NetClassification::HighSpeed);
    }

    #[test]
    fn test_custom_patterns() {
        let mut classifier = NetClassifier::default();
        classifier.add_high_speed_pattern("CUSTOM_HS");
        
        assert_eq!(classifier.classify_net("CUSTOM_HS_0"), NetClassification::HighSpeed);
    }
}
