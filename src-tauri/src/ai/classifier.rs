//! Component Role Classifier
//!
//! Uses a Small Language Model (Phi-3 via Ollama) to classify component roles
//! in a netlist based on Part Number and Reference Designator.
//!
//! Input: Part Number + Reference Designator
//! Output: Role (e.g., Buck_Regulator, MCU_GPIO, I2C_Slave)

use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::Duration;

use crate::ai::AIError;

const DEFAULT_OLLAMA_URL: &str = "http://localhost:11434";
const DEFAULT_MODEL: &str = "phi3";
const CLASSIFIER_TIMEOUT_SECS: u64 = 60;

/// Component role categories for circuit analysis
/// 
/// Note: We use underscores in variant names intentionally to match
/// the domain terminology (e.g., MCU_GPIO, I2C_Slave) for better
/// readability and consistency with electronics conventions.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[allow(non_camel_case_types)]
pub enum ComponentRole {
    // Power Management
    BuckRegulator,
    BoostRegulator,
    BuckBoostRegulator,
    LDORegulator,
    SwitchingRegulator,
    PowerMOSFET,
    PowerDiode,
    PowerInductor,
    BulkCapacitor,
    DecouplingCapacitor,
    FilterCapacitor,
    
    // Microcontrollers & Processors
    MCU,
    MCU_GPIO,
    MCU_ADC,
    MCU_PWM,
    MCU_Timer,
    MCU_UART,
    MCU_SPI,
    MCU_I2C,
    DSP,
    FPGA,
    CPLD,
    
    // Communication Interfaces
    I2C_Master,
    I2C_Slave,
    SPI_Master,
    SPI_Slave,
    UART_Transceiver,
    RS232_Driver,
    RS485_Transceiver,
    CAN_Transceiver,
    USB_Controller,
    USB_PHY,
    Ethernet_PHY,
    WiFi_Module,
    Bluetooth_Module,
    LoRa_Module,
    
    // Analog
    OpAmp,
    Comparator,
    ADC,
    DAC,
    VoltageReference,
    CurrentSense,
    Instrumentation_Amp,
    PGA,
    
    // Timing & Oscillators
    Crystal,
    Oscillator,
    PLL,
    ClockBuffer,
    TimerIC,
    RTC,
    
    // Sensors
    TemperatureSensor,
    PressureSensor,
    AccelerometerGyro,
    Magnetometer,
    LightSensor,
    ProximitySensor,
    CurrentSensor,
    VoltageSensor,
    
    // Memory
    EEPROM,
    Flash,
    SRAM,
    DRAM,
    FRAM,
    
    // Protection
    TVS_Diode,
    Fuse,
    PolyFuse,
    ESD_Protection,
    OvervoltageProtection,
    ReversePolarity,
    
    // Passive Components
    PullUpResistor,
    PullDownResistor,
    CurrentLimitResistor,
    VoltageDivider,
    FilterResistor,
    TerminationResistor,
    FeedbackResistor,
    
    // Connectors & Interface
    PowerConnector,
    SignalConnector,
    DebugConnector,
    ProgrammingHeader,
    TestPoint,
    
    // Display & Indicators
    LED_Indicator,
    LED_Driver,
    LCD_Display,
    OLED_Display,
    SevenSegment,
    
    // Switching & Control
    LoadSwitch,
    AnalogSwitch,
    Multiplexer,
    Relay,
    RelayDriver,
    MotorDriver,
    GateDriver,
    
    // RF & Wireless
    RF_Amplifier,
    RF_Filter,
    Antenna,
    Balun,
    RF_Switch,
    
    // Audio
    AudioCodec,
    AudioAmplifier,
    Microphone,
    Speaker,
    
    // Isolation
    Optocoupler,
    DigitalIsolator,
    IsolatedDCDC,
    
    // Unknown/Generic
    GenericIC,
    GenericPassive,
    Unknown,
}

