//! Circuit Analysis Module
//!
//! This module provides analysis functions that work with the Circuit graph.
//! These functions leverage the graph structure for efficient queries.

use std::collections::{HashMap, HashSet, VecDeque};

use super::circuit::Circuit;
use super::schema::*;

// ============================================================================
// Voltage propagation types
// ============================================================================

/// Result of voltage propagation analysis
#[derive(Debug, Clone)]
pub struct VoltagePropagationResult {
    /// Net name to voltage mapping
    pub net_voltages: HashMap<String, f64>,

    /// Detailed tracking of how each voltage was determined
    pub voltage_assignments: HashMap<String, VoltageAssignment>,

    /// Components with potential voltage issues
    pub voltage_issues: Vec<VoltageIssue>,
}

/// How a voltage was determined
#[derive(Debug, Clone, PartialEq)]
pub enum VoltageSource {
    /// Pre-set on the net (e.g. from schematic annotation)
    NetAnnotation,
    /// Derived from a power symbol value (VCC, +3V3, etc.)
    PowerSymbol,
    /// Output of a recognised voltage regulator
    RegulatorOutput,
    /// Heuristically parsed from the net name
    NetNameHeuristic,
    /// Propagated through a passive component
    Propagated,
    /// Calculated from a resistive voltage divider
    VoltageDivider,
    /// Propagated through a diode with a forward-voltage drop
    DiodeDrop,
}

/// Tracks how voltage was assigned to a net
#[derive(Debug, Clone)]
pub struct VoltageAssignment {
    pub voltage: f64,
    pub source: VoltageSource,
    /// The net the voltage was propagated from (if applicable)
    pub source_net: Option<String>,
    /// Confidence in the assignment (0.0 – 1.0)
    pub confidence: f64,
}

