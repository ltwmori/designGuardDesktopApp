//! DesignGuard CLI - KiCAD schematic and PCB validation from the command line.

use clap::{Parser, Subcommand, ValueEnum};
use designguard::{
    DesignGuardCore, Issue, Severity, ValidationOptions, ValidationResult,
};
use std::path::PathBuf;
use std::process;

#[derive(Parser)]
#[command(name = "designguard")]
#[command(about = "KiCAD schematic and PCB validation tool", long_about = None)]
#[command(version)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Validate a single schematic or PCB file
    Check {
        /// Path to .kicad_sch or .kicad_pcb file
        #[arg(value_name = "FILE")]
        file: PathBuf,

        /// Output format
        #[arg(short, long, value_enum, default_value = "human")]
        format: OutputFormat,

        /// Exit with error code if issues found at this severity or higher
        #[arg(long, value_enum)]
        fail_on: Option<FailOnSeverity>,

        /// Disable AI/datasheet features (offline mode)
        #[arg(long)]
        no_ai: bool,

        /// Enable strict validation (all rules)
        #[arg(long)]
        strict: bool,
    },

    /// Validate all KiCAD files in a directory
    Project {
        /// Path to project directory
        #[arg(value_name = "DIR", default_value = ".")]
        dir: PathBuf,

        /// Output format
        #[arg(short, long, value_enum, default_value = "human")]
        format: OutputFormat,

        /// Exit with error code if issues found at this severity or higher
        #[arg(long, value_enum)]
        fail_on: Option<FailOnSeverity>,

        /// Disable AI/datasheet features (offline mode)
        #[arg(long)]
        no_ai: bool,

        /// Enable strict validation (all rules)
        #[arg(long)]
        strict: bool,
    },

    /// List available validation rules
    Rules {
        /// Show detailed rule descriptions
        #[arg(short, long)]
        verbose: bool,
    },
}

#[derive(Clone, ValueEnum)]
enum OutputFormat {
    /// Human-readable output
    Human,
    /// JSON output for CI/CD
    Json,
    /// GitHub Actions format
    Github,
    /// GitLab CI format
    Gitlab,
}

#[derive(Clone, ValueEnum)]
enum FailOnSeverity {
    Critical,
    High,
    Medium,
    Low,
    Info,
}

fn main() {
    let cli = Cli::parse();

    let exit_code = match cli.command {
        Commands::Check {
            file,
            format,
            fail_on,
            no_ai,
            strict,
        } => handle_check(&file, format, fail_on, no_ai, strict),
        Commands::Project {
            dir,
            format,
            fail_on,
            no_ai,
            strict,
        } => handle_project(&dir, format, fail_on, no_ai, strict),
        Commands::Rules { verbose } => {
            handle_rules(verbose);
            0
        }
    };

    process::exit(exit_code);
}

fn handle_check(
    file: &PathBuf,
    format: OutputFormat,
    fail_on: Option<FailOnSeverity>,
    no_ai: bool,
    strict: bool,
) -> i32 {
    let options = ValidationOptions {
        enable_ai: !no_ai,
        offline_mode: no_ai,
        strict_mode: strict,
        rules: vec![],
    };

    let ext = file.extension().and_then(|s| s.to_str());
    let result = match ext {
        Some("kicad_sch") | Some("sch") => DesignGuardCore::validate_schematic(file, options),
        Some("kicad_pcb") | Some("brd") => DesignGuardCore::validate_pcb(file, options),
        _ => {
            eprintln!("Error: File must be .kicad_sch, .sch, .kicad_pcb, or .brd");
            return 1;
        }
    };

    match result {
        Ok(validation) => {
            output_results(&[validation.clone()], &format);
            if let Some(severity) = fail_on {
                if should_fail(&validation, &severity) {
                    return 1;
                }
            }
            0
        }
        Err(e) => {
            eprintln!("Error: {}", e);
            1
        }
    }
}

fn handle_project(
    dir: &PathBuf,
    format: OutputFormat,
    fail_on: Option<FailOnSeverity>,
    no_ai: bool,
    strict: bool,
) -> i32 {
    let options = ValidationOptions {
        enable_ai: !no_ai,
        offline_mode: no_ai,
        strict_mode: strict,
        rules: vec![],
    };

    match DesignGuardCore::validate_project(dir, options) {
        Ok(results) => {
            output_results(&results, &format);
            if let Some(severity) = fail_on {
                for result in &results {
                    if should_fail(result, &severity) {
                        return 1;
                    }
                }
            }
            0
        }
        Err(e) => {
            eprintln!("Error: {}", e);
            1
        }
    }
}

fn should_fail(result: &ValidationResult, severity: &FailOnSeverity) -> bool {
    match severity {
        FailOnSeverity::Critical => result.stats.critical > 0,
        FailOnSeverity::High => result.has_high_or_critical(),
        FailOnSeverity::Medium => {
            result.stats.critical > 0
                || result.stats.high > 0
                || result.stats.medium > 0
        }
        FailOnSeverity::Low | FailOnSeverity::Info => result.total_issues() > 0,
    }
}

fn output_results(results: &[ValidationResult], format: &OutputFormat) {
    match format {
        OutputFormat::Human => output_human(results),
        OutputFormat::Json => output_json(results),
        OutputFormat::Github => output_github(results),
        OutputFormat::Gitlab => output_gitlab(results),
    }
}