impl ComponentRole {
    /// Get a human-readable description of the role
    pub fn description(&self) -> &'static str {
        match self {
            ComponentRole::BuckRegulator => "Step-down DC-DC converter",
            ComponentRole::BoostRegulator => "Step-up DC-DC converter",
            ComponentRole::BuckBoostRegulator => "Step-up/down DC-DC converter",
            ComponentRole::LDORegulator => "Low dropout linear regulator",
            ComponentRole::SwitchingRegulator => "Switching power supply controller",
            ComponentRole::PowerMOSFET => "Power switching MOSFET",
            ComponentRole::PowerDiode => "Power rectifier or Schottky diode",
            ComponentRole::PowerInductor => "Energy storage inductor for switching supply",
            ComponentRole::BulkCapacitor => "Bulk energy storage capacitor",
            ComponentRole::DecouplingCapacitor => "High-frequency noise filtering capacitor",
            ComponentRole::FilterCapacitor => "Signal filtering capacitor",
            
            ComponentRole::MCU => "Microcontroller unit",
            ComponentRole::MCU_GPIO => "MCU general purpose I/O",
            ComponentRole::MCU_ADC => "MCU analog-to-digital converter",
            ComponentRole::MCU_PWM => "MCU pulse width modulation output",
            ComponentRole::MCU_Timer => "MCU timer/counter peripheral",
            ComponentRole::MCU_UART => "MCU serial communication",
            ComponentRole::MCU_SPI => "MCU SPI interface",
            ComponentRole::MCU_I2C => "MCU I2C interface",
            ComponentRole::DSP => "Digital signal processor",
            ComponentRole::FPGA => "Field programmable gate array",
            ComponentRole::CPLD => "Complex programmable logic device",
            
            ComponentRole::I2C_Master => "I2C bus master device",
            ComponentRole::I2C_Slave => "I2C bus slave device",
            ComponentRole::SPI_Master => "SPI bus master device",
            ComponentRole::SPI_Slave => "SPI bus slave device",
            ComponentRole::UART_Transceiver => "UART level shifter/transceiver",
            ComponentRole::RS232_Driver => "RS-232 line driver",
            ComponentRole::RS485_Transceiver => "RS-485 differential transceiver",
            ComponentRole::CAN_Transceiver => "CAN bus transceiver",
            ComponentRole::USB_Controller => "USB protocol controller",
            ComponentRole::USB_PHY => "USB physical layer transceiver",
            ComponentRole::Ethernet_PHY => "Ethernet physical layer",
            ComponentRole::WiFi_Module => "WiFi wireless module",
            ComponentRole::Bluetooth_Module => "Bluetooth wireless module",
            ComponentRole::LoRa_Module => "LoRa long-range wireless module",
            
            ComponentRole::OpAmp => "Operational amplifier",
            ComponentRole::Comparator => "Voltage comparator",
            ComponentRole::ADC => "Analog-to-digital converter IC",
            ComponentRole::DAC => "Digital-to-analog converter IC",
            ComponentRole::VoltageReference => "Precision voltage reference",
            ComponentRole::CurrentSense => "Current sensing circuit",
            ComponentRole::Instrumentation_Amp => "Instrumentation amplifier",
            ComponentRole::PGA => "Programmable gain amplifier",
            
            ComponentRole::Crystal => "Quartz crystal oscillator element",
            ComponentRole::Oscillator => "Clock oscillator module",
            ComponentRole::PLL => "Phase-locked loop",
            ComponentRole::ClockBuffer => "Clock distribution buffer",
            ComponentRole::TimerIC => "Timer/counter IC (e.g., 555)",
            ComponentRole::RTC => "Real-time clock",
            
            ComponentRole::TemperatureSensor => "Temperature measurement sensor",
            ComponentRole::PressureSensor => "Pressure measurement sensor",
            ComponentRole::AccelerometerGyro => "Motion/orientation sensor",
            ComponentRole::Magnetometer => "Magnetic field sensor",
            ComponentRole::LightSensor => "Ambient light sensor",
            ComponentRole::ProximitySensor => "Proximity detection sensor",
            ComponentRole::CurrentSensor => "Current measurement sensor",
            ComponentRole::VoltageSensor => "Voltage measurement sensor",
            
            ComponentRole::EEPROM => "Electrically erasable programmable ROM",
            ComponentRole::Flash => "Flash memory",
            ComponentRole::SRAM => "Static RAM",
            ComponentRole::DRAM => "Dynamic RAM",
            ComponentRole::FRAM => "Ferroelectric RAM",
            
            ComponentRole::TVS_Diode => "Transient voltage suppressor",
            ComponentRole::Fuse => "Overcurrent protection fuse",
            ComponentRole::PolyFuse => "Resettable polyfuse",
            ComponentRole::ESD_Protection => "ESD protection device",
            ComponentRole::OvervoltageProtection => "Overvoltage protection circuit",
            ComponentRole::ReversePolarity => "Reverse polarity protection",
            
            ComponentRole::PullUpResistor => "Logic high pull-up resistor",
            ComponentRole::PullDownResistor => "Logic low pull-down resistor",
            ComponentRole::CurrentLimitResistor => "Current limiting resistor",
            ComponentRole::VoltageDivider => "Voltage divider resistor network",
            ComponentRole::FilterResistor => "RC filter resistor",
            ComponentRole::TerminationResistor => "Signal termination resistor",
            ComponentRole::FeedbackResistor => "Feedback network resistor",
            
            ComponentRole::PowerConnector => "Power input/output connector",
            ComponentRole::SignalConnector => "Signal interface connector",
            ComponentRole::DebugConnector => "Debug/JTAG connector",
            ComponentRole::ProgrammingHeader => "Programming interface header",
            ComponentRole::TestPoint => "Test point for measurement",
            
            ComponentRole::LED_Indicator => "Status indicator LED",
            ComponentRole::LED_Driver => "LED driver IC",
            ComponentRole::LCD_Display => "LCD display module",
            ComponentRole::OLED_Display => "OLED display module",
            ComponentRole::SevenSegment => "7-segment display",
            
            ComponentRole::LoadSwitch => "High-side/low-side load switch",
            ComponentRole::AnalogSwitch => "Analog signal switch",
            ComponentRole::Multiplexer => "Signal multiplexer/demultiplexer",
            ComponentRole::Relay => "Electromechanical relay",
            ComponentRole::RelayDriver => "Relay driver circuit",
            ComponentRole::MotorDriver => "Motor driver IC",
            ComponentRole::GateDriver => "MOSFET/IGBT gate driver",
            
            ComponentRole::RF_Amplifier => "RF signal amplifier",
            ComponentRole::RF_Filter => "RF bandpass/lowpass filter",
            ComponentRole::Antenna => "RF antenna",
            ComponentRole::Balun => "Balanced-unbalanced transformer",
            ComponentRole::RF_Switch => "RF signal switch",
            
            ComponentRole::AudioCodec => "Audio codec IC",
            ComponentRole::AudioAmplifier => "Audio power amplifier",
            ComponentRole::Microphone => "Microphone element",
            ComponentRole::Speaker => "Speaker/buzzer element",
            
            ComponentRole::Optocoupler => "Optically isolated coupler",
            ComponentRole::DigitalIsolator => "Digital signal isolator",
            ComponentRole::IsolatedDCDC => "Isolated DC-DC converter",
            
            ComponentRole::GenericIC => "Generic integrated circuit",
            ComponentRole::GenericPassive => "Generic passive component",
            ComponentRole::Unknown => "Unknown component role",
        }
    }
    
    /// Get the category of this role
    pub fn category(&self) -> &'static str {
        match self {
            ComponentRole::BuckRegulator | ComponentRole::BoostRegulator |
            ComponentRole::BuckBoostRegulator | ComponentRole::LDORegulator |
            ComponentRole::SwitchingRegulator | ComponentRole::PowerMOSFET |
            ComponentRole::PowerDiode | ComponentRole::PowerInductor |
            ComponentRole::BulkCapacitor | ComponentRole::DecouplingCapacitor |
            ComponentRole::FilterCapacitor => "Power Management",
            
            ComponentRole::MCU | ComponentRole::MCU_GPIO | ComponentRole::MCU_ADC |
            ComponentRole::MCU_PWM | ComponentRole::MCU_Timer | ComponentRole::MCU_UART |
            ComponentRole::MCU_SPI | ComponentRole::MCU_I2C | ComponentRole::DSP |
            ComponentRole::FPGA | ComponentRole::CPLD => "Microcontrollers & Processors",
            
            ComponentRole::I2C_Master | ComponentRole::I2C_Slave |
            ComponentRole::SPI_Master | ComponentRole::SPI_Slave |
            ComponentRole::UART_Transceiver | ComponentRole::RS232_Driver |
            ComponentRole::RS485_Transceiver | ComponentRole::CAN_Transceiver |
            ComponentRole::USB_Controller | ComponentRole::USB_PHY |
            ComponentRole::Ethernet_PHY | ComponentRole::WiFi_Module |
            ComponentRole::Bluetooth_Module | ComponentRole::LoRa_Module => "Communication",
            
            ComponentRole::OpAmp | ComponentRole::Comparator | ComponentRole::ADC |
            ComponentRole::DAC | ComponentRole::VoltageReference |
            ComponentRole::CurrentSense | ComponentRole::Instrumentation_Amp |
            ComponentRole::PGA => "Analog",
            
            ComponentRole::Crystal | ComponentRole::Oscillator | ComponentRole::PLL |
            ComponentRole::ClockBuffer | ComponentRole::TimerIC |
            ComponentRole::RTC => "Timing & Oscillators",
            
            ComponentRole::TemperatureSensor | ComponentRole::PressureSensor |
            ComponentRole::AccelerometerGyro | ComponentRole::Magnetometer |
            ComponentRole::LightSensor | ComponentRole::ProximitySensor |
            ComponentRole::CurrentSensor | ComponentRole::VoltageSensor => "Sensors",
            
            ComponentRole::EEPROM | ComponentRole::Flash | ComponentRole::SRAM |
            ComponentRole::DRAM | ComponentRole::FRAM => "Memory",
            
            ComponentRole::TVS_Diode | ComponentRole::Fuse | ComponentRole::PolyFuse |
            ComponentRole::ESD_Protection | ComponentRole::OvervoltageProtection |
            ComponentRole::ReversePolarity => "Protection",
            
            ComponentRole::PullUpResistor | ComponentRole::PullDownResistor |
            ComponentRole::CurrentLimitResistor | ComponentRole::VoltageDivider |
            ComponentRole::FilterResistor | ComponentRole::TerminationResistor |
            ComponentRole::FeedbackResistor => "Passive Components",
            
            ComponentRole::PowerConnector | ComponentRole::SignalConnector |
            ComponentRole::DebugConnector | ComponentRole::ProgrammingHeader |
            ComponentRole::TestPoint => "Connectors & Interface",
            
            ComponentRole::LED_Indicator | ComponentRole::LED_Driver |
            ComponentRole::LCD_Display | ComponentRole::OLED_Display |
            ComponentRole::SevenSegment => "Display & Indicators",
            
            ComponentRole::LoadSwitch | ComponentRole::AnalogSwitch |
            ComponentRole::Multiplexer | ComponentRole::Relay |
            ComponentRole::RelayDriver | ComponentRole::MotorDriver |
            ComponentRole::GateDriver => "Switching & Control",
            
            ComponentRole::RF_Amplifier | ComponentRole::RF_Filter |
            ComponentRole::Antenna | ComponentRole::Balun |
            ComponentRole::RF_Switch => "RF & Wireless",
            
            ComponentRole::AudioCodec | ComponentRole::AudioAmplifier |
            ComponentRole::Microphone | ComponentRole::Speaker => "Audio",
            
            ComponentRole::Optocoupler | ComponentRole::DigitalIsolator |
            ComponentRole::IsolatedDCDC => "Isolation",
            
            ComponentRole::GenericIC | ComponentRole::GenericPassive |
            ComponentRole::Unknown => "Unknown",
        }
    }
}

