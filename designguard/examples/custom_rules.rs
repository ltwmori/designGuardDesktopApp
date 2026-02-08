//! Example: using RulesEngine and parser directly (without DesignGuardCore).
//! Run with: cargo run --example custom_rules [path/to/file.kicad_sch]

use designguard::{parse_schematic, RulesEngine, Severity};
use std::path::Path;

fn main() -> Result<(), designguard::DesignGuardError> {
    let path = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "examples/test.kicad_sch".to_string());
    let path = Path::new(&path);

    if !path.exists() {
        eprintln!("File not found: {}", path.display());
        eprintln!("Usage: cargo run --example custom_rules [path/to/file.kicad_sch]");
        std::process::exit(1);
    }

    let schematic = parse_schematic(path)?;
    let engine = RulesEngine::with_default_rules();
    let issues = engine.analyze(&schematic);

    println!("Custom validation found {} issues for {}", issues.len(), path.display());
    for issue in &issues {
        println!("  [{:?}] {}", issue.severity, issue.message);
        if let Some(ref comp) = issue.component {
            println!("    Component: {}", comp);
        }
    }

    let critical = issues.iter().filter(|i| matches!(i.severity, Severity::Error)).count();
    if critical > 0 {
        std::process::exit(1);
    }
    Ok(())
}
