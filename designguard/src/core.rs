//! Core validation logic shared by GUI and CLI.
//! No Tauri or app state dependencies.

use std::path::{Path, PathBuf};

use crate::analyzer::rules::{Issue, RuleContext, RulesEngine, Severity};
use crate::compliance::emi::generate_emi_report;
use crate::datasheets::checker::DatasheetChecker;
use crate::parser::kicad::KicadParser;
use crate::parser::netlist::NetlistBuilder;
use crate::parser::pcb::PcbParser;
use crate::parser::schema::Schematic;
use crate::ucs::adapters::{CircuitAdapter, KicadAdapter};
use crate::analyzer::capacitor_classifier::CapacitorClassifier;
use crate::analyzer::decoupling_groups::DecouplingGroupsAnalyzer;
use crate::compliance::power_net_registry::PowerNetRegistry;

#[derive(Debug, thiserror::Error)]
pub enum DesignGuardError {
    #[error("Parse error: {0}")]
    Parse(String),
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("{0}")]
    Other(String),
}

impl From<crate::parser::kicad::KicadParseError> for DesignGuardError {
    fn from(e: crate::parser::kicad::KicadParseError) -> Self {
        DesignGuardError::Parse(e.to_string())
    }
}

impl From<crate::parser::pcb::PcbParseError> for DesignGuardError {
    fn from(e: crate::parser::pcb::PcbParseError) -> Self {
        DesignGuardError::Parse(e.to_string())
    }
}

/// Options for validation runs (CLI or GUI).
#[derive(Clone, Debug)]
pub struct ValidationOptions {
    pub enable_ai: bool,
    pub offline_mode: bool,
    pub strict_mode: bool,
    pub rules: Vec<String>,
}

impl Default for ValidationOptions {
    fn default() -> Self {
        Self {
            enable_ai: true,
            offline_mode: false,
            strict_mode: false,
            rules: vec![],
        }
    }
}

/// Per-file validation result with issues and counts.
#[derive(Debug, Clone)]
pub struct ValidationResult {
    pub file: PathBuf,
    pub issues: Vec<Issue>,
    pub stats: ValidationStats,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct ValidationStats {
    pub critical: usize,
    pub high: usize,
    pub medium: usize,
    pub low: usize,
    pub info: usize,
}

impl ValidationResult {
    pub fn has_critical(&self) -> bool {
        self.stats.critical > 0
    }

    pub fn has_high_or_critical(&self) -> bool {
        self.stats.critical > 0 || self.stats.high > 0
    }

    pub fn total_issues(&self) -> usize {
        self.stats.critical
            + self.stats.high
            + self.stats.medium
            + self.stats.low
            + self.stats.info
    }
}

fn issues_to_stats(issues: &[Issue]) -> ValidationStats {
    let mut critical = 0;
    let mut high = 0;
    let mut medium = 0;
    let mut low = 0;
    let info = 0;
    for i in issues {
        match i.severity {
            Severity::Error => critical += 1,
            Severity::Warning => high += 1,
            Severity::Suggestion => medium += 1,
            Severity::Info => low += 1,
        }
    }
    ValidationStats {
        critical,
        high,
        medium,
        low,
        info,
    }
}

/// Recursively discover KiCAD schematic and PCB files in a directory.
pub fn discover_kicad_files(dir: &Path) -> Result<Vec<PathBuf>, DesignGuardError> {
    let mut files = Vec::new();
    walk_dir(dir, &mut files, 0)?;
    Ok(files)
}

fn walk_dir(dir: &Path, files: &mut Vec<PathBuf>, depth: usize) -> Result<(), DesignGuardError> {
    if depth > 20 {
        return Ok(());
    }
    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
            if name.starts_with('.') || name == "node_modules" || name == "target" || name == "build" {
                continue;
            }
            walk_dir(&path, files, depth + 1)?;
        } else if path.is_file() {
            if let Some(ext) = path.extension().and_then(|s| s.to_str()) {
                match ext {
                    "kicad_sch" | "sch" | "kicad_pcb" | "brd" => files.push(path),
                    _ => {}
                }
            }
        }
    }
    Ok(())
}