fn output_human(results: &[ValidationResult]) {
    for result in results {
        println!("\nFile: {}", result.file.display());
        println!("{}", "─".repeat(60));

        if result.total_issues() == 0 {
            println!("  No issues found");
            continue;
        }

        let critical: Vec<_> = result
            .issues
            .iter()
            .filter(|i| matches!(i.severity, Severity::Error))
            .collect();
        let high: Vec<_> = result
            .issues
            .iter()
            .filter(|i| matches!(i.severity, Severity::Warning))
            .collect();
        let medium: Vec<_> = result
            .issues
            .iter()
            .filter(|i| matches!(i.severity, Severity::Suggestion))
            .collect();
        let low: Vec<_> = result
            .issues
            .iter()
            .filter(|i| matches!(i.severity, Severity::Info))
            .collect();

        if !critical.is_empty() {
            println!("\n  CRITICAL:");
            for issue in critical {
                println!("    - {}", issue.message);
                if let Some(ref comp) = issue.component {
                    println!("      Component: {}", comp);
                }
            }
        }
        if !high.is_empty() {
            println!("\n  HIGH:");
            for issue in high {
                println!("    - {}", issue.message);
                if let Some(ref comp) = issue.component {
                    println!("      Component: {}", comp);
                }
            }
        }
        if !medium.is_empty() {
            println!("\n  MEDIUM:");
            for issue in medium {
                println!("    - {}", issue.message);
            }
        }
        if !low.is_empty() {
            println!("\n  LOW:");
            for issue in low {
                println!("    - {}", issue.message);
            }
        }

        println!("\n  Summary:");
        println!("    Critical: {}", result.stats.critical);
        println!("    High:     {}", result.stats.high);
        println!("    Medium:   {}", result.stats.medium);
        println!("    Low:      {}", result.stats.low);
        println!("    Info:     {}", result.stats.info);
    }
}

fn output_json(results: &[ValidationResult]) {
    let output = serde_json::json!({
        "results": results.iter().map(|r| {
            serde_json::json!({
                "file": r.file.display().to_string(),
                "issues": r.issues,
                "stats": {
                    "critical": r.stats.critical,
                    "high": r.stats.high,
                    "medium": r.stats.medium,
                    "low": r.stats.low,
                    "info": r.stats.info,
                }
            })
        }).collect::<Vec<_>>(),
        "summary": {
            "total_files": results.len(),
            "total_issues": results.iter().map(|r| r.total_issues()).sum::<usize>(),
            "critical": results.iter().map(|r| r.stats.critical).sum::<usize>(),
        }
    });
    println!("{}", serde_json::to_string_pretty(&output).unwrap());
}

fn severity_to_github(issue: &Issue) -> &'static str {
    match issue.severity {
        Severity::Error | Severity::Warning => "error",
        Severity::Suggestion => "warning",
        Severity::Info => "notice",
    }
}

fn output_github(results: &[ValidationResult]) {
    for result in results {
        for issue in &result.issues {
            let level = severity_to_github(issue);
            println!(
                "::{} file={}::{}",
                level,
                result.file.display(),
                issue.message.replace('\n', " ")
            );
        }
    }
}

fn severity_to_gitlab(issue: &Issue) -> &'static str {
    match issue.severity {
        Severity::Error => "blocker",
        Severity::Warning => "major",
        Severity::Suggestion => "minor",
        Severity::Info => "info",
    }
}

fn output_gitlab(results: &[ValidationResult]) {
    let mut reports = Vec::new();
    for result in results {
        for issue in &result.issues {
            reports.push(serde_json::json!({
                "description": issue.message,
                "severity": severity_to_gitlab(issue),
                "location": {
                    "path": result.file.display().to_string(),
                }
            }));
        }
    }
    println!("{}", serde_json::to_string_pretty(&reports).unwrap());
}

fn handle_rules(verbose: bool) {
    println!("Available validation rules:\n");

    let rules = [
        (
            "decoupling_capacitor",
            "Decoupling capacitor validation",
            "Checks for 100nF bypass caps near IC power pins",
        ),
        (
            "i2c_pull_resistor",
            "I2C pull-up resistors",
            "Validates 2.2k-10k pull-ups on SDA/SCL",
        ),
        (
            "crystal_load_capacitor",
            "Crystal load capacitors",
            "Checks for 10-33pF paired caps",
        ),
        (
            "power_pin",
            "Power pin validation",
            "Ensures GND symbols and IC power connections",
        ),
        (
            "esd_protection",
            "ESD protection",
            "Requires TVS diodes on USB/Ethernet",
        ),
        (
            "bulk_capacitor",
            "Bulk capacitors",
            "Validates 10-100uF caps on regulators",
        ),
        (
            "emi",
            "EMI (PCB)",
            "High-speed trace and reference plane checks",
        ),
        (
            "datasheet",
            "Datasheet compliance",
            "Validates against component datasheets (use without --no-ai)",
        ),
    ];

    for (name, short, long) in &rules {
        println!("  {}", name);
        println!("    {}", short);
        if verbose {
            println!("    {}", long);
        }
        println!();
    }
}
