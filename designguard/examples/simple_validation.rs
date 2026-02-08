//! Simple validation example: validate a schematic and print results.

use designguard::prelude::*;
use std::path::Path;

fn main() -> Result<(), DesignGuardError> {
    let path = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "examples/test.kicad_sch".to_string());
    let path = Path::new(&path);

    if !path.exists() {
        eprintln!("File not found: {}", path.display());
        eprintln!("Usage: cargo run --example simple_validation [path/to/file.kicad_sch]");
        std::process::exit(1);
    }

    let options = ValidationOptions {
        enable_ai: false,
        offline_mode: true,
        strict_mode: false,
        rules: vec![],
    };

    let result = DesignGuardCore::validate_schematic(path, options)?;

    println!("Validation results for: {}", result.file.display());
    println!("Total issues: {}", result.total_issues());
    println!();

    if result.stats.critical > 0 {
        println!("CRITICAL issues:");
        for issue in result.issues.iter().filter(|i| matches!(i.severity, Severity::Error)) {
            println!("  - {}", issue.message);
            if let Some(ref component) = issue.component {
                println!("    Component: {}", component);
            }
        }
    }

    if result.has_critical() {
        println!("\nValidation failed (critical issues).");
        std::process::exit(1);
    }

    println!("\nValidation passed (no critical issues).");
    Ok(())
}