/// Core validation API used by both GUI and CLI.
pub struct DesignGuardCore;

impl DesignGuardCore {
    /// Validate a single schematic file.
    pub fn validate_schematic(
        path: &Path,
        options: ValidationOptions,
    ) -> Result<ValidationResult, DesignGuardError> {
        let schematic = KicadParser::parse_schematic(path)?;
        let mut issues = Vec::new();

        let engine = RulesEngine::with_default_rules();
        let context = build_rule_context(path, &schematic);
        issues.extend(engine.analyze_enhanced(&schematic, Some(&context)));

        if !options.offline_mode {
            let checker = DatasheetChecker::new();
            issues.extend(checker.check_as_issues(&schematic));
        }

        let stats = issues_to_stats(&issues);
        Ok(ValidationResult {
            file: path.to_path_buf(),
            issues,
            stats,
        })
    }

    /// Validate a single PCB file (EMI and high-level checks; no schematic DRC).
    pub fn validate_pcb(
        path: &Path,
        _options: ValidationOptions,
    ) -> Result<ValidationResult, DesignGuardError> {
        let pcb = PcbParser::parse_pcb(path)?;
        let emi_report = generate_emi_report(&pcb);
        let issues: Vec<Issue> = emi_report
            .issues
            .iter()
            .map(|e| {
                let severity = match e.severity {
                    crate::compliance::emi::EmiSeverity::Critical => Severity::Error,
                    crate::compliance::emi::EmiSeverity::High => Severity::Warning,
                    crate::compliance::emi::EmiSeverity::Medium => Severity::Suggestion,
                    crate::compliance::emi::EmiSeverity::Low
                    | crate::compliance::emi::EmiSeverity::Info => Severity::Info,
                };
                Issue {
                    id: e.id.clone(),
                    rule_id: "emi".to_string(),
                    severity,
                    message: format!("{} {}", e.message, e.recommendation),
                    component: Some(e.net_name.clone()),
                    location: None,
                    suggestion: None,
                    risk_score: None,
                }
            })
            .collect();
        let stats = issues_to_stats(&issues);
        Ok(ValidationResult {
            file: path.to_path_buf(),
            issues,
            stats,
        })
    }

    /// Validate all KiCAD files in a directory (schematics and PCBs).
    pub fn validate_project(
        dir: &Path,
        options: ValidationOptions,
    ) -> Result<Vec<ValidationResult>, DesignGuardError> {
        let files = discover_kicad_files(dir)?;
        let mut results = Vec::new();
        for path in files {
            let ext = path.extension().and_then(|s| s.to_str());
            match ext {
                Some("kicad_sch") | Some("sch") => {
                    match Self::validate_schematic(&path, options.clone()) {
                        Ok(r) => results.push(r),
                        Err(e) => return Err(e),
                    }
                }
                Some("kicad_pcb") | Some("brd") => {
                    match Self::validate_pcb(&path, options.clone()) {
                        Ok(r) => results.push(r),
                        Err(e) => return Err(e),
                    }
                }
                _ => {}
            }
        }
        Ok(results)
    }
}

fn build_rule_context(path: &Path, schematic: &Schematic) -> RuleContext {
    let pin_to_net = NetlistBuilder::build_netlist(schematic);
    let power_registry = PowerNetRegistry::new(schematic);
    let classifications =
        CapacitorClassifier::classify_capacitors(schematic, &power_registry, &pin_to_net);

    let decoupling_groups = if let Ok(circuit) = KicadAdapter::new().parse_to_circuit(path) {
        let groups = DecouplingGroupsAnalyzer::build_groups_from_circuit(&circuit, &classifications);
        if groups.is_empty() {
            DecouplingGroupsAnalyzer::build_groups(
                schematic,
                &power_registry,
                &classifications,
                &pin_to_net,
            )
        } else {
            groups
        }
    } else {
        DecouplingGroupsAnalyzer::build_groups(
            schematic,
            &power_registry,
            &classifications,
            &pin_to_net,
        )
    };

    RuleContext {
        capacitor_classifications: classifications,
        decoupling_groups,
        power_registry,
        pcb: None,
    }
}
