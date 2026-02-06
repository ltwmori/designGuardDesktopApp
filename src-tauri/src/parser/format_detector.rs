//! KiCad Format Detection and Routing
//!
//! This module detects KiCad file format versions and routes parsing to the appropriate parser.
//! Supports KiCad versions 4 through 9.

use std::path::Path;
use crate::parser::schema::Schematic;
use crate::parser::pcb_schema::PcbDesign;
use crate::parser::kicad::{KicadParser, KicadParseError};
use crate::parser::pcb::{PcbParser, PcbParseError};
use crate::parser::kicad_legacy::LegacyParser;

/// KiCad file format version
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KicadVersion {
    /// KiCad 4.0 - Legacy text format
    Legacy4,
    /// KiCad 5.0 - Legacy text format
    Legacy5,
    /// KiCad 6.0 - Modern S-expression format
    Modern6,
    /// KiCad 7.0 - Modern S-expression format
    Modern7,
    /// KiCad 8.0 - Modern S-expression format
    Modern8,
    /// KiCad 9.0 - Modern S-expression format
    Modern9,
}

impl KicadVersion {
    /// Get version string for display
    pub fn as_str(&self) -> &'static str {
        match self {
            KicadVersion::Legacy4 => "KiCad 4",
            KicadVersion::Legacy5 => "KiCad 5",
            KicadVersion::Modern6 => "KiCad 6",
            KicadVersion::Modern7 => "KiCad 7",
            KicadVersion::Modern8 => "KiCad 8",
            KicadVersion::Modern9 => "KiCad 9",
        }
    }

    /// Check if this is a legacy format
    pub fn is_legacy(&self) -> bool {
        matches!(self, KicadVersion::Legacy4 | KicadVersion::Legacy5)
    }

    /// Check if this is a modern format
    pub fn is_modern(&self) -> bool {
        !self.is_legacy()
    }
}

/// Detect KiCad file format version from file content
pub fn detect_format(content: &str) -> Option<KicadVersion> {
    let trimmed = content.trim_start();
    
    // Check for legacy schematic format
    if trimmed.starts_with("EESchema Schematic File Version") {
        if trimmed.contains("Version 4") {
            return Some(KicadVersion::Legacy4);
        } else if trimmed.contains("Version 5") {
            return Some(KicadVersion::Legacy5);
        }
        // Default to Legacy5 if version string is present but version number unclear
        return Some(KicadVersion::Legacy5);
    }
    
    // Check for legacy PCB format
    if trimmed.starts_with("PCBNEW") {
        // Legacy PCB format - default to Legacy5 (most common)
        // Could parse version from file if needed
        return Some(KicadVersion::Legacy5);
    }
    
    // Check for modern S-expression format
    if trimmed.starts_with("(kicad_sch") {
        // Extract version from file content
        return detect_modern_version(content);
    }
    
    if trimmed.starts_with("(kicad_pcb") {
        // Extract version from file content
        return detect_modern_version(content);
    }
    
    None
}

/// Detect modern format version (6-9) from S-expression content
fn detect_modern_version(content: &str) -> Option<KicadVersion> {
    // Look for version field in the file
    // Format: (version YYYYMMDD) or (generator "eeschema" "version")
    // Version dates:
    // - 20211014: KiCad 6.0
    // - 20221118: KiCad 7.0
    // - 20240208: KiCad 8.0
    // - 20241017: KiCad 9.0
    
    // Try to extract version number
    if let Some(version_start) = content.find("(version ") {
        let version_section = &content[version_start..version_start.min(content.len()).saturating_add(50)];
        if let Some(version_end) = version_section.find(')') {
            let version_str = &version_section[9..version_end].trim();
            if let Ok(version_num) = version_str.parse::<u32>() {
                // Determine version based on date
                if version_num >= 20241017 {
                    return Some(KicadVersion::Modern9);
                } else if version_num >= 20240208 {
                    return Some(KicadVersion::Modern8);
                } else if version_num >= 20221118 {
                    return Some(KicadVersion::Modern7);
                } else if version_num >= 20211014 {
                    return Some(KicadVersion::Modern6);
                }
            }
        }
    }
    
    // Default to Modern6 if we can't determine (backward compatibility)
    Some(KicadVersion::Modern6)
}

/// Detect and parse a schematic file, automatically routing to the correct parser
pub fn detect_and_parse_schematic(path: &Path) -> Result<Schematic, KicadParseError> {
    let content = std::fs::read_to_string(path)
        .map_err(|e| KicadParseError::Io(e))?;
    
    let version = detect_format(&content)
        .ok_or_else(|| KicadParseError::InvalidFormat(
            "Could not detect KiCad file format. Expected EESchema, PCBNEW, or (kicad_sch/(kicad_pcb header.".to_string()
        ))?;
    
    let filename = path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("unknown")
        .to_string();
    
    match version {
        KicadVersion::Legacy4 | KicadVersion::Legacy5 => {
            LegacyParser::parse_legacy_schematic(&content, &filename)
                .map_err(|e| KicadParseError::InvalidFormat(format!("Legacy format parse error: {}", e)))
        }
        KicadVersion::Modern6 | KicadVersion::Modern7 | KicadVersion::Modern8 | KicadVersion::Modern9 => {
            KicadParser::parse_schematic_str(&content, &filename)
        }
    }
}

/// Detect and parse a PCB file, automatically routing to the correct parser
pub fn detect_and_parse_pcb(path: &Path) -> Result<PcbDesign, PcbParseError> {
    let content = std::fs::read_to_string(path)
        .map_err(|e| PcbParseError::Io(e))?;
    
    let version = detect_format(&content)
        .ok_or_else(|| PcbParseError::InvalidFormat(
            "Could not detect KiCad file format. Expected PCBNEW or (kicad_pcb header.".to_string()
        ))?;
    
    let filename = path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("unknown")
        .to_string();
    
    match version {
        KicadVersion::Legacy4 | KicadVersion::Legacy5 => {
            LegacyParser::parse_legacy_pcb(&content, &filename)
                .map_err(|e| PcbParseError::InvalidFormat(format!("Legacy format parse error: {}", e)))
        }
        KicadVersion::Modern6 | KicadVersion::Modern7 | KicadVersion::Modern8 | KicadVersion::Modern9 => {
            PcbParser::parse_pcb_str(&content, &filename)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_legacy_schematic_v4() {
        let content = "EESchema Schematic File Version 4\nEELAYER 30 0";
        assert_eq!(detect_format(content), Some(KicadVersion::Legacy4));
    }

    #[test]
    fn test_detect_legacy_schematic_v5() {
        let content = "EESchema Schematic File Version 5\nEELAYER 30 0";
        assert_eq!(detect_format(content), Some(KicadVersion::Legacy5));
    }

    #[test]
    fn test_detect_modern_schematic() {
        let content = "(kicad_sch (version 20211014) (generator \"eeschema\"))";
        assert_eq!(detect_format(content), Some(KicadVersion::Modern6));
    }

    #[test]
    fn test_detect_modern_pcb() {
        let content = "(kicad_pcb (version 20211014))";
        assert_eq!(detect_format(content), Some(KicadVersion::Modern6));
    }

    #[test]
    fn test_detect_legacy_pcb() {
        let content = "PCBNEW\n$MODULE";
        assert_eq!(detect_format(content), Some(KicadVersion::Legacy5));
    }
}