/// A potential voltage-related issue
#[derive(Debug, Clone)]
pub struct VoltageIssue {
    pub component_ref: String,
    pub pin_number: String,
    pub net_name: String,
    pub expected_voltage: Option<f64>,
    pub actual_voltage: f64,
    pub issue_type: VoltageIssueType,
    pub message: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VoltageIssueType {
    Overvoltage,
    Undervoltage,
    VoltageMismatch,
    UnknownVoltage,
    ConflictingVoltage,
}

// ============================================================================
// Voltage propagation algorithm (4-phase BFS)
// ============================================================================

/// Analyse voltage propagation in the circuit.
///
/// **Phase 1 – Seed**: collect known voltages from annotations, power symbols,
/// regulator outputs, and net-name heuristics.
///
/// **Phase 2 – Propagate**: BFS through the component graph.  Passives (C, L)
/// pass voltage through; resistors propagate at reduced confidence; diodes
/// subtract a forward-voltage drop; regulators and ICs block propagation.
///
/// **Phase 3 – Voltage dividers**: detect R-R junctions and compute Vout.
///
/// **Phase 4 – Validate**: check component voltage ratings and flag issues.
pub fn analyze_voltage_propagation(circuit: &Circuit) -> VoltagePropagationResult {
    let mut result = VoltagePropagationResult {
        net_voltages: HashMap::new(),
        voltage_assignments: HashMap::new(),
        voltage_issues: Vec::new(),
    };

    // ------------------------------------------------------------------
    // Phase 1: Seed
    // ------------------------------------------------------------------
    let mut queue: VecDeque<String> = VecDeque::new();

    // 1a. Pre-annotated voltages on nets
    for net in circuit.nets() {
        if let Some(voltage) = net.voltage_level {
            seed_voltage(
                &mut result,
                &mut queue,
                &net.net_name,
                voltage,
                VoltageSource::NetAnnotation,
                None,
                1.0,
            );
        }
    }

    // 1b. Voltage sources identified by the circuit graph (regulators + power symbols)
    let voltage_sources = circuit.find_voltage_sources_pub();
    for (net_name, voltage) in &voltage_sources {
        if !result.net_voltages.contains_key(net_name) {
            seed_voltage(
                &mut result,
                &mut queue,
                net_name,
                *voltage,
                VoltageSource::RegulatorOutput,
                None,
                0.95,
            );
        }
    }

    // 1c. Heuristic: parse voltage from net names that were not yet seeded
    for net in circuit.nets() {
        if !result.net_voltages.contains_key(&net.net_name) {
            if let Some(voltage) = parse_voltage_from_name(&net.net_name) {
                seed_voltage(
                    &mut result,
                    &mut queue,
                    &net.net_name,
                    voltage,
                    VoltageSource::NetNameHeuristic,
                    None,
                    0.80,
                );
            }
        }
    }

    // ------------------------------------------------------------------
    // Phase 2: BFS propagation
    // ------------------------------------------------------------------
    let mut visited_edges: HashSet<(String, String)> = HashSet::new(); // (component, net)

    while let Some(net_name) = queue.pop_front() {
        let voltage = match result.net_voltages.get(&net_name) {
            Some(&v) => v,
            None => continue,
        };
        let confidence = result
            .voltage_assignments
            .get(&net_name)
            .map(|a| a.confidence)
            .unwrap_or(0.5);

        let components = circuit.components_on_net(&net_name);

        for comp in &components {
            // Skip if we already traversed this component from this net
            let edge_key = (comp.ref_des.clone(), net_name.clone());
            if visited_edges.contains(&edge_key) {
                continue;
            }
            visited_edges.insert(edge_key);

            let comp_type = comp.component_type();

            match comp_type {
                // ---- Capacitors & Inductors: pass voltage through ----
                ComponentType::Capacitor | ComponentType::Inductor => {
                    let other_nets = find_other_side_nets(circuit, &comp.ref_des, &net_name);
                    for other in &other_nets {
                        try_propagate(
                            &mut result,
                            &mut queue,
                            other,
                            voltage,
                            VoltageSource::Propagated,
                            Some(net_name.clone()),
                            confidence * 0.95,
                        );
                    }
                }

                // ---- Resistors: propagate with lower confidence ----
                ComponentType::Resistor => {
                    let other_nets = find_other_side_nets(circuit, &comp.ref_des, &net_name);
                    for other in &other_nets {
                        // Don't overwrite higher-confidence values
                        try_propagate(
                            &mut result,
                            &mut queue,
                            other,
                            voltage,
                            VoltageSource::Propagated,
                            Some(net_name.clone()),
                            confidence * 0.6, // resistors can have voltage drop
                        );
                    }
                }

                // ---- Diodes: subtract forward voltage drop ----
                ComponentType::Diode => {
                    let other_nets = find_other_side_nets(circuit, &comp.ref_des, &net_name);
                    // Heuristic: assume current flows from higher-voltage net (anode)
                    // to the other side (cathode), dropping ~0.7 V.
                    let dropped = (voltage - 0.7).max(0.0);
                    for other in &other_nets {
                        try_propagate(
                            &mut result,
                            &mut queue,
                            other,
                            dropped,
                            VoltageSource::DiodeDrop,
                            Some(net_name.clone()),
                            confidence * 0.7,
                        );
                    }
                }

                // ---- ICs, regulators, transistors: do NOT propagate ----
                _ => {}
            }
        }
    }

    // ------------------------------------------------------------------
    // Phase 3: Voltage divider detection
    // ------------------------------------------------------------------
    let divider_results = detect_voltage_dividers(circuit, &result.net_voltages);
    for (net_name, voltage) in divider_results {
        if !result.net_voltages.contains_key(&net_name) {
            result.net_voltages.insert(net_name.clone(), voltage);
            result.voltage_assignments.insert(
                net_name,
                VoltageAssignment {
                    voltage,
                    source: VoltageSource::VoltageDivider,
                    source_net: None,
                    confidence: 0.75,
                },
            );
        }
    }

    // ------------------------------------------------------------------
    // Phase 4: Validation
    // ------------------------------------------------------------------
    result.voltage_issues = validate_component_ratings(circuit, &result.net_voltages);

    result
}

// ============================================================================
// Voltage propagation helpers
// ============================================================================

/// Insert a seed voltage into the result and enqueue for BFS.
fn seed_voltage(
    result: &mut VoltagePropagationResult,
    queue: &mut VecDeque<String>,
    net_name: &str,
    voltage: f64,
    source: VoltageSource,
    source_net: Option<String>,
    confidence: f64,
) {
    result.net_voltages.insert(net_name.to_string(), voltage);
    result.voltage_assignments.insert(
        net_name.to_string(),
        VoltageAssignment {
            voltage,
            source,
            source_net,
            confidence,
        },
    );
    queue.push_back(net_name.to_string());
}

/// Attempt to propagate a voltage to a net. Only succeeds if the net has no
/// voltage yet, or if the new assignment has higher confidence.
fn try_propagate(
    result: &mut VoltagePropagationResult,
    queue: &mut VecDeque<String>,
    net_name: &str,
    voltage: f64,
    source: VoltageSource,
    source_net: Option<String>,
    confidence: f64,
) {
    if let Some(existing) = result.voltage_assignments.get(net_name) {
        // Already assigned with higher or equal confidence — check for conflict
        if existing.confidence >= confidence {
            if (existing.voltage - voltage).abs() > 0.5 {
                result.voltage_issues.push(VoltageIssue {
                    component_ref: String::new(),
                    pin_number: String::new(),
                    net_name: net_name.to_string(),
                    expected_voltage: Some(existing.voltage),
                    actual_voltage: voltage,
                    issue_type: VoltageIssueType::ConflictingVoltage,
                    message: format!(
                        "Net {} has conflicting voltage sources: {:.2}V vs {:.2}V",
                        net_name, existing.voltage, voltage
                    ),
                });
            }
            return;
        }
    }

    result.net_voltages.insert(net_name.to_string(), voltage);
    result.voltage_assignments.insert(
        net_name.to_string(),
        VoltageAssignment {
            voltage,
            source,
            source_net,
            confidence,
        },
    );
    queue.push_back(net_name.to_string());
}

/// Parse a voltage value from a net name.
///
/// Handles patterns such as `+3V3`, `+5V`, `VCC_12V`, `+1V8`, `3.3V`, `5V0`.
pub fn parse_voltage_from_name(name: &str) -> Option<f64> {
    let upper = name.to_uppercase();

    // Explicit ground
    if upper == "GND" || upper == "VSS" || upper == "AGND" || upper == "DGND" || upper == "0V" {
        return Some(0.0);
    }

    // Pattern: digits V digits  (e.g. "3V3" -> 3.3, "1V8" -> 1.8, "5V0" -> 5.0)
    for window in upper.as_bytes().windows(3) {
        if window[1] == b'V' && window[0].is_ascii_digit() && window[2].is_ascii_digit() {
            let integer = (window[0] - b'0') as f64;
            let fraction = (window[2] - b'0') as f64 / 10.0;
            return Some(integer + fraction);
        }
    }

    // Pattern: "5V", "12V", "24V", "3.3V" etc. (digits followed by V)
    // Scan for every 'V' in the string and check if digits precede it
    for (pos, _) in upper.char_indices().filter(|&(_, c)| c == 'V') {
        let prefix = &upper[..pos];
        let num_str: String = prefix
            .chars()
            .rev()
            .take_while(|c| c.is_ascii_digit() || *c == '.')
            .collect::<Vec<_>>()
            .into_iter()
            .rev()
            .collect();
        if !num_str.is_empty() {
            if let Ok(v) = num_str.parse::<f64>() {
                if v > 0.0 && v <= 100.0 {
                    return Some(v);
                }
            }
        }
    }

    None
}

/// Parse a resistance value string into Ohms.
///
/// Handles `"10k"`, `"4.7k"`, `"100R"`, `"1M"`, `"470"`, `"10K"`, `"2k2"`.
pub fn parse_resistance_value(value: &str) -> Option<f64> {
    let v = value.trim().to_lowercase();
    if v.is_empty() {
        return None;
    }

    // Pattern: "XkY" / "XmY" / "XrY" (e.g. "4k7" -> 4700, "2r2" -> 2.2)
    for (sep, mult) in [('k', 1_000.0), ('m', 1_000_000.0), ('r', 1.0)] {
        if let Some(pos) = v.find(sep) {
            let integer: f64 = v[..pos].parse().unwrap_or(0.0);
            let fraction_str = &v[pos + 1..];
            let fraction: f64 = if fraction_str.is_empty() {
                0.0
            } else {
                fraction_str
                    .parse::<f64>()
                    .map(|f| f / 10.0_f64.powi(fraction_str.len() as i32))
                    .unwrap_or(0.0)
            };
            let ohms = (integer + fraction) * mult;
            if ohms > 0.0 {
                return Some(ohms);
            }
        }
    }

    // Plain numeric (assume ohms)
    v.parse::<f64>().ok().filter(|&x| x > 0.0)
}

/// Find nets on the "other side" of a 2-terminal passive component.
fn find_other_side_nets<'a>(
    circuit: &'a Circuit,
    ref_des: &str,
    current_net: &str,
) -> Vec<String> {
    circuit
        .nets_for_component(ref_des)
        .iter()
        .filter(|n| n.net_name != current_net)
        .map(|n| n.net_name.clone())
        .collect()
}

