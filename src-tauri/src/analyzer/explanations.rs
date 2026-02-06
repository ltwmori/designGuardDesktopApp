//! Issue Explanations System
//!
//! Provides detailed, educational explanations for design issues.
//! Transforms simple warnings into learning opportunities.

use serde::{Deserialize, Serialize};
use crate::analyzer::rules::{Issue, Severity};
use crate::parser::schema::Position;

/// A detailed issue with full educational content
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DetailedIssue {
    /// Basic issue info
    pub id: String,
    pub severity: Severity,
    pub rule_id: String,
    pub title: String,
    pub summary: String,
    
    /// Affected components
    pub components: Vec<String>,
    pub location: Option<Position>,
    
    /// The educational content
    pub explanation: IssueExplanation,
    
    /// User actions
    pub user_actions: UserActions,
}

/// Full explanation of an issue
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IssueExplanation {
    /// What is the problem? (1-2 sentences)
    pub what: String,
    
    /// Why does this matter? What could go wrong?
    pub why: WhySection,
    
    /// Technical background (expandable)
    pub technical_background: Option<TechnicalBackground>,
    
    /// How to fix it
    pub how_to_fix: HowToFix,
    
    /// Visual aids
    #[serde(default)]
    pub diagrams: Vec<Diagram>,
    
    /// Further reading
    #[serde(default)]
    pub references: Vec<Reference>,
}

/// Explanation of why an issue matters
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WhySection {
    /// Plain English explanation
    pub summary: String,
    
    /// What could go wrong if not fixed?
    #[serde(default)]
    pub consequences: Vec<Consequence>,
    
    /// Real-world failure examples
    #[serde(default)]
    pub failure_examples: Vec<String>,
}

/// A potential consequence of not fixing an issue
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Consequence {
    pub description: String,
    pub severity: ConsequenceSeverity,
    pub likelihood: Likelihood,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ConsequenceSeverity {
    Annoying,      // Intermittent issues, hard to debug
    Problematic,   // Reduced performance or reliability
    Serious,       // May not work in some conditions
    Critical,      // Will likely fail
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Likelihood {
    Rare,
    Occasional,
    Likely,
    Certain,
}

/// Technical background for those who want to learn more
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TechnicalBackground {
    /// The physics/EE concept involved
    pub concept: String,
    
    /// Detailed explanation (for those who want to learn)
    pub detailed_explanation: String,
    
    /// Relevant equations or calculations
    #[serde(default)]
    pub equations: Vec<Equation>,
    
    /// Related concepts
    #[serde(default)]
    pub related_concepts: Vec<String>,
}

/// An equation with explanation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Equation {
    pub name: String,
    pub formula: String,        // Plain text formula
    #[serde(default)]
    pub variables: Vec<Variable>,
    pub example_calculation: Option<String>,
}

/// A variable in an equation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Variable {
    pub symbol: String,
    pub name: String,
    pub unit: String,
    pub typical_value: Option<String>,
}

/// How to fix the issue
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HowToFix {
    /// Step-by-step fix instructions
    #[serde(default)]
    pub steps: Vec<FixStep>,
    
    /// Component suggestions
    #[serde(default)]
    pub component_suggestions: Vec<ComponentSuggestion>,
    
    /// Common mistakes to avoid
    #[serde(default)]
    pub pitfalls: Vec<String>,
    
    /// How to verify the fix worked
    pub verification: String,
}

/// A step in the fix process
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FixStep {
    pub step_number: u32,
    pub instruction: String,
    pub details: Option<String>,
    pub image: Option<String>,  // Path to illustration
}

/// A suggested component
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComponentSuggestion {
    pub component_type: String,
    pub value: String,
    pub footprint: String,
    pub notes: String,
    #[serde(default)]
    pub example_part_numbers: Vec<String>,
}

