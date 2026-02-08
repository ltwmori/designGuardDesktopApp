use crate::analyzer::rules::{Issue, Severity};
use crate::parser::schema::*;
use std::collections::HashSet;

pub fn build_analysis_prompt(schematic: &Schematic, existing_issues: &[Issue]) -> String {
    let components_summary = summarize_components(schematic);
    let power_rails = summarize_power_rails(schematic);
    let signal_nets = summarize_signal_nets(schematic);
    let detected_issues = summarize_issues(existing_issues);

    format!(
        r#"You are an expert PCB design engineer reviewing a KiCAD schematic.

Schematic Context:
Components: {}
Power Rails: {}
Signal Nets: {}
Existing Issues: {}

Analyze this schematic and provide a comprehensive review. Focus on:
1. Circuit functionality and design correctness
2. Component selection and values
3. Power distribution and decoupling
4. Signal integrity considerations
5. Best practices and potential improvements

Respond ONLY with valid JSON in this exact format (no markdown, no code blocks, just pure JSON):
{{
  "summary": "Brief one-sentence circuit description",
  "circuit_description": "Detailed explanation of what this circuit does and how it works",
  "potential_issues": ["Issue 1", "Issue 2"],
  "improvement_suggestions": ["Suggestion 1", "Suggestion 2"],
  "component_recommendations": [
    {{
      "component": "R1",
      "current_value": "10k",
      "suggested_value": "4.7k",
      "reason": "Better impedance matching for this application"
    }}
  ]
}}

Important: Return ONLY the JSON object, nothing else."#,
        components_summary, power_rails, signal_nets, detected_issues
    )
}

pub fn build_question_prompt(schematic: &Schematic, question: &str) -> String {
    let components_summary = summarize_components(schematic);
    let power_rails = summarize_power_rails(schematic);

    format!(
        r#"You are an expert PCB design engineer. You are reviewing a KiCAD schematic.

Schematic Context:
Components: {}
Power Rails: {}

Question: {}

Please provide a detailed, technical answer based on the schematic context. If the question cannot be answered from the available information, please state that clearly."#,
        components_summary, power_rails, question
    )
}

fn summarize_components(schematic: &Schematic) -> String {
    let mut summary = String::new();
    
    // Count components by type
    let mut ic_count = 0;
    let mut resistor_count = 0;
    let mut capacitor_count = 0;
    let mut inductor_count = 0;
    let mut diode_count = 0;
    let mut connector_count = 0;
    let mut other_count = 0;
    
    let all_components = get_all_components(schematic);
    
    for component in &all_components {
        let ref_upper = component.reference.to_uppercase();
        if ref_upper.starts_with('U') {
            ic_count += 1;
            if summary.len() < 500 {
                summary.push_str(&format!("{} ({}) ", component.reference, component.value));
            }
        } else if ref_upper.starts_with('R') {
            resistor_count += 1;
        } else if ref_upper.starts_with('C') {
            capacitor_count += 1;
        } else if ref_upper.starts_with('L') {
            inductor_count += 1;
        } else if ref_upper.starts_with('D') {
            diode_count += 1;
        } else if ref_upper.starts_with('J') || ref_upper.starts_with('P') {
            connector_count += 1;
        } else {
            other_count += 1;
        }
    }
    
    format!(
        "Total: {} components ({} ICs, {} resistors, {} capacitors, {} inductors, {} diodes, {} connectors, {} other). Key ICs: {}",
        all_components.len(),
        ic_count,
        resistor_count,
        capacitor_count,
        inductor_count,
        diode_count,
        connector_count,
        other_count,
        if summary.is_empty() { "None listed".to_string() } else { summary.trim().to_string() }
    )
}

fn summarize_power_rails(schematic: &Schematic) -> String {
    let mut rails = HashSet::new();
    
    // Check power symbols
    for component in &schematic.power_symbols {
        rails.insert(component.value.clone());
    }
    
    // Check labels
    for label in &schematic.labels {
        let label_upper = label.text.to_uppercase();
        if label_upper.starts_with("V") || 
           label_upper == "GND" || 
           label_upper == "GROUND" ||
           label_upper == "VSS" ||
           label_upper == "VDD" ||
           label_upper == "VCC" {
            rails.insert(label.text.clone());
        }
    }
    
    // Check nets
    for net in &schematic.nets {
        let net_upper = net.name.to_uppercase();
        if net_upper.starts_with("V") || 
           net_upper == "GND" || 
           net_upper == "GROUND" {
            rails.insert(net.name.clone());
        }
    }
    
    if rails.is_empty() {
        "No power rails detected".to_string()
    } else {
        let rails_vec: Vec<&str> = rails.iter().map(|s| s.as_str()).collect();
        format!("Power rails: {}", rails_vec.join(", "))
    }
}

fn summarize_signal_nets(schematic: &Schematic) -> String {
    let mut signal_nets = Vec::new();
    
    // Collect signal nets from labels
    for label in &schematic.labels {
        let label_upper = label.text.to_uppercase();
        if !label_upper.starts_with("V") && 
           label_upper != "GND" && 
           label_upper != "GROUND" &&
           label_upper != "VSS" &&
           label_upper != "VDD" &&
           label_upper != "VCC" {
            signal_nets.push(label.text.clone());
        }
    }
    
    // Collect from nets
    for net in &schematic.nets {
        let net_upper = net.name.to_uppercase();
        if !net_upper.starts_with("V") && 
           net_upper != "GND" && 
           net_upper != "GROUND" {
            signal_nets.push(net.name.clone());
        }
    }
    
    if signal_nets.is_empty() {
        "No signal nets identified".to_string()
    } else {
        let unique_nets: Vec<&str> = signal_nets.iter().take(10).map(|s| s.as_str()).collect();
        format!("Signal nets: {} (showing first 10)", unique_nets.join(", "))
    }
}

fn summarize_issues(issues: &[Issue]) -> String {
    if issues.is_empty() {
        return "No issues detected by automated checks".to_string();
    }
    
    let mut summary = format!("{} issues found: ", issues.len());
    let mut issue_types = std::collections::HashMap::new();
    
    for issue in issues {
        *issue_types.entry(issue.severity.clone()).or_insert(0) += 1;
    }
    
    let mut parts = Vec::new();
    if let Some(count) = issue_types.get(&Severity::Error) {
        parts.push(format!("{} errors", count));
    }
    if let Some(count) = issue_types.get(&Severity::Warning) {
        parts.push(format!("{} warnings", count));
    }
    if let Some(count) = issue_types.get(&Severity::Info) {
        parts.push(format!("{} info", count));
    }
    
    summary.push_str(&parts.join(", "));
    
    // Add first few issue messages
    for issue in issues.iter().take(3) {
        summary.push_str(&format!(". {}: {}", issue.rule_id, issue.message));
    }
    
    summary
}

fn get_all_components(schematic: &Schematic) -> Vec<&Component> {
    let mut all = Vec::new();
    all.extend(&schematic.components);
    all.extend(&schematic.power_symbols);
    all
}