/// Detect resistive voltage dividers and calculate junction voltages.
///
/// A divider is identified when a net is connected to exactly two resistors
/// (and nothing else), and both "other sides" of those resistors already have
/// known voltages.
fn detect_voltage_dividers(
    circuit: &Circuit,
    net_voltages: &HashMap<String, f64>,
) -> Vec<(String, f64)> {
    let mut dividers: Vec<(String, f64)> = Vec::new();

    for net in circuit.nets() {
        // Skip nets that already have a voltage
        if net_voltages.contains_key(&net.net_name) {
            continue;
        }

        let components = circuit.components_on_net(&net.net_name);

        // Must be exactly 2 resistors (optionally other passives)
        let resistors: Vec<&&UcsComponent> = components
            .iter()
            .filter(|c| c.is_resistor())
            .collect();

        if resistors.len() != 2 {
            continue;
        }

        // Get resistance values
        let r1_val = resistors[0]
            .value
            .as_deref()
            .and_then(parse_resistance_value);
        let r2_val = resistors[1]
            .value
            .as_deref()
            .and_then(parse_resistance_value);

        let (r1, r2) = match (r1_val, r2_val) {
            (Some(a), Some(b)) => (a, b),
            _ => continue,
        };

        // Find the voltage on the "other side" of each resistor
        let other1 = find_other_side_nets(circuit, &resistors[0].ref_des, &net.net_name);
        let other2 = find_other_side_nets(circuit, &resistors[1].ref_des, &net.net_name);

        let v1 = other1.iter().filter_map(|n| net_voltages.get(n)).next();
        let v2 = other2.iter().filter_map(|n| net_voltages.get(n)).next();

        if let (Some(&va), Some(&vb)) = (v1, v2) {
            // Standard voltage divider: Vout = Vlow + (Vhigh - Vlow) * R_low / (R_high + R_low)
            // We need to figure out which resistor connects to which voltage.
            let (v_high, v_low, r_top, r_bot) = if va >= vb {
                (va, vb, r1, r2)
            } else {
                (vb, va, r2, r1)
            };

            let vout = v_low + (v_high - v_low) * r_bot / (r_top + r_bot);
            dividers.push((net.net_name.clone(), vout));
        }
    }

    dividers
}