/// A diagram or visual aid
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Diagram {
    pub diagram_type: DiagramType,
    pub title: String,
    pub description: String,
    pub svg_content: Option<String>,  // Inline SVG
    pub image_path: Option<String>,   // Path to image file
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum DiagramType {
    SchematicSnippet,
    WaveformBefore,
    WaveformAfter,
    PlacementGuide,
    Comparison,
}

/// A reference for further reading
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Reference {
    pub title: String,
    pub url: Option<String>,
    pub reference_type: ReferenceType,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ReferenceType {
    Datasheet,
    ApplicationNote,
    Tutorial,
    Wikipedia,
    TextBook,
}

/// Actions available to the user
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserActions {
    /// Can this be auto-fixed?
    pub auto_fix_available: bool,
    
    /// Dismiss options
    pub can_dismiss: bool,
    
    #[serde(default)]
    pub dismiss_options: Vec<DismissOption>,
}

/// An option for dismissing an issue
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DismissOption {
    pub label: String,
    pub reason_code: String,
    pub requires_comment: bool,
}

impl Default for UserActions {
    fn default() -> Self {
        Self {
            auto_fix_available: false,
            can_dismiss: true,
            dismiss_options: vec![
                DismissOption {
                    label: "Not applicable to my design".to_string(),
                    reason_code: "not_applicable".to_string(),
                    requires_comment: false,
                },
                DismissOption {
                    label: "I'll fix this later".to_string(),
                    reason_code: "defer".to_string(),
                    requires_comment: false,
                },
                DismissOption {
                    label: "False positive".to_string(),
                    reason_code: "false_positive".to_string(),
                    requires_comment: true,
                },
            ],
        }
    }
}

/// Convert a basic Issue to a DetailedIssue with full explanation
impl From<Issue> for DetailedIssue {
    fn from(issue: Issue) -> Self {
        // Generate explanation based on rule_id
        let explanation = generate_explanation(&issue);
        
        DetailedIssue {
            id: issue.id.clone(),
            severity: issue.severity.clone(),
            rule_id: issue.rule_id.clone(),
            title: generate_title(&issue),
            summary: issue.message.clone(),
            components: issue.component.map(|c| vec![c]).unwrap_or_default(),
            location: issue.location,
            explanation,
            user_actions: UserActions::default(),
        }
    }
}

/// Generate a user-friendly title for an issue
fn generate_title(issue: &Issue) -> String {
    match issue.rule_id.as_str() {
        "decoupling_capacitor" => {
            if let Some(ref comp) = issue.component {
                format!("{} - Missing Decoupling Capacitor", comp)
            } else {
                "Missing Decoupling Capacitor".to_string()
            }
        }
        "i2c_pull_resistors" => "I2C Bus - Missing Pull-up Resistors".to_string(),
        "crystal_load_capacitors" => {
            if let Some(ref comp) = issue.component {
                format!("{} - Missing Crystal Load Capacitors", comp)
            } else {
                "Missing Crystal Load Capacitors".to_string()
            }
        }
        "power_pins" => "Power Connection Issue".to_string(),
        "esd_protection" => "Missing ESD Protection".to_string(),
        "bulk_capacitor" => {
            if let Some(ref comp) = issue.component {
                format!("{} - Missing Bulk Capacitor", comp)
            } else {
                "Missing Bulk Capacitor".to_string()
            }
        }
        "datasheet_decoupling_capacitor" => {
            if let Some(ref comp) = issue.component {
                format!("{} - Datasheet Decoupling Requirement", comp)
            } else {
                "Datasheet Decoupling Requirement".to_string()
            }
        }
        _ => issue.message.chars().take(50).collect::<String>() + "...",
    }
}

/// Generate a full explanation for an issue based on its rule_id
fn generate_explanation(issue: &Issue) -> IssueExplanation {
    match issue.rule_id.as_str() {
        "decoupling_capacitor" | "datasheet_decoupling_capacitor" => {
            create_decoupling_explanation(issue)
        }
        "i2c_pull_resistors" => create_i2c_pullup_explanation(issue),
        "crystal_load_capacitors" => create_crystal_explanation(issue),
        "power_pins" => create_power_pin_explanation(issue),
        "esd_protection" => create_esd_explanation(issue),
        "bulk_capacitor" => create_bulk_cap_explanation(issue),
        _ => create_generic_explanation(issue),
    }
}

/// Create explanation for decoupling capacitor issues
fn create_decoupling_explanation(issue: &Issue) -> IssueExplanation {
    IssueExplanation {
        what: format!(
            "{}. Decoupling capacitors provide local energy storage and filter high-frequency noise.",
            issue.message
        ),
        why: WhySection {
            summary: "Decoupling capacitors provide local energy storage for rapid current demands and filter high-frequency noise from the power supply.".to_string(),
            consequences: vec![
                Consequence {
                    description: "Logic glitches when multiple outputs switch simultaneously".to_string(),
                    severity: ConsequenceSeverity::Serious,
                    likelihood: Likelihood::Likely,
                },
                Consequence {
                    description: "Increased EMI/EMC emissions".to_string(),
                    severity: ConsequenceSeverity::Problematic,
                    likelihood: Likelihood::Certain,
                },
                Consequence {
                    description: "Random resets or lockups under heavy load".to_string(),
                    severity: ConsequenceSeverity::Critical,
                    likelihood: Likelihood::Occasional,
                },
                Consequence {
                    description: "Analog peripherals (ADC/DAC) produce noisy readings".to_string(),
                    severity: ConsequenceSeverity::Annoying,
                    likelihood: Likelihood::Likely,
                },
            ],
            failure_examples: vec![
                "A USB device that disconnects randomly when CPU load increases".to_string(),
                "An ADC that reads ±50 LSB noise on a stable DC input".to_string(),
                "A microcontroller that works on the bench but fails in the field".to_string(),
            ],
        },
        technical_background: Some(TechnicalBackground {
            concept: "Power Distribution Network (PDN) and Transient Response".to_string(),
            detailed_explanation: r#"When a digital IC switches outputs, it draws current pulses from the power supply. A CMOS output switching from low to high must charge the load capacitance, creating a current spike.

For example, switching a 20pF load in 2ns requires:
I = 20pF × (3.3V / 2ns) = 33mA

This current must come from somewhere. If the only source is the power supply through long PCB traces, the inductance creates a voltage drop:
V = L × dI/dt

A 10nH trace inductance with a 33mA/2ns current change produces:
V = 10nH × (33mA / 2ns) = 165mV

This 165mV drop can push VCC below minimum operating voltage and cause logic errors.

Decoupling capacitors solve this by providing a local energy reservoir. The cap supplies the transient current while the bulk supply catches up."#.to_string(),
            equations: vec![
                Equation {
                    name: "Current spike from capacitive load".to_string(),
                    formula: "I = C_load × (ΔV / Δt)".to_string(),
                    variables: vec![
                        Variable { symbol: "I".to_string(), name: "Peak current".to_string(), unit: "A".to_string(), typical_value: Some("10-100mA".to_string()) },
                        Variable { symbol: "C_load".to_string(), name: "Load capacitance".to_string(), unit: "F".to_string(), typical_value: Some("10-50pF".to_string()) },
                    ],
                    example_calculation: Some("20pF × (3.3V / 2ns) = 33mA".to_string()),
                },
                Equation {
                    name: "Voltage drop from trace inductance".to_string(),
                    formula: "V_drop = L × (dI / dt)".to_string(),
                    variables: vec![
                        Variable { symbol: "L".to_string(), name: "Trace inductance".to_string(), unit: "H".to_string(), typical_value: Some("~1nH/mm".to_string()) },
                    ],
                    example_calculation: Some("10nH × (33mA / 2ns) = 165mV drop".to_string()),
                },
            ],
            related_concepts: vec![
                "Impedance vs Frequency".to_string(),
                "Via inductance".to_string(),
                "Ground bounce".to_string(),
            ],
        }),
        how_to_fix: HowToFix {
            steps: vec![
                FixStep {
                    step_number: 1,
                    instruction: "Add a 100nF ceramic capacitor".to_string(),
                    details: Some("Use 0402 or 0603 package, X7R or X5R dielectric, 16V or higher rating".to_string()),
                    image: None,
                },
                FixStep {
                    step_number: 2,
                    instruction: "Place within 3-5mm of the IC's VDD pin".to_string(),
                    details: Some("Shorter is better. The capacitor should be between the power source and the IC.".to_string()),
                    image: None,
                },
                FixStep {
                    step_number: 3,
                    instruction: "Connect with short, wide traces".to_string(),
                    details: Some("Use 0.3mm+ trace width. Route directly from cap pads to VDD/GND pins.".to_string()),
                    image: None,
                },
                FixStep {
                    step_number: 4,
                    instruction: "Place ground via at capacitor pad".to_string(),
                    details: Some("Via should be directly at the capacitor's GND pad, not further away.".to_string()),
                    image: None,
                },
            ],
            component_suggestions: vec![
                ComponentSuggestion {
                    component_type: "Ceramic Capacitor".to_string(),
                    value: "100nF".to_string(),
                    footprint: "0402 (1005 Metric)".to_string(),
                    notes: "X7R or X5R dielectric, 16V rating minimum".to_string(),
                    example_part_numbers: vec![
                        "GRM155R71H104KE14D (Murata)".to_string(),
                        "CL05B104KO5NNNC (Samsung)".to_string(),
                    ],
                },
            ],
            pitfalls: vec![
                "Don't use Y5V dielectric - loses 80% capacitance at extremes".to_string(),
                "Don't route ground through long trace to distant via".to_string(),
                "Don't share one cap between multiple ICs".to_string(),
            ],
            verification: "VDD ripple should be < 100mV peak-to-peak under maximum switching load.".to_string(),
        },
        diagrams: vec![],
        references: vec![
            Reference {
                title: "TI: Decoupling Capacitor Placement".to_string(),
                url: Some("https://www.ti.com/lit/an/slva157/slva157.pdf".to_string()),
                reference_type: ReferenceType::ApplicationNote,
            },
            Reference {
                title: "Why Decoupling Capacitors are Important".to_string(),
                url: Some("https://www.allaboutcircuits.com/technical-articles/why-decoupling-capacitors-are-important/".to_string()),
                reference_type: ReferenceType::Tutorial,
            },
        ],
    }
}

/// Create explanation for I2C pull-up resistor issues
fn create_i2c_pullup_explanation(issue: &Issue) -> IssueExplanation {
    IssueExplanation {
        what: "I2C bus detected but no pull-up resistors found. I2C is an open-drain bus that requires external pull-ups to function.".to_string(),
        why: WhySection {
            summary: "I2C uses open-drain outputs that can only pull the line LOW. Pull-up resistors are required to return the line to HIGH state.".to_string(),
            consequences: vec![
                Consequence {
                    description: "Bus will not work at all - lines stuck LOW".to_string(),
                    severity: ConsequenceSeverity::Critical,
                    likelihood: Likelihood::Certain,
                },
                Consequence {
                    description: "Communication errors and data corruption".to_string(),
                    severity: ConsequenceSeverity::Serious,
                    likelihood: Likelihood::Likely,
                },
            ],
            failure_examples: vec![
                "I2C devices not detected during bus scan".to_string(),
                "Intermittent communication failures at higher speeds".to_string(),
            ],
        },
        technical_background: Some(TechnicalBackground {
            concept: "Open-Drain Bus Architecture".to_string(),
            detailed_explanation: "I2C uses open-drain (or open-collector) outputs. Each device can only pull the line LOW by connecting it to ground. To return to HIGH, an external pull-up resistor is required.\n\nThe pull-up value is a trade-off:\n- Too high: slow rise time, limits bus speed\n- Too low: excessive current when driving LOW, may exceed sink capability".to_string(),
            equations: vec![
                Equation {
                    name: "Rise time calculation".to_string(),
                    formula: "t_rise = 0.8473 × R_pullup × C_bus".to_string(),
                    variables: vec![
                        Variable { symbol: "R_pullup".to_string(), name: "Pull-up resistance".to_string(), unit: "Ω".to_string(), typical_value: Some("2.2k-10k".to_string()) },
                        Variable { symbol: "C_bus".to_string(), name: "Bus capacitance".to_string(), unit: "F".to_string(), typical_value: Some("50-400pF".to_string()) },
                    ],
                    example_calculation: Some("4.7kΩ × 100pF = 0.47µs rise time".to_string()),
                },
            ],
            related_concepts: vec![
                "Open-drain vs push-pull".to_string(),
                "Bus capacitance".to_string(),
                "I2C speed modes".to_string(),
            ],
        }),
        how_to_fix: HowToFix {
            steps: vec![
                FixStep {
                    step_number: 1,
                    instruction: "Add pull-up resistors to SDA and SCL".to_string(),
                    details: Some("Typical value is 4.7kΩ for 100kHz/400kHz. Use 2.2kΩ for faster speeds or longer buses.".to_string()),
                    image: None,
                },
                FixStep {
                    step_number: 2,
                    instruction: "Connect resistors to VCC (usually 3.3V)".to_string(),
                    details: Some("Use the same voltage as your I2C devices' logic level.".to_string()),
                    image: None,
                },
            ],
            component_suggestions: vec![
                ComponentSuggestion {
                    component_type: "Resistor".to_string(),
                    value: "4.7kΩ".to_string(),
                    footprint: "0402 or 0603".to_string(),
                    notes: "1% tolerance recommended for consistency".to_string(),
                    example_part_numbers: vec![
                        "RC0402FR-074K7L (Yageo)".to_string(),
                    ],
                },
            ],
            pitfalls: vec![
                "Don't add pull-ups if they already exist on a module".to_string(),
                "Don't use values below 1kΩ - may exceed device sink current".to_string(),
            ],
            verification: "Use oscilloscope to verify clean square waves with proper rise times on SDA/SCL.".to_string(),
        },
        diagrams: vec![],
        references: vec![
            Reference {
                title: "NXP I2C Manual".to_string(),
                url: Some("https://www.nxp.com/docs/en/user-guide/UM10204.pdf".to_string()),
                reference_type: ReferenceType::ApplicationNote,
            },
        ],
    }
}

/// Create explanation for crystal load capacitor issues
fn create_crystal_explanation(issue: &Issue) -> IssueExplanation {
    IssueExplanation {
        what: format!("{}. Crystals require load capacitors to oscillate at the correct frequency.", issue.message),
        why: WhySection {
            summary: "Load capacitors are part of the oscillator circuit and determine the crystal's operating frequency. Wrong or missing caps cause frequency errors or failure to oscillate.".to_string(),
            consequences: vec![
                Consequence {
                    description: "Crystal may not start oscillating".to_string(),
                    severity: ConsequenceSeverity::Critical,
                    likelihood: Likelihood::Occasional,
                },
                Consequence {
                    description: "Frequency error causing timing/communication issues".to_string(),
                    severity: ConsequenceSeverity::Serious,
                    likelihood: Likelihood::Likely,
                },
                Consequence {
                    description: "USB communication failures due to clock drift".to_string(),
                    severity: ConsequenceSeverity::Serious,
                    likelihood: Likelihood::Likely,
                },
            ],
            failure_examples: vec![
                "USB device not recognized due to clock frequency error".to_string(),
                "UART communication garbled at higher baud rates".to_string(),
            ],
        },
        technical_background: Some(TechnicalBackground {
            concept: "Crystal Oscillator Load Capacitance".to_string(),
            detailed_explanation: "A crystal specifies a 'load capacitance' (CL) that it needs to see to oscillate at its rated frequency. The load capacitors and stray capacitance must combine to equal CL.\n\nFormula: CL = (C1 × C2) / (C1 + C2) + Cstray\n\nFor equal capacitors: CL = C/2 + Cstray\n\nStray capacitance is typically 2-5pF from PCB traces and IC pins.".to_string(),
            equations: vec![
                Equation {
                    name: "Load capacitance calculation".to_string(),
                    formula: "CL = (C1 × C2) / (C1 + C2) + C_stray".to_string(),
                    variables: vec![
                        Variable { symbol: "CL".to_string(), name: "Crystal load capacitance".to_string(), unit: "pF".to_string(), typical_value: Some("12-20pF".to_string()) },
                        Variable { symbol: "C_stray".to_string(), name: "Stray capacitance".to_string(), unit: "pF".to_string(), typical_value: Some("2-5pF".to_string()) },
                    ],
                    example_calculation: Some("For CL=20pF, Cstray=3pF: C1=C2=34pF".to_string()),
                },
            ],
            related_concepts: vec![
                "Pierce oscillator".to_string(),
                "Crystal ESR".to_string(),
                "Negative resistance".to_string(),
            ],
        }),
        how_to_fix: HowToFix {
            steps: vec![
                FixStep {
                    step_number: 1,
                    instruction: "Check crystal datasheet for load capacitance (CL)".to_string(),
                    details: Some("Common values are 12pF, 18pF, or 20pF.".to_string()),
                    image: None,
                },
                FixStep {
                    step_number: 2,
                    instruction: "Calculate required capacitor values".to_string(),
                    details: Some("C = 2 × (CL - Cstray). For CL=20pF, Cstray=3pF: C=34pF, use 33pF.".to_string()),
                    image: None,
                },
                FixStep {
                    step_number: 3,
                    instruction: "Add capacitors from each crystal pin to GND".to_string(),
                    details: Some("Place as close to crystal as possible. Use NP0/C0G ceramic caps.".to_string()),
                    image: None,
                },
            ],
            component_suggestions: vec![
                ComponentSuggestion {
                    component_type: "Ceramic Capacitor".to_string(),
                    value: "22pF".to_string(),
                    footprint: "0402".to_string(),
                    notes: "NP0/C0G dielectric for stability. Common starting value.".to_string(),
                    example_part_numbers: vec![
                        "GRM1555C1H220JA01D (Murata)".to_string(),
                    ],
                },
            ],
            pitfalls: vec![
                "Don't use X7R capacitors - temperature coefficient too high".to_string(),
                "Don't route high-speed signals near crystal traces".to_string(),
            ],
            verification: "Measure frequency with counter or check USB/UART communication works reliably.".to_string(),
        },
        diagrams: vec![],
        references: vec![
            Reference {
                title: "Crystal Oscillator Design Guide".to_string(),
                url: Some("https://www.st.com/resource/en/application_note/an2867-oscillator-design-guide-for-stm8afals-stm32-mcus-and-mpus-stmicroelectronics.pdf".to_string()),
                reference_type: ReferenceType::ApplicationNote,
            },
        ],
    }
}

/// Create explanation for power pin issues
fn create_power_pin_explanation(issue: &Issue) -> IssueExplanation {
    IssueExplanation {
        what: issue.message.clone(),
        why: WhySection {
            summary: "Every circuit needs proper power and ground connections. Missing or improper connections will prevent the circuit from functioning.".to_string(),
            consequences: vec![
                Consequence {
                    description: "Circuit will not power on".to_string(),
                    severity: ConsequenceSeverity::Critical,
                    likelihood: Likelihood::Certain,
                },
            ],
            failure_examples: vec![
                "IC appears dead because VDD not connected".to_string(),
            ],
        },
        technical_background: None,
        how_to_fix: HowToFix {
            steps: vec![
                FixStep {
                    step_number: 1,
                    instruction: "Add power symbols (VCC/VDD and GND) to schematic".to_string(),
                    details: Some("Use appropriate symbols from the power library.".to_string()),
                    image: None,
                },
                FixStep {
                    step_number: 2,
                    instruction: "Connect all IC power pins".to_string(),
                    details: Some("Every VDD/VCC pin needs connection, even if they're internally connected.".to_string()),
                    image: None,
                },
            ],
            component_suggestions: vec![],
            pitfalls: vec![
                "Don't assume internally connected pins don't need external connection".to_string(),
            ],
            verification: "Run ERC in KiCAD to verify all power pins are connected.".to_string(),
        },
        diagrams: vec![],
        references: vec![],
    }
}

/// Create explanation for ESD protection issues
fn create_esd_explanation(issue: &Issue) -> IssueExplanation {
    IssueExplanation {
        what: issue.message.clone(),
        why: WhySection {
            summary: "External interfaces (USB, Ethernet) are exposed to ESD events from users and the environment. Without protection, these events can damage sensitive ICs.".to_string(),
            consequences: vec![
                Consequence {
                    description: "IC damage from static discharge".to_string(),
                    severity: ConsequenceSeverity::Critical,
                    likelihood: Likelihood::Occasional,
                },
                Consequence {
                    description: "Intermittent failures after ESD event".to_string(),
                    severity: ConsequenceSeverity::Serious,
                    likelihood: Likelihood::Likely,
                },
            ],
            failure_examples: vec![
                "USB port stops working after user touches connector".to_string(),
                "Device fails EMC testing".to_string(),
            ],
        },
        technical_background: Some(TechnicalBackground {
            concept: "ESD Protection with TVS Diodes".to_string(),
            detailed_explanation: "TVS (Transient Voltage Suppressor) diodes clamp voltage spikes by conducting when voltage exceeds a threshold. They shunt the ESD energy to ground, protecting downstream components.".to_string(),
            equations: vec![],
            related_concepts: vec![
                "Human Body Model (HBM)".to_string(),
                "IEC 61000-4-2".to_string(),
            ],
        }),
        how_to_fix: HowToFix {
            steps: vec![
                FixStep {
                    step_number: 1,
                    instruction: "Add TVS diode array near connector".to_string(),
                    details: Some("Place as close to connector as possible, before any other components.".to_string()),
                    image: None,
                },
                FixStep {
                    step_number: 2,
                    instruction: "Select appropriate clamping voltage".to_string(),
                    details: Some("For 3.3V signals, use TVS with ~5V clamping. For USB, use USB-specific TVS.".to_string()),
                    image: None,
                },
            ],
            component_suggestions: vec![
                ComponentSuggestion {
                    component_type: "TVS Diode Array".to_string(),
                    value: "USBLC6-2SC6".to_string(),
                    footprint: "SOT-23-6".to_string(),
                    notes: "Common choice for USB ESD protection".to_string(),
                    example_part_numbers: vec![
                        "USBLC6-2SC6 (STMicroelectronics)".to_string(),
                        "TPD2E001 (TI)".to_string(),
                    ],
                },
            ],
            pitfalls: vec![
                "Don't place TVS after the IC - it won't protect it".to_string(),
                "Don't forget to connect TVS ground to chassis ground".to_string(),
            ],
            verification: "Test with ESD gun per IEC 61000-4-2 standard.".to_string(),
        },
        diagrams: vec![],
        references: vec![
            Reference {
                title: "TI: System-Level ESD Protection Guide".to_string(),
                url: Some("https://www.ti.com/lit/sg/sszb130c/sszb130c.pdf".to_string()),
                reference_type: ReferenceType::ApplicationNote,
            },
        ],
    }
}

/// Create explanation for bulk capacitor issues
fn create_bulk_cap_explanation(issue: &Issue) -> IssueExplanation {
    IssueExplanation {
        what: format!("{}. Bulk capacitors provide energy storage for load transients.", issue.message),
        why: WhySection {
            summary: "Voltage regulators need bulk capacitors for stability and to handle load transients. Without them, output voltage may oscillate or droop under load.".to_string(),
            consequences: vec![
                Consequence {
                    description: "Regulator oscillation causing noise".to_string(),
                    severity: ConsequenceSeverity::Serious,
                    likelihood: Likelihood::Likely,
                },
                Consequence {
                    description: "Voltage droop during load transients".to_string(),
                    severity: ConsequenceSeverity::Problematic,
                    likelihood: Likelihood::Likely,
                },
                Consequence {
                    description: "Regulator may not start up properly".to_string(),
                    severity: ConsequenceSeverity::Critical,
                    likelihood: Likelihood::Occasional,
                },
            ],
            failure_examples: vec![
                "3.3V rail shows 100mV ripple at switching frequency".to_string(),
                "System resets when motor starts".to_string(),
            ],
        },
        technical_background: Some(TechnicalBackground {
            concept: "LDO Stability and Output Capacitance".to_string(),
            detailed_explanation: "LDO regulators use a feedback loop that can become unstable without proper output capacitance. The capacitor's ESR (Equivalent Series Resistance) is often critical - some LDOs require specific ESR ranges for stability.".to_string(),
            equations: vec![],
            related_concepts: vec![
                "ESR and stability".to_string(),
                "Transient response".to_string(),
                "Ceramic vs tantalum capacitors".to_string(),
            ],
        }),
        how_to_fix: HowToFix {
            steps: vec![
                FixStep {
                    step_number: 1,
                    instruction: "Check regulator datasheet for capacitor requirements".to_string(),
                    details: Some("Note minimum capacitance AND ESR requirements.".to_string()),
                    image: None,
                },
                FixStep {
                    step_number: 2,
                    instruction: "Add output capacitor meeting specifications".to_string(),
                    details: Some("Typical: 10-22µF ceramic or tantalum.".to_string()),
                    image: None,
                },
                FixStep {
                    step_number: 3,
                    instruction: "Add input capacitor if far from source".to_string(),
                    details: Some("10µF typical for input filtering.".to_string()),
                    image: None,
                },
            ],
            component_suggestions: vec![
                ComponentSuggestion {
                    component_type: "Ceramic Capacitor".to_string(),
                    value: "22µF".to_string(),
                    footprint: "0805 or 1206".to_string(),
                    notes: "X5R or X7R, voltage rating 2x output voltage minimum".to_string(),
                    example_part_numbers: vec![
                        "GRM21BR61E226ME44L (Murata)".to_string(),
                    ],
                },
            ],
            pitfalls: vec![
                "Some LDOs need ESR in specific range - pure ceramic may cause oscillation".to_string(),
                "Ceramic capacitors lose capacitance with DC bias - derate accordingly".to_string(),
            ],
            verification: "Check output with oscilloscope for ripple < 50mV and stable DC level.".to_string(),
        },
        diagrams: vec![],
        references: vec![
            Reference {
                title: "TI: LDO Basics".to_string(),
                url: Some("https://www.ti.com/lit/an/slva079/slva079.pdf".to_string()),
                reference_type: ReferenceType::ApplicationNote,
            },
        ],
    }
}

/// Create a generic explanation for unknown rule types
fn create_generic_explanation(issue: &Issue) -> IssueExplanation {
    IssueExplanation {
        what: issue.message.clone(),
        why: WhySection {
            summary: "This issue was detected by the design rule checker. Review the details and fix if applicable to your design.".to_string(),
            consequences: vec![],
            failure_examples: vec![],
        },
        technical_background: None,
        how_to_fix: HowToFix {
            steps: vec![
                FixStep {
                    step_number: 1,
                    instruction: issue.suggestion.clone().unwrap_or_else(|| "Review and address the issue".to_string()),
                    details: None,
                    image: None,
                },
            ],
            component_suggestions: vec![],
            pitfalls: vec![],
            verification: "Verify the issue is resolved by re-running analysis.".to_string(),
        },
        diagrams: vec![],
        references: vec![],
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_issue_to_detailed() {
        let issue = Issue {
            risk_score: None,
            id: "test-1".to_string(),
            rule_id: "decoupling_capacitor".to_string(),
            severity: Severity::Warning,
            message: "U1 missing decoupling capacitor".to_string(),
            component: Some("U1".to_string()),
            location: None,
            suggestion: Some("Add 100nF capacitor".to_string()),
        };
        
        let detailed: DetailedIssue = issue.into();
        
        assert_eq!(detailed.rule_id, "decoupling_capacitor");
        assert!(!detailed.explanation.why.consequences.is_empty());
        assert!(!detailed.explanation.how_to_fix.steps.is_empty());
    }
    
    #[test]
    fn test_decoupling_explanation() {
        let issue = Issue {
            risk_score: None,
            id: "test-1".to_string(),
            rule_id: "decoupling_capacitor".to_string(),
            severity: Severity::Warning,
            message: "Test message".to_string(),
            component: Some("U1".to_string()),
            location: None,
            suggestion: None,
        };
        
        let explanation = create_decoupling_explanation(&issue);
        
        assert!(explanation.technical_background.is_some());
        assert!(!explanation.references.is_empty());
    }
}