impl std::fmt::Display for ComponentRole {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // Convert enum variant to snake_case string
        let s = format!("{:?}", self);
        write!(f, "{}", s)
    }
}

impl Default for ComponentRole {
    fn default() -> Self {
        ComponentRole::Unknown
    }
}

/// Input for component classification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComponentInput {
    /// Reference designator (e.g., "U1", "R1", "C10")
    pub ref_des: String,
    
    /// Part number / value (e.g., "LM7805", "10k", "STM32F411")
    pub part_number: String,
    
    /// Optional: Library ID for additional context
    #[serde(skip_serializing_if = "Option::is_none")]
    pub lib_id: Option<String>,
    
    /// Optional: Footprint for additional context
    #[serde(skip_serializing_if = "Option::is_none")]
    pub footprint: Option<String>,
}

impl ComponentInput {
    pub fn new(ref_des: impl Into<String>, part_number: impl Into<String>) -> Self {
        Self {
            ref_des: ref_des.into(),
            part_number: part_number.into(),
            lib_id: None,
            footprint: None,
        }
    }
    
    pub fn with_lib_id(mut self, lib_id: impl Into<String>) -> Self {
        self.lib_id = Some(lib_id.into());
        self
    }
    
    pub fn with_footprint(mut self, footprint: impl Into<String>) -> Self {
        self.footprint = Some(footprint.into());
        self
    }
}

/// Result of component classification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClassificationResult {
    /// The input component
    pub component: ComponentInput,
    
    /// Primary classified role
    pub role: ComponentRole,
    
    /// Confidence score (0.0 - 1.0)
    pub confidence: f32,
    
    /// Alternative roles if uncertain
    #[serde(default)]
    pub alternatives: Vec<(ComponentRole, f32)>,
    
    /// Reasoning from the model
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reasoning: Option<String>,
}

/// Component Role Classifier using Phi-3 via Ollama
pub struct ComponentRoleClassifier {
    client: Client,
    base_url: String,
    model: String,
}