/// Check for overvoltage or voltage-mismatch issues on IC power pins.
fn validate_component_ratings(
    circuit: &Circuit,
    net_voltages: &HashMap<String, f64>,
) -> Vec<VoltageIssue> {
    let mut issues = Vec::new();

    for ic in circuit.ics() {
        if ic.is_virtual {
            continue;
        }

        // Collect power nets connected to this IC
        let power_nets: Vec<&UcsNet> = circuit
            .nets_for_component(&ic.ref_des)
            .into_iter()
            .filter(|n| n.is_power_rail)
            .collect();

        // Check for power pins with unknown voltage
        for pnet in &power_nets {
            if pnet.signal_type == SignalType::Ground {
                continue; // ground is always 0V, not an issue
            }
            if !net_voltages.contains_key(&pnet.net_name) {
                let pin = circuit
                    .get_connection_pin(&ic.ref_des, &pnet.net_name)
                    .map(|e| e.pin_number.clone())
                    .unwrap_or_default();
                issues.push(VoltageIssue {
                    component_ref: ic.ref_des.clone(),
                    pin_number: pin,
                    net_name: pnet.net_name.clone(),
                    expected_voltage: None,
                    actual_voltage: 0.0,
                    issue_type: VoltageIssueType::UnknownVoltage,
                    message: format!(
                        "{} power pin on net {} has no determinable voltage",
                        ic.ref_des, pnet.net_name
                    ),
                });
            }
        }

        // Check for voltage mismatch between power pins on the same IC
        let pin_voltages: Vec<f64> = power_nets
            .iter()
            .filter(|n| n.signal_type != SignalType::Ground)
            .filter_map(|n| net_voltages.get(&n.net_name))
            .cloned()
            .collect();

        if pin_voltages.len() >= 2 {
            let min_v = pin_voltages.iter().cloned().fold(f64::INFINITY, f64::min);
            let max_v = pin_voltages
                .iter()
                .cloned()
                .fold(f64::NEG_INFINITY, f64::max);
            // Flag if power pins differ by more than 0.5 V (suggests mixed domains)
            if (max_v - min_v) > 0.5 && min_v > 0.0 {
                issues.push(VoltageIssue {
                    component_ref: ic.ref_des.clone(),
                    pin_number: String::new(),
                    net_name: String::new(),
                    expected_voltage: Some(min_v),
                    actual_voltage: max_v,
                    issue_type: VoltageIssueType::VoltageMismatch,
                    message: format!(
                        "{} has power pins at different voltages ({:.1}V and {:.1}V) — check for mixed voltage domains",
                        ic.ref_des, min_v, max_v
                    ),
                });
            }
        }
    }

    issues
}

/// Result of connectivity analysis
#[derive(Debug, Clone)]
pub struct ConnectivityResult {
    /// Components that appear to be floating (not connected to power/ground)
    pub floating_components: Vec<String>,
    
    /// Nets with only one connection (potential issues)
    pub single_connection_nets: Vec<String>,
    
    /// Power nets and their connected components
    pub power_connections: HashMap<String, Vec<String>>,
    
    /// Ground nets and their connected components  
    pub ground_connections: HashMap<String, Vec<String>>,
}

/// Analyze circuit connectivity
pub fn analyze_connectivity(circuit: &Circuit) -> ConnectivityResult {
    let mut result = ConnectivityResult {
        floating_components: Vec::new(),
        single_connection_nets: Vec::new(),
        power_connections: HashMap::new(),
        ground_connections: HashMap::new(),
    };
    
    // Find power and ground nets
    for net in circuit.nets() {
        let connected: Vec<String> = circuit.components_on_net(&net.net_name)
            .iter()
            .map(|c| c.ref_des.clone())
            .collect();
        
        // Check for single-connection nets (potential issues)
        if connected.len() == 1 && !net.is_power_rail {
            result.single_connection_nets.push(net.net_name.clone());
        }
        
        if net.signal_type == SignalType::Power {
            result.power_connections.insert(net.net_name.clone(), connected);
        } else if net.signal_type == SignalType::Ground {
            result.ground_connections.insert(net.net_name.clone(), connected);
        }
    }
    
    // Find floating components (ICs not connected to power or ground)
    let power_connected: HashSet<String> = result.power_connections
        .values()
        .flatten()
        .cloned()
        .collect();
    
    let ground_connected: HashSet<String> = result.ground_connections
        .values()
        .flatten()
        .cloned()
        .collect();
    
    for ic in circuit.ics() {
        if !ic.is_virtual {
            let has_power = power_connected.contains(&ic.ref_des);
            let has_ground = ground_connected.contains(&ic.ref_des);
            
            if !has_power || !has_ground {
                result.floating_components.push(ic.ref_des.clone());
            }
        }
    }
    
    result
}

/// Result of decoupling capacitor analysis
#[derive(Debug, Clone)]
pub struct DecouplingAnalysisResult {
    /// ICs and their nearby decoupling capacitors
    pub ic_decoupling: HashMap<String, Vec<DecouplingCapInfo>>,
    
    /// ICs missing decoupling capacitors
    pub missing_decoupling: Vec<MissingDecoupling>,
}

