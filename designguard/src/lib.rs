//! DesignGuard - KiCad schematic and PCB validation library
//!
//! This library provides validation rules for KiCad designs, checking for
//! common mistakes like missing decoupling capacitors, incorrect pull-up
//! resistors, and trace width issues.
//!
//! # Quick Start
//!
//! ```no_run
//! use designguard::{DesignGuardCore, ValidationOptions};
//! use std::path::Path;
//!
//! let options = ValidationOptions::default();
//! let result = DesignGuardCore::validate_schematic(
//!     Path::new("design.kicad_sch"),
//!     options,
//! ).unwrap();
//!
//! for issue in &result.issues {
//!     println!("{:?}: {}", issue.severity, issue.message);
//! }
//! ```
//!
//! # Features
//!
//! - **Schematic validation**: Decoupling caps, pull-ups, power integrity
//! - **PCB validation**: Trace widths, EMI checks, IPC-2221 compliance
//! - **Datasheet checking**: Component-specific validation
//! - **Optional AI**: Ollama/Claude integration (used by GUI/CLI)

pub mod analyzer;
pub mod compliance;
pub mod core;
pub mod datasheets;
pub mod parser;
pub mod ucs;
pub mod ai;

// Re-export main types
pub use core::{
    DesignGuardError, DesignGuardCore, ValidationOptions, ValidationResult, ValidationStats,
    discover_kicad_files,
};
pub use analyzer::rules::{Issue, Severity, RulesEngine};
pub use parser::schema::Schematic;
pub use parser::pcb_schema::PcbDesign;
pub use parser::kicad::KicadParser;
pub use parser::pcb::PcbParser;

/// Parse a schematic file (convenience wrapper).
pub fn parse_schematic(path: &std::path::Path) -> Result<Schematic, DesignGuardError> {
    KicadParser::parse_schematic(path).map_err(|e| DesignGuardError::Parse(e.to_string()))
}

/// Parse a PCB file (convenience wrapper).
pub fn parse_pcb(
    path: &std::path::Path,
) -> Result<PcbDesign, DesignGuardError> {
    PcbParser::parse_pcb(path).map_err(|e| DesignGuardError::Parse(e.to_string()))
}

/// Prelude for convenient imports.
pub mod prelude {
    pub use crate::{
        DesignGuardCore, DesignGuardError, Issue, Severity, ValidationOptions, ValidationResult,
        ValidationStats,
    };
}