#[derive(Debug, Serialize)]
struct OllamaRequest {
    model: String,
    prompt: String,
    stream: bool,
    options: OllamaOptions,
}

#[derive(Debug, Serialize)]
struct OllamaOptions {
    temperature: f32,
    num_predict: i32,
    top_p: f32,
}

#[derive(Debug, Deserialize)]
struct OllamaResponse {
    response: String,
    #[allow(dead_code)]
    done: bool,
}

impl ComponentRoleClassifier {
    /// Create a new classifier with default settings (Phi-3)
    pub fn new() -> Self {
        Self::with_config(None, None)
    }
    
    /// Create a classifier with custom Ollama URL and model
    pub fn with_config(base_url: Option<String>, model: Option<String>) -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(CLASSIFIER_TIMEOUT_SECS))
            .build()
            .unwrap_or_default();
        
        Self {
            client,
            base_url: base_url.unwrap_or_else(|| DEFAULT_OLLAMA_URL.to_string()),
            model: model.unwrap_or_else(|| DEFAULT_MODEL.to_string()),
        }
    }
    
    /// Set the model to use
    pub fn with_model(mut self, model: String) -> Self {
        self.model = model;
        self
    }
    
    /// List all available models from Ollama
    pub async fn list_available_models(&self) -> Result<Vec<String>, AIError> {
        let url = format!("{}/api/tags", self.base_url);
        
        let response = self.client.get(&url).send().await
            .map_err(AIError::RequestFailed)?;
        
        if !response.status().is_success() {
            return Err(AIError::ApiError {
                status: response.status().as_u16(),
                message: "Failed to list models".to_string(),
            });
        }
        
        let models: serde_json::Value = response.json().await
            .map_err(|e| AIError::ParseError(e.to_string()))?;
        
        if let Some(models_array) = models.get("models").and_then(|m| m.as_array()) {
            Ok(models_array
                .iter()
                .filter_map(|m| m.get("name").and_then(|n| n.as_str()).map(|s| s.to_string()))
                .collect())
        } else {
            Ok(vec![])
        }
    }
    
    /// Check if Phi-3 is available
    pub async fn is_available(&self) -> bool {
        let url = format!("{}/api/tags", self.base_url);
        
        match self.client.get(&url).send().await {
            Ok(response) => {
                if response.status().is_success() {
                    // Check if the model is actually available
                    if let Ok(models) = response.json::<serde_json::Value>().await {
                        if let Some(models_array) = models.get("models").and_then(|m| m.as_array()) {
                            // Log available models for debugging
                            let available_names: Vec<String> = models_array
                                .iter()
                                .filter_map(|m| m.get("name").and_then(|n| n.as_str()).map(|s| s.to_string()))
                                .collect();
                            tracing::debug!("Available Ollama models: {:?}, looking for: {}", available_names, self.model);
                            
                            // Check if any model name matches (supports variations)
                            let found = models_array.iter().any(|m| {
                                if let Some(name) = m.get("name").and_then(|n| n.as_str()) {
                                    self.model_matches(name, &self.model)
                                } else {
                                    false
                                }
                            });
                            
                            if !found {
                                tracing::warn!(
                                    "Model '{}' not found in available models: {:?}. Try: ollama pull {}",
                                    self.model,
                                    available_names,
                                    self.model
                                );
                            }
                            
                            return found;
                        }
                    }
                    false
                } else {
                    tracing::warn!("Ollama API returned non-success status: {}", response.status());
                    false
                }
            }
            Err(e) => {
                tracing::warn!("Failed to connect to Ollama at {}: {}", self.base_url, e);
                false
            }
        }
    }
    
    /// Check if a model name matches the requested model (case-insensitive, handles variations)
    fn model_matches(&self, available_name: &str, requested: &str) -> bool {
        let available_lower = available_name.to_lowercase();
        let requested_lower = requested.to_lowercase();
        
        // Exact match (case-insensitive)
        if available_lower == requested_lower {
            return true;
        }
        
        // Normalize model names by removing common separators for comparison
        // This helps match "phi3", "phi-3", "phi_3", "microsoft/phi-3-mini", etc.
        let normalize = |s: &str| -> String {
            s.replace('-', "").replace('_', "").replace('/', "").replace(':', "")
        };
        
        let available_normalized = normalize(&available_lower);
        let requested_normalized = normalize(&requested_lower);
        
        // Check if normalized names match (handles "phi3" vs "phi-3" vs "microsoft/phi-3-mini")
        if available_normalized.contains("phi3") && requested_normalized.contains("phi3") {
            // Both contain phi3, so they're related
            // But be more specific: check if the base name matches
            if available_normalized.starts_with("phi3") || requested_normalized.starts_with("phi3") {
                return true;
            }
            // Also match if one is a variant of the other (e.g., "phi3mini" contains "phi3")
            if available_normalized.contains(&requested_normalized) || requested_normalized.contains(&available_normalized) {
                return true;
            }
        }
        
        // Check if model name starts with requested model (e.g., "phi3:latest" starts with "phi3")
        if available_lower.starts_with(&requested_lower) {
            let remaining = &available_lower[requested_lower.len()..];
            // Match if followed by ':' (tag), '-' (variant), '/' (namespace), or end of string
            return remaining.is_empty() || remaining.starts_with(':') || remaining.starts_with('-') || remaining.starts_with('/');
        }
        
        // Check if requested model starts with installed model name
        if requested_lower.starts_with(&available_lower) {
            let remaining = &requested_lower[available_lower.len()..];
            return remaining.is_empty() || remaining.starts_with(':') || remaining.starts_with('-') || remaining.starts_with('/');
        }
        
        false
    }
    
    /// Classify a single component
    pub async fn classify(&self, input: &ComponentInput) -> Result<ClassificationResult, AIError> {
        let prompt = self.build_classification_prompt(input);
        let response = self.generate(&prompt).await?;
        self.parse_classification_response(input, &response)
    }
    
    /// Classify multiple components in batch (more efficient)
    pub async fn classify_batch(&self, inputs: &[ComponentInput]) -> Result<Vec<ClassificationResult>, AIError> {
        if inputs.is_empty() {
            return Ok(vec![]);
        }
        
        // For small batches, classify individually
        if inputs.len() <= 3 {
            let mut results = Vec::with_capacity(inputs.len());
            for input in inputs {
                results.push(self.classify(input).await?);
            }
            return Ok(results);
        }
        
        // For larger batches, use batch prompt
        let prompt = self.build_batch_classification_prompt(inputs);
        let response = self.generate(&prompt).await?;
        self.parse_batch_classification_response(inputs, &response)
    }
    
    /// Build the classification prompt for a single component
    fn build_classification_prompt(&self, input: &ComponentInput) -> String {
        let context = if let Some(ref lib_id) = input.lib_id {
            format!(" (Library: {})", lib_id)
        } else {
            String::new()
        };
        
        format!(r#"You are an expert electronics engineer. Classify the role of this component in a circuit.

COMPONENT:
- Reference Designator: {}
- Part Number/Value: {}{}

TASK: Determine the functional role of this component in a circuit design.

Common roles include:
- Power: Buck_Regulator, Boost_Regulator, LDO_Regulator, Decoupling_Capacitor, Bulk_Capacitor
- MCU: MCU, MCU_GPIO, MCU_ADC, MCU_PWM, MCU_I2C, MCU_SPI, MCU_UART
- Communication: I2C_Slave, I2C_Master, SPI_Slave, UART_Transceiver, USB_Controller, CAN_Transceiver
- Analog: OpAmp, Comparator, ADC, DAC, Voltage_Reference
- Timing: Crystal, Oscillator, RTC, Timer_IC
- Sensors: Temperature_Sensor, Accelerometer_Gyro, Current_Sensor
- Memory: EEPROM, Flash, SRAM
- Protection: TVS_Diode, Fuse, ESD_Protection
- Passive: Pull_Up_Resistor, Pull_Down_Resistor, Voltage_Divider, Filter_Resistor
- Display: LED_Indicator, LED_Driver, LCD_Display
- Switching: Load_Switch, Motor_Driver, Relay_Driver

Respond with ONLY a JSON object in this exact format:
{{
  "role": "ROLE_NAME",
  "confidence": 0.95,
  "reasoning": "Brief explanation"
}}

Use underscores in role names (e.g., Buck_Regulator, not BuckRegulator)."#,
            input.ref_des,
            input.part_number,
            context
        )
    }
    
    /// Build batch classification prompt
    fn build_batch_classification_prompt(&self, inputs: &[ComponentInput]) -> String {
        let components_list: String = inputs
            .iter()
            .enumerate()
            .map(|(i, c)| {
                let lib_info = c.lib_id.as_ref()
                    .map(|l| format!(" [{}]", l))
                    .unwrap_or_default();
                format!("{}. {} - {}{}", i + 1, c.ref_des, c.part_number, lib_info)
            })
            .collect::<Vec<_>>()
            .join("\n");
        
        format!(r#"You are an expert electronics engineer. Classify the roles of these components in a circuit.

COMPONENTS:
{}

TASK: For each component, determine its functional role in the circuit.

Common roles: Buck_Regulator, LDO_Regulator, MCU, MCU_GPIO, I2C_Slave, SPI_Slave, UART_Transceiver, 
OpAmp, ADC, DAC, Crystal, EEPROM, TVS_Diode, Pull_Up_Resistor, Decoupling_Capacitor, LED_Indicator, etc.

Respond with ONLY a JSON array:
[
  {{"ref": "U1", "role": "MCU", "confidence": 0.95}},
  {{"ref": "R1", "role": "Pull_Up_Resistor", "confidence": 0.8}},
  ...
]

Use underscores in role names."#,
            components_list
        )
    }
    
    /// Generate completion from Ollama
    async fn generate(&self, prompt: &str) -> Result<String, AIError> {
        let url = format!("{}/api/generate", self.base_url);
        
        let request = OllamaRequest {
            model: self.model.clone(),
            prompt: prompt.to_string(),
            stream: false,
            options: OllamaOptions {
                temperature: 0.1,  // Low temperature for consistent classification
                num_predict: 500,
                top_p: 0.9,
            },
        };
        
        tracing::debug!("Sending classification request to Ollama ({})", self.model);
        
        let response = self.client
            .post(&url)
            .json(&request)
            .send()
            .await
            .map_err(AIError::RequestFailed)?;
        
        if !response.status().is_success() {
            let status = response.status().as_u16();
            let message = response.text().await.unwrap_or_default();
            return Err(AIError::ApiError { status, message });
        }
        
        let ollama_response: OllamaResponse = response.json().await
            .map_err(|e| AIError::ParseError(e.to_string()))?;
        
        Ok(ollama_response.response)
    }
    
    /// Parse the classification response
    fn parse_classification_response(
        &self,
        input: &ComponentInput,
        response: &str,
    ) -> Result<ClassificationResult, AIError> {
        // Try to extract JSON from the response
        let json_str = extract_json_object(response);
        
        if let Some(json) = json_str {
            if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(&json) {
                let role_str = parsed.get("role")
                    .and_then(|v| v.as_str())
                    .unwrap_or("Unknown");
                
                let confidence = parsed.get("confidence")
                    .and_then(|v| v.as_f64())
                    .unwrap_or(0.5) as f32;
                
                let reasoning = parsed.get("reasoning")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string());
                
                let role = parse_role_string(role_str);
                
                return Ok(ClassificationResult {
                    component: input.clone(),
                    role,
                    confidence,
                    alternatives: vec![],
                    reasoning,
                });
            }
        }
        
        // Fallback: try to infer from reference designator
        let role = infer_role_from_ref_des(&input.ref_des, &input.part_number);
        
        Ok(ClassificationResult {
            component: input.clone(),
            role,
            confidence: 0.3,
            alternatives: vec![],
            reasoning: Some("Fallback classification based on reference designator".to_string()),
        })
    }
    
    /// Parse batch classification response
    fn parse_batch_classification_response(
        &self,
        inputs: &[ComponentInput],
        response: &str,
    ) -> Result<Vec<ClassificationResult>, AIError> {
        // Try to extract JSON array
        let json_str = extract_json_array(response);
        
        let mut results = Vec::with_capacity(inputs.len());
        let mut parsed_map: HashMap<String, (ComponentRole, f32)> = HashMap::new();
        
        if let Some(json) = json_str {
            if let Ok(parsed) = serde_json::from_str::<Vec<serde_json::Value>>(&json) {
                for item in parsed {
                    if let (Some(ref_des), Some(role_str)) = (
                        item.get("ref").and_then(|v| v.as_str()),
                        item.get("role").and_then(|v| v.as_str()),
                    ) {
                        let confidence = item.get("confidence")
                            .and_then(|v| v.as_f64())
                            .unwrap_or(0.5) as f32;
                        
                        let role = parse_role_string(role_str);
                        parsed_map.insert(ref_des.to_string(), (role, confidence));
                    }
                }
            }
        }
        
        // Build results, using parsed data or fallback
        for input in inputs {
            let (role, confidence) = parsed_map
                .get(&input.ref_des)
                .cloned()
                .unwrap_or_else(|| {
                    let role = infer_role_from_ref_des(&input.ref_des, &input.part_number);
                    (role, 0.3)
                });
            
            results.push(ClassificationResult {
                component: input.clone(),
                role,
                confidence,
                alternatives: vec![],
                reasoning: None,
            });
        }
        
        Ok(results)
    }
    
    /// Get the current model name
    pub fn model(&self) -> &str {
        &self.model
    }
}