#[derive(Debug, Clone)]
pub struct DecouplingCapInfo {
    pub capacitor_ref: String,
    pub value: Option<String>,
    pub distance_mm: f64,
    pub shared_nets: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct MissingDecoupling {
    pub ic_ref: String,
    pub ic_value: Option<String>,
    pub power_pins: Vec<String>,
    pub recommendation: String,
}

/// Analyze decoupling capacitor placement
pub fn analyze_decoupling(circuit: &Circuit, max_distance_mm: f64) -> DecouplingAnalysisResult {
    let mut result = DecouplingAnalysisResult {
        ic_decoupling: HashMap::new(),
        missing_decoupling: Vec::new(),
    };
    
    for ic in circuit.ics() {
        if ic.is_virtual {
            continue;
        }
        
        let nearby_caps = circuit.capacitors_near(&ic.ref_des, max_distance_mm);
        
        let mut decoupling_caps: Vec<DecouplingCapInfo> = Vec::new();
        
        for cap in &nearby_caps {
            // Check if the capacitor shares a power net with the IC
            let ic_nets: HashSet<String> = circuit.nets_for_component(&ic.ref_des)
                .iter()
                .map(|n| n.net_name.clone())
                .collect();
            
            let cap_nets: HashSet<String> = circuit.nets_for_component(&cap.ref_des)
                .iter()
                .map(|n| n.net_name.clone())
                .collect();
            
            let shared: Vec<String> = ic_nets.intersection(&cap_nets)
                .cloned()
                .collect();
            
            // Calculate distance
            let distance = if let (Some(ic_pos), Some(cap_pos)) = (&ic.position, &cap.position) {
                ic_pos.distance_to(cap_pos)
            } else {
                f64::MAX
            };
            
            // Check if it's a decoupling cap (typically 100nF or similar)
            let is_decoupling = cap.value.as_ref()
                .map(|v| is_decoupling_value(v))
                .unwrap_or(false);
            
            if is_decoupling && !shared.is_empty() {
                decoupling_caps.push(DecouplingCapInfo {
                    capacitor_ref: cap.ref_des.clone(),
                    value: cap.value.clone(),
                    distance_mm: distance,
                    shared_nets: shared,
                });
            }
        }
        
        if decoupling_caps.is_empty() {
            // Find power pins on this IC
            let power_nets: Vec<String> = circuit.nets_for_component(&ic.ref_des)
                .iter()
                .filter(|n| n.is_power_rail)
                .map(|n| n.net_name.clone())
                .collect();
            
            result.missing_decoupling.push(MissingDecoupling {
                ic_ref: ic.ref_des.clone(),
                ic_value: ic.value.clone(),
                power_pins: power_nets,
                recommendation: format!(
                    "Add 100nF ceramic capacitor within {}mm of {}",
                    max_distance_mm, ic.ref_des
                ),
            });
        }
        
        result.ic_decoupling.insert(ic.ref_des.clone(), decoupling_caps);
    }
    
    result
}

/// Check if a capacitor value is typical for decoupling
fn is_decoupling_value(value: &str) -> bool {
    let value_lower = value.to_lowercase();
    
    // Common decoupling values: 100nF, 0.1uF, 10nF, 1uF, 4.7uF, 10uF
    value_lower.contains("100n") || value_lower.contains("0.1u") 
        || value_lower.contains("10n") || value_lower.contains("1u")
        || value_lower.contains("4.7u") || value_lower.contains("10u")
        || value_lower.contains("100pf") // For high-speed
}

/// Result of signal integrity analysis
#[derive(Debug, Clone)]
pub struct SignalIntegrityResult {
    /// High-speed signals that might need termination
    pub unterminated_signals: Vec<UnterminatedSignal>,
    
    /// I2C buses without pull-ups
    pub i2c_without_pullups: Vec<I2cBusInfo>,
    
    /// SPI buses info
    pub spi_buses: Vec<SpiBusInfo>,
}

#[derive(Debug, Clone)]
pub struct UnterminatedSignal {
    pub net_name: String,
    pub signal_type: SignalType,
    pub connected_components: Vec<String>,
    pub recommendation: String,
}

#[derive(Debug, Clone)]
pub struct I2cBusInfo {
    pub sda_net: Option<String>,
    pub scl_net: Option<String>,
    pub has_pullups: bool,
    pub connected_devices: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct SpiBusInfo {
    pub mosi_net: Option<String>,
    pub miso_net: Option<String>,
    pub sck_net: Option<String>,
    pub cs_nets: Vec<String>,
    pub connected_devices: Vec<String>,
}

/// Analyze signal integrity concerns
pub fn analyze_signal_integrity(circuit: &Circuit) -> SignalIntegrityResult {
    let mut result = SignalIntegrityResult {
        unterminated_signals: Vec::new(),
        i2c_without_pullups: Vec::new(),
        spi_buses: Vec::new(),
    };
    
    // Find I2C buses
    let mut sda_net: Option<String> = None;
    let mut scl_net: Option<String> = None;
    
    for net in circuit.nets() {
        let name_upper = net.net_name.to_uppercase();
        if name_upper.contains("SDA") {
            sda_net = Some(net.net_name.clone());
        }
        if name_upper.contains("SCL") {
            scl_net = Some(net.net_name.clone());
        }
    }
    
    if sda_net.is_some() || scl_net.is_some() {
        // Check for pull-up resistors
        let has_pullups = check_for_pullups(circuit, &sda_net, &scl_net);
        
        let mut connected_devices: Vec<String> = Vec::new();
        if let Some(ref sda) = sda_net {
            connected_devices.extend(
                circuit.components_on_net(sda)
                    .iter()
                    .filter(|c| c.is_ic())
                    .map(|c| c.ref_des.clone())
            );
        }
        
        result.i2c_without_pullups.push(I2cBusInfo {
            sda_net,
            scl_net,
            has_pullups,
            connected_devices,
        });
    }
    
    // Find high-speed signals that might need termination
    for net in circuit.nets() {
        if net.signal_type == SignalType::Clock || net.signal_type == SignalType::HighSpeed {
            let components: Vec<String> = circuit.components_on_net(&net.net_name)
                .iter()
                .map(|c| c.ref_des.clone())
                .collect();
            
            // Check if there's a termination resistor
            let has_termination = components.iter().any(|ref_des| {
                ref_des.starts_with('R') || ref_des.starts_with('r')
            });
            
            if !has_termination && components.len() > 1 {
                result.unterminated_signals.push(UnterminatedSignal {
                    net_name: net.net_name.clone(),
                    signal_type: net.signal_type,
                    connected_components: components,
                    recommendation: "Consider adding series termination resistor for signal integrity".to_string(),
                });
            }
        }
    }
    
    result
}

/// Check if I2C bus has pull-up resistors
fn check_for_pullups(circuit: &Circuit, sda_net: &Option<String>, scl_net: &Option<String>) -> bool {
    let check_net = |net_name: &Option<String>| -> bool {
        if let Some(name) = net_name {
            let components = circuit.components_on_net(name);
            components.iter().any(|c| {
                c.is_resistor() && c.value.as_ref()
                    .map(|v| is_pullup_value(v))
                    .unwrap_or(false)
            })
        } else {
            false
        }
    };
    
    check_net(sda_net) && check_net(scl_net)
}

/// Check if a resistor value is typical for I2C pull-ups
fn is_pullup_value(value: &str) -> bool {
    let value_lower = value.to_lowercase();
    
    // Typical I2C pull-up values: 2.2k, 4.7k, 10k
    value_lower.contains("2.2k") || value_lower.contains("2k2")
        || value_lower.contains("4.7k") || value_lower.contains("4k7")
        || value_lower.contains("10k")
}

/// Create a summary for AI analysis
pub fn create_ai_summary(circuit: &Circuit) -> AiCircuitSummary {
    let stats = circuit.stats();
    
    let connectivity = analyze_connectivity(circuit);
    let decoupling = analyze_decoupling(circuit, 20.0);
    let signal_integrity = analyze_signal_integrity(circuit);
    let voltage = analyze_voltage_propagation(circuit);
    
    // Collect IC information
    let ics: Vec<IcSummary> = circuit.ics()
        .filter(|ic| !ic.is_virtual)
        .map(|ic| {
            let connected_nets: Vec<String> = circuit.nets_for_component(&ic.ref_des)
                .iter()
                .map(|n| n.net_name.clone())
                .collect();
            
            let power_nets: Vec<String> = circuit.nets_for_component(&ic.ref_des)
                .iter()
                .filter(|n| n.is_power_rail)
                .map(|n| {
                    if let Some(v) = n.voltage_level {
                        format!("{} ({}V)", n.net_name, v)
                    } else {
                        n.net_name.clone()
                    }
                })
                .collect();
            
            IcSummary {
                ref_des: ic.ref_des.clone(),
                value: ic.value.clone(),
                mpn: ic.mpn.clone(),
                power_nets,
                connected_net_count: connected_nets.len(),
                has_decoupling: decoupling.ic_decoupling
                    .get(&ic.ref_des)
                    .map(|caps| !caps.is_empty())
                    .unwrap_or(false),
            }
        })
        .collect();
    
    // Collect power rail information
    let power_rails: Vec<PowerRailSummary> = circuit.power_nets()
        .map(|net| {
            let components: Vec<String> = circuit.components_on_net(&net.net_name)
                .iter()
                .map(|c| c.ref_des.clone())
                .collect();
            
            PowerRailSummary {
                name: net.net_name.clone(),
                voltage: net.voltage_level,
                connected_component_count: components.len(),
            }
        })
        .collect();
    
    AiCircuitSummary {
        project_name: circuit.metadata.project_name.clone(),
        source_cad: circuit.metadata.source_cad.to_string(),
        component_count: stats.component_count,
        net_count: stats.net_count,
        ic_count: stats.ic_count,
        ics,
        power_rails,
        potential_issues: collect_potential_issues(&connectivity, &decoupling, &signal_integrity, &voltage),
    }
}

/// Summary of an IC for AI analysis
#[derive(Debug, Clone, serde::Serialize)]
pub struct IcSummary {
    pub ref_des: String,
    pub value: Option<String>,
    pub mpn: Option<String>,
    pub power_nets: Vec<String>,
    pub connected_net_count: usize,
    pub has_decoupling: bool,
}

/// Summary of a power rail for AI analysis
#[derive(Debug, Clone, serde::Serialize)]
pub struct PowerRailSummary {
    pub name: String,
    pub voltage: Option<f64>,
    pub connected_component_count: usize,
}

/// Complete circuit summary for AI analysis
#[derive(Debug, Clone, serde::Serialize)]
pub struct AiCircuitSummary {
    pub project_name: String,
    pub source_cad: String,
    pub component_count: usize,
    pub net_count: usize,
    pub ic_count: usize,
    pub ics: Vec<IcSummary>,
    pub power_rails: Vec<PowerRailSummary>,
    pub potential_issues: Vec<String>,
}

/// Collect potential issues from various analyses
fn collect_potential_issues(
    connectivity: &ConnectivityResult,
    decoupling: &DecouplingAnalysisResult,
    signal_integrity: &SignalIntegrityResult,
    voltage: &VoltagePropagationResult,
) -> Vec<String> {
    let mut issues = Vec::new();

    // Floating components
    for comp in &connectivity.floating_components {
        issues.push(format!("{} may not be connected to power/ground", comp));
    }

    // Missing decoupling
    for missing in &decoupling.missing_decoupling {
        issues.push(format!(
            "{} ({}) missing decoupling capacitor",
            missing.ic_ref,
            missing.ic_value.as_deref().unwrap_or("unknown")
        ));
    }

    // I2C without pull-ups
    for i2c in &signal_integrity.i2c_without_pullups {
        if !i2c.has_pullups {
            issues.push("I2C bus detected without pull-up resistors".to_string());
        }
    }

    // Unterminated high-speed signals
    for signal in &signal_integrity.unterminated_signals {
        issues.push(format!(
            "High-speed signal {} may need termination",
            signal.net_name
        ));
    }

    // Voltage issues
    for vi in &voltage.voltage_issues {
        issues.push(vi.message.clone());
    }

    issues
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ucs::schema::{UcsComponent, UcsNet, UcsPin, ElectricalType};

    // ====================================================================
    // Existing helper tests
    // ====================================================================

    #[test]
    fn test_is_decoupling_value() {
        assert!(is_decoupling_value("100nF"));
        assert!(is_decoupling_value("0.1uF"));
        assert!(is_decoupling_value("10uF"));
        assert!(!is_decoupling_value("10k"));
        assert!(!is_decoupling_value("1M"));
    }

    #[test]
    fn test_is_pullup_value() {
        assert!(is_pullup_value("4.7k"));
        assert!(is_pullup_value("4k7"));
        assert!(is_pullup_value("10k"));
        assert!(!is_pullup_value("100"));
        assert!(!is_pullup_value("1M"));
    }

    // ====================================================================
    // parse_voltage_from_name tests
    // ====================================================================

    #[test]
    fn test_parse_voltage_from_name() {
        assert_eq!(parse_voltage_from_name("+3V3"), Some(3.3));
        assert_eq!(parse_voltage_from_name("+5V0"), Some(5.0));
        assert_eq!(parse_voltage_from_name("+1V8"), Some(1.8));
        assert_eq!(parse_voltage_from_name("VCC_12V"), Some(12.0));
        assert_eq!(parse_voltage_from_name("5V"), Some(5.0));
        assert_eq!(parse_voltage_from_name("GND"), Some(0.0));
        assert_eq!(parse_voltage_from_name("VSS"), Some(0.0));
        assert_eq!(parse_voltage_from_name("AGND"), Some(0.0));
        assert_eq!(parse_voltage_from_name("SDA"), None);
        assert_eq!(parse_voltage_from_name("CLK"), None);
    }

    // ====================================================================
    // parse_resistance_value tests
    // ====================================================================

    #[test]
    fn test_parse_resistance_value() {
        assert!((parse_resistance_value("10k").unwrap() - 10_000.0).abs() < 1.0);
        assert!((parse_resistance_value("4.7k").unwrap() - 4_700.0).abs() < 1.0);
        assert!((parse_resistance_value("4k7").unwrap() - 4_700.0).abs() < 1.0);
        assert!((parse_resistance_value("100R").unwrap() - 100.0).abs() < 0.01);
        assert!((parse_resistance_value("1M").unwrap() - 1_000_000.0).abs() < 1.0);
        assert!((parse_resistance_value("470").unwrap() - 470.0).abs() < 0.01);
        assert!(parse_resistance_value("").is_none());
    }

    // ====================================================================
    // Voltage propagation integration tests
    // ====================================================================

    /// Build a simple test circuit:
    ///   VCC (3.3V) --[R1 10k]-- NET1 --[C1 100nF]-- GND (0V)
    fn build_simple_circuit() -> Circuit {
        let mut circuit = Circuit::new();

        // Components (must be added before nets so edges can connect)
        let mut r1 = UcsComponent::new("R1").with_value("10k");
        r1.add_pin(UcsPin::new("1").with_name("A").with_type(ElectricalType::Passive));
        r1.add_pin(UcsPin::new("2").with_name("B").with_type(ElectricalType::Passive));
        circuit.add_component(r1);

        let mut c1 = UcsComponent::new("C1").with_value("100nF");
        c1.add_pin(UcsPin::new("1").with_type(ElectricalType::Passive));
        c1.add_pin(UcsPin::new("2").with_type(ElectricalType::Passive));
        circuit.add_component(c1);

        // Nets (use add_net_with_connections to create graph edges)
        let mut vcc = UcsNet::new("VCC").with_voltage(3.3);
        vcc.add_connection("R1", "1");
        circuit.add_net_with_connections(vcc);

        let mut net1 = UcsNet::new("NET1");
        net1.add_connection("R1", "2");
        net1.add_connection("C1", "1");
        circuit.add_net_with_connections(net1);

        let mut gnd = UcsNet::new("GND").with_voltage(0.0);
        gnd.add_connection("C1", "2");
        circuit.add_net_with_connections(gnd);

        circuit
    }

    #[test]
    fn test_voltage_propagation_simple() {
        let circuit = build_simple_circuit();
        let result = analyze_voltage_propagation(&circuit);

        // VCC should be seeded at 3.3V
        assert_eq!(result.net_voltages.get("VCC"), Some(&3.3));
        // GND should be 0V
        assert_eq!(result.net_voltages.get("GND"), Some(&0.0));
        // NET1 should have a propagated voltage (through R1 from VCC, or through C1 from GND)
        assert!(result.net_voltages.contains_key("NET1"));
    }

    #[test]
    fn test_voltage_propagation_diode_drop() {
        let mut circuit = Circuit::new();

        let mut d1 = UcsComponent::new("D1").with_value("1N4148");
        d1.add_pin(UcsPin::new("A").with_name("Anode").with_type(ElectricalType::Passive));
        d1.add_pin(UcsPin::new("K").with_name("Cathode").with_type(ElectricalType::Passive));
        circuit.add_component(d1);

        let mut vin = UcsNet::new("+5V").with_voltage(5.0);
        vin.add_connection("D1", "A");
        circuit.add_net_with_connections(vin);

        let mut vout = UcsNet::new("VOUT");
        vout.add_connection("D1", "K");
        circuit.add_net_with_connections(vout);

        let result = analyze_voltage_propagation(&circuit);

        // VOUT should be ~4.3V (5.0 - 0.7)
        let vout_v = result.net_voltages.get("VOUT").copied().unwrap_or(0.0);
        assert!((vout_v - 4.3).abs() < 0.1, "Expected ~4.3V, got {}V", vout_v);
    }

    #[test]
    fn test_voltage_divider_detection() {
        let mut circuit = Circuit::new();

        // Two resistors forming a divider: VCC(5V) --[R1 10k]-- MID --[R2 10k]-- GND
        let mut r1 = UcsComponent::new("R1").with_value("10k");
        r1.add_pin(UcsPin::new("1").with_type(ElectricalType::Passive));
        r1.add_pin(UcsPin::new("2").with_type(ElectricalType::Passive));
        circuit.add_component(r1);

        let mut r2 = UcsComponent::new("R2").with_value("10k");
        r2.add_pin(UcsPin::new("1").with_type(ElectricalType::Passive));
        r2.add_pin(UcsPin::new("2").with_type(ElectricalType::Passive));
        circuit.add_component(r2);

        let mut vcc = UcsNet::new("+5V").with_voltage(5.0);
        vcc.add_connection("R1", "1");
        circuit.add_net_with_connections(vcc);

        let mut mid = UcsNet::new("MID");
        mid.add_connection("R1", "2");
        mid.add_connection("R2", "1");
        circuit.add_net_with_connections(mid);

        let mut gnd = UcsNet::new("GND").with_voltage(0.0);
        gnd.add_connection("R2", "2");
        circuit.add_net_with_connections(gnd);

        let result = analyze_voltage_propagation(&circuit);

        // MID should be ~2.5V (from BFS propagation or voltage divider detection)
        let mid_v = result.net_voltages.get("MID").copied().unwrap_or(-1.0);
        assert!(
            mid_v >= 0.0,
            "Expected a determined voltage at divider output, got {}V",
            mid_v
        );
    }

    #[test]
    fn test_regulator_blocks_propagation() {
        let mut circuit = Circuit::new();

        // U1 is a regulator
        let mut u1 = UcsComponent::new("U1").with_value("AMS1117-3.3");
        u1.add_pin(UcsPin::new("1").with_name("VIN").with_type(ElectricalType::PowerIn));
        u1.add_pin(UcsPin::new("2").with_name("GND").with_type(ElectricalType::PowerIn));
        u1.add_pin(UcsPin::new("3").with_name("VOUT").with_type(ElectricalType::PowerOut));
        circuit.add_component(u1);

        let mut vin = UcsNet::new("+5V").with_voltage(5.0);
        vin.add_connection("U1", "1");
        circuit.add_net_with_connections(vin);

        let mut vout = UcsNet::new("+3V3");
        vout.add_connection("U1", "3");
        circuit.add_net_with_connections(vout);

        let mut gnd = UcsNet::new("GND").with_voltage(0.0);
        gnd.add_connection("U1", "2");
        circuit.add_net_with_connections(gnd);

        let result = analyze_voltage_propagation(&circuit);

        // +3V3 should get 3.3V from net name heuristic,
        // NOT 5.0V propagated from VIN (ICs block propagation).
        let v3v3 = result.net_voltages.get("+3V3").copied().unwrap_or(0.0);
        assert!(
            (v3v3 - 3.3).abs() < 0.1,
            "Expected ~3.3V (net name heuristic), got {}V — input voltage should not propagate through IC",
            v3v3
        );
    }

    #[test]
    fn test_net_name_heuristic_seeding() {
        let mut circuit = Circuit::new();

        // A standalone net with a voltage-like name
        let net = UcsNet::new("+1V8");
        circuit.add_net_with_connections(net);

        let result = analyze_voltage_propagation(&circuit);
        assert_eq!(result.net_voltages.get("+1V8"), Some(&1.8));
    }
}