impl Default for ComponentRoleClassifier {
    fn default() -> Self {
        Self::new()
    }
}

/// Extract JSON object from response text
fn extract_json_object(text: &str) -> Option<String> {
    let start = text.find('{')?;
    let mut depth = 0;
    let mut end = start;
    
    for (i, c) in text[start..].char_indices() {
        match c {
            '{' => depth += 1,
            '}' => {
                depth -= 1;
                if depth == 0 {
                    end = start + i + 1;
                    break;
                }
            }
            _ => {}
        }
    }
    
    if depth == 0 && end > start {
        Some(text[start..end].to_string())
    } else {
        None
    }
}

/// Extract JSON array from response text
fn extract_json_array(text: &str) -> Option<String> {
    let start = text.find('[')?;
    let mut depth = 0;
    let mut end = start;
    
    for (i, c) in text[start..].char_indices() {
        match c {
            '[' => depth += 1,
            ']' => {
                depth -= 1;
                if depth == 0 {
                    end = start + i + 1;
                    break;
                }
            }
            _ => {}
        }
    }
    
    if depth == 0 && end > start {
        Some(text[start..end].to_string())
    } else {
        None
    }
}

/// Parse role string to ComponentRole enum
fn parse_role_string(s: &str) -> ComponentRole {
    // Normalize: remove underscores, convert to lowercase for matching
    let normalized = s.replace('_', "").to_lowercase();
    
    match normalized.as_str() {
        // Power Management
        "buckregulator" | "buck" | "stepdown" => ComponentRole::BuckRegulator,
        "boostregulator" | "boost" | "stepup" => ComponentRole::BoostRegulator,
        "buckboostregulator" | "buckboost" => ComponentRole::BuckBoostRegulator,
        "ldoregulator" | "ldo" | "linearregulator" => ComponentRole::LDORegulator,
        "switchingregulator" | "smps" => ComponentRole::SwitchingRegulator,
        "powermosfet" | "mosfet" => ComponentRole::PowerMOSFET,
        "powerdiode" | "schottky" | "rectifier" => ComponentRole::PowerDiode,
        "powerinductor" | "inductor" => ComponentRole::PowerInductor,
        "bulkcapacitor" | "bulk" => ComponentRole::BulkCapacitor,
        "decouplingcapacitor" | "decoupling" | "bypass" => ComponentRole::DecouplingCapacitor,
        "filtercapacitor" | "filter" => ComponentRole::FilterCapacitor,
        
        // MCU
        "mcu" | "microcontroller" => ComponentRole::MCU,
        "mcugpio" | "gpio" => ComponentRole::MCU_GPIO,
        "mcuadc" => ComponentRole::MCU_ADC,
        "mcupwm" | "pwm" => ComponentRole::MCU_PWM,
        "mcutimer" => ComponentRole::MCU_Timer,
        "mcuuart" => ComponentRole::MCU_UART,
        "mcuspi" => ComponentRole::MCU_SPI,
        "mcui2c" => ComponentRole::MCU_I2C,
        "dsp" => ComponentRole::DSP,
        "fpga" => ComponentRole::FPGA,
        "cpld" => ComponentRole::CPLD,
        
        // Communication
        "i2cmaster" => ComponentRole::I2C_Master,
        "i2cslave" | "i2c" => ComponentRole::I2C_Slave,
        "spimaster" => ComponentRole::SPI_Master,
        "spislave" | "spi" => ComponentRole::SPI_Slave,
        "uarttransceiver" | "uart" => ComponentRole::UART_Transceiver,
        "rs232driver" | "rs232" => ComponentRole::RS232_Driver,
        "rs485transceiver" | "rs485" => ComponentRole::RS485_Transceiver,
        "cantransceiver" | "can" => ComponentRole::CAN_Transceiver,
        "usbcontroller" | "usb" => ComponentRole::USB_Controller,
        "usbphy" => ComponentRole::USB_PHY,
        "ethernetphy" | "ethernet" => ComponentRole::Ethernet_PHY,
        "wifimodule" | "wifi" => ComponentRole::WiFi_Module,
        "bluetoothmodule" | "bluetooth" | "ble" => ComponentRole::Bluetooth_Module,
        "loramodule" | "lora" => ComponentRole::LoRa_Module,
        
        // Analog
        "opamp" | "operationalamplifier" => ComponentRole::OpAmp,
        "comparator" => ComponentRole::Comparator,
        "adc" | "analogtodigital" => ComponentRole::ADC,
        "dac" | "digitaltoanalog" => ComponentRole::DAC,
        "voltagereference" | "vref" => ComponentRole::VoltageReference,
        "currentsense" => ComponentRole::CurrentSense,
        "instrumentationamp" => ComponentRole::Instrumentation_Amp,
        "pga" | "programmablegain" => ComponentRole::PGA,
        
        // Timing
        "crystal" | "xtal" => ComponentRole::Crystal,
        "oscillator" | "osc" => ComponentRole::Oscillator,
        "pll" => ComponentRole::PLL,
        "clockbuffer" => ComponentRole::ClockBuffer,
        "timeric" | "timer" | "555" => ComponentRole::TimerIC,
        "rtc" | "realtimeclock" => ComponentRole::RTC,
        
        // Sensors
        "temperaturesensor" | "temp" | "thermistor" => ComponentRole::TemperatureSensor,
        "pressuresensor" | "pressure" => ComponentRole::PressureSensor,
        "accelerometergyro" | "imu" | "accelerometer" | "gyro" => ComponentRole::AccelerometerGyro,
        "magnetometer" | "compass" => ComponentRole::Magnetometer,
        "lightsensor" | "ambient" | "als" => ComponentRole::LightSensor,
        "proximitysensor" | "proximity" => ComponentRole::ProximitySensor,
        "currentsensor" => ComponentRole::CurrentSensor,
        "voltagesensor" => ComponentRole::VoltageSensor,
        
        // Memory
        "eeprom" => ComponentRole::EEPROM,
        "flash" => ComponentRole::Flash,
        "sram" => ComponentRole::SRAM,
        "dram" => ComponentRole::DRAM,
        "fram" => ComponentRole::FRAM,
        
        // Protection
        "tvsdiode" | "tvs" => ComponentRole::TVS_Diode,
        "fuse" => ComponentRole::Fuse,
        "polyfuse" | "ptc" => ComponentRole::PolyFuse,
        "esdprotection" | "esd" => ComponentRole::ESD_Protection,
        "overvoltageprotection" | "ovp" => ComponentRole::OvervoltageProtection,
        "reversepolarity" => ComponentRole::ReversePolarity,
        
        // Passive
        "pullupresistor" | "pullup" => ComponentRole::PullUpResistor,
        "pulldownresistor" | "pulldown" => ComponentRole::PullDownResistor,
        "currentlimitresistor" | "currentlimit" => ComponentRole::CurrentLimitResistor,
        "voltagedivider" | "divider" => ComponentRole::VoltageDivider,
        "filterresistor" => ComponentRole::FilterResistor,
        "terminationresistor" | "termination" => ComponentRole::TerminationResistor,
        "feedbackresistor" | "feedback" => ComponentRole::FeedbackResistor,
        
        // Connectors
        "powerconnector" => ComponentRole::PowerConnector,
        "signalconnector" | "connector" => ComponentRole::SignalConnector,
        "debugconnector" | "jtag" | "swd" => ComponentRole::DebugConnector,
        "programmingheader" | "isp" => ComponentRole::ProgrammingHeader,
        "testpoint" | "tp" => ComponentRole::TestPoint,
        
        // Display
        "ledindicator" | "led" => ComponentRole::LED_Indicator,
        "leddriver" => ComponentRole::LED_Driver,
        "lcddisplay" | "lcd" => ComponentRole::LCD_Display,
        "oleddisplay" | "oled" => ComponentRole::OLED_Display,
        "sevensegment" | "7seg" => ComponentRole::SevenSegment,
        
        // Switching
        "loadswitch" => ComponentRole::LoadSwitch,
        "analogswitch" => ComponentRole::AnalogSwitch,
        "multiplexer" | "mux" => ComponentRole::Multiplexer,
        "relay" => ComponentRole::Relay,
        "relaydriver" => ComponentRole::RelayDriver,
        "motordriver" | "hbridge" => ComponentRole::MotorDriver,
        "gatedriver" => ComponentRole::GateDriver,
        
        // RF
        "rfamplifier" | "lna" | "pa" => ComponentRole::RF_Amplifier,
        "rffilter" | "saw" => ComponentRole::RF_Filter,
        "antenna" => ComponentRole::Antenna,
        "balun" => ComponentRole::Balun,
        "rfswitch" => ComponentRole::RF_Switch,
        
        // Audio
        "audiocodec" | "codec" => ComponentRole::AudioCodec,
        "audioamplifier" | "classab" | "classd" => ComponentRole::AudioAmplifier,
        "microphone" | "mic" => ComponentRole::Microphone,
        "speaker" | "buzzer" => ComponentRole::Speaker,
        
        // Isolation
        "optocoupler" | "opto" => ComponentRole::Optocoupler,
        "digitalisolator" | "isolator" => ComponentRole::DigitalIsolator,
        "isolateddcdc" => ComponentRole::IsolatedDCDC,
        
        // Generic
        "generici" | "ic" => ComponentRole::GenericIC,
        "genericpassive" | "passive" => ComponentRole::GenericPassive,
        
        _ => ComponentRole::Unknown,
    }
}

/// Infer role from reference designator and part number (fallback)
fn infer_role_from_ref_des(ref_des: &str, part_number: &str) -> ComponentRole {
    let ref_upper = ref_des.to_uppercase();
    let part_upper = part_number.to_uppercase();
    
    // Check part number patterns first
    if part_upper.contains("LM78") || part_upper.contains("LM79") || 
       part_upper.contains("7805") || part_upper.contains("7812") ||
       part_upper.contains("AMS1117") || part_upper.contains("LM1117") ||
       part_upper.contains("LD1117") || part_upper.contains("LDO") {
        return ComponentRole::LDORegulator;
    }
    
    if part_upper.contains("LM2596") || part_upper.contains("MP1584") ||
       part_upper.contains("TPS54") || part_upper.contains("LTC3") ||
       part_upper.contains("BUCK") {
        return ComponentRole::BuckRegulator;
    }
    
    if part_upper.contains("STM32") || part_upper.contains("ATMEGA") ||
       part_upper.contains("PIC") || part_upper.contains("ESP32") ||
       part_upper.contains("RP2040") || part_upper.contains("NRF52") {
        return ComponentRole::MCU;
    }
    
    if part_upper.contains("24C") || part_upper.contains("24LC") ||
       part_upper.contains("AT24") || part_upper.contains("EEPROM") {
        return ComponentRole::EEPROM;
    }
    
    if part_upper.contains("NE555") || part_upper.contains("LM555") {
        return ComponentRole::TimerIC;
    }
    
    if part_upper.contains("CH340") || part_upper.contains("CP210") ||
       part_upper.contains("FT232") || part_upper.contains("PL2303") {
        return ComponentRole::USB_Controller;
    }
    
    if part_upper.contains("MAX232") || part_upper.contains("SP232") {
        return ComponentRole::RS232_Driver;
    }
    
    if part_upper.contains("MCP2551") || part_upper.contains("TJA1050") ||
       part_upper.contains("SN65HVD") {
        return ComponentRole::CAN_Transceiver;
    }
    
    // Reference designator based inference
    let prefix: String = ref_upper.chars().take_while(|c| c.is_alphabetic()).collect();
    
    match prefix.as_str() {
        "U" => {
            // Check if it looks like a voltage regulator
            if part_upper.contains("REG") || part_upper.contains("LDO") {
                ComponentRole::LDORegulator
            } else {
                ComponentRole::GenericIC
            }
        }
        "R" => {
            // Could be pull-up, pull-down, or general resistor
            if part_upper.contains("K") && !part_upper.contains("M") {
                // Common pull-up values
                if part_upper == "10K" || part_upper == "4.7K" || part_upper == "4K7" {
                    ComponentRole::PullUpResistor
                } else {
                    ComponentRole::GenericPassive
                }
            } else {
                ComponentRole::GenericPassive
            }
        }
        "C" => {
            // Check capacitor value for decoupling
            if part_upper.contains("100N") || part_upper.contains("0.1U") ||
               part_upper.contains("10N") || part_upper.contains("1U") {
                ComponentRole::DecouplingCapacitor
            } else if part_upper.contains("U") || part_upper.contains("MF") {
                // Larger values are likely bulk
                ComponentRole::BulkCapacitor
            } else {
                ComponentRole::FilterCapacitor
            }
        }
        "L" => ComponentRole::PowerInductor,
        "D" => {
            if part_upper.contains("LED") {
                ComponentRole::LED_Indicator
            } else if part_upper.contains("TVS") || part_upper.contains("SMBJ") {
                ComponentRole::TVS_Diode
            } else {
                ComponentRole::PowerDiode
            }
        }
        "Q" => ComponentRole::PowerMOSFET,
        "Y" | "X" => ComponentRole::Crystal,
        "J" | "P" | "CN" => ComponentRole::SignalConnector,
        "F" => ComponentRole::Fuse,
        "LED" => ComponentRole::LED_Indicator,
        "SW" | "S" => ComponentRole::LoadSwitch,
        "K" => ComponentRole::Relay,
        "T" => ComponentRole::Optocoupler,
        "TP" => ComponentRole::TestPoint,
        _ => ComponentRole::Unknown,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_parse_role_string() {
        assert_eq!(parse_role_string("Buck_Regulator"), ComponentRole::BuckRegulator);
        assert_eq!(parse_role_string("LDO_Regulator"), ComponentRole::LDORegulator);
        assert_eq!(parse_role_string("MCU"), ComponentRole::MCU);
        assert_eq!(parse_role_string("I2C_Slave"), ComponentRole::I2C_Slave);
        assert_eq!(parse_role_string("Pull_Up_Resistor"), ComponentRole::PullUpResistor);
        assert_eq!(parse_role_string("Decoupling_Capacitor"), ComponentRole::DecouplingCapacitor);
    }
    
    #[test]
    fn test_infer_role_from_ref_des() {
        assert_eq!(
            infer_role_from_ref_des("U1", "LM7805"),
            ComponentRole::LDORegulator
        );
        assert_eq!(
            infer_role_from_ref_des("U2", "STM32F411"),
            ComponentRole::MCU
        );
        assert_eq!(
            infer_role_from_ref_des("C1", "100nF"),
            ComponentRole::DecouplingCapacitor
        );
        assert_eq!(
            infer_role_from_ref_des("Y1", "8MHz"),
            ComponentRole::Crystal
        );
    }
    
    #[test]
    fn test_component_role_display() {
        assert_eq!(format!("{}", ComponentRole::BuckRegulator), "BuckRegulator");
        assert_eq!(format!("{}", ComponentRole::MCU_GPIO), "MCU_GPIO");
    }
    
    #[test]
    fn test_component_role_category() {
        assert_eq!(ComponentRole::BuckRegulator.category(), "Power Management");
        assert_eq!(ComponentRole::MCU.category(), "Microcontrollers & Processors");
        assert_eq!(ComponentRole::I2C_Slave.category(), "Communication");
    }
    
    #[test]
    fn test_extract_json_object() {
        let text = r#"Here is the result: {"role": "MCU", "confidence": 0.95} and more text"#;
        let json = extract_json_object(text).unwrap();
        assert!(json.contains("MCU"));
    }
    
    #[test]
    fn test_component_input_builder() {
        let input = ComponentInput::new("U1", "STM32F411")
            .with_lib_id("Device:STM32F411")
            .with_footprint("QFP-48");
        
        assert_eq!(input.ref_des, "U1");
        assert_eq!(input.part_number, "STM32F411");
        assert_eq!(input.lib_id, Some("Device:STM32F411".to_string()));
        assert_eq!(input.footprint, Some("QFP-48".to_string()));
    }
}
