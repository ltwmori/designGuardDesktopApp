//! Circuit Adapters Module
//!
//! This module provides adapters for converting different CAD file formats
//! into the Unified Circuit Schema (UCS). Each adapter implements the
//! CircuitAdapter trait.
//!
//! Supported formats:
//! - KiCAD (.kicad_sch, .sch) - supports KiCad 4-9
//! - Altium (planned)
//! - EasyEDA (planned)
//! - Eagle (planned)
//! - Generic Netlist (.net)

pub mod kicad;

use std::path::Path;
use thiserror::Error;

use super::{Circuit, UnifiedCircuitSchema, SourceCAD};

/// Errors that can occur during circuit adaptation
#[derive(Debug, Error)]
pub enum AdapterError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    
    #[error("Parse error: {0}")]
    Parse(String),
    
    #[error("Unsupported format: {0}")]
    UnsupportedFormat(String),
    
    #[error("Missing required field: {0}")]
    MissingField(String),
    
    #[error("Invalid data: {0}")]
    InvalidData(String),
}

/// Trait for adapting CAD files to UCS
pub trait CircuitAdapter {
    /// The source CAD type this adapter handles
    fn source_cad(&self) -> SourceCAD;
    
    /// File extensions this adapter can handle
    fn supported_extensions(&self) -> &[&str];
    
    /// Check if this adapter can handle a file
    fn can_handle(&self, path: &Path) -> bool {
        path.extension()
            .and_then(|ext| ext.to_str())
            .map(|ext| {
                self.supported_extensions()
                    .iter()
                    .any(|&supported| supported.eq_ignore_ascii_case(ext))
            })
            .unwrap_or(false)
    }
    
    /// Parse a file and return a UnifiedCircuitSchema
    fn parse_file(&self, path: &Path) -> Result<UnifiedCircuitSchema, AdapterError>;
    
    /// Parse a string and return a UnifiedCircuitSchema
    fn parse_string(&self, content: &str, filename: &str) -> Result<UnifiedCircuitSchema, AdapterError>;
    
    /// Parse a file and return a Circuit (graph-based)
    fn parse_to_circuit(&self, path: &Path) -> Result<Circuit, AdapterError> {
        let ucs = self.parse_file(path)?;
        Ok(Circuit::from_ucs(ucs))
    }
    
    /// Parse a string and return a Circuit (graph-based)
    fn parse_string_to_circuit(&self, content: &str, filename: &str) -> Result<Circuit, AdapterError> {
        let ucs = self.parse_string(content, filename)?;
        Ok(Circuit::from_ucs(ucs))
    }
}

/// Registry of available adapters
pub struct AdapterRegistry {
    adapters: Vec<Box<dyn CircuitAdapter + Send + Sync>>,
}

impl AdapterRegistry {
    /// Create a new registry with default adapters
    pub fn new() -> Self {
        let mut registry = Self {
            adapters: Vec::new(),
        };
        
        // Register default adapters
        registry.register(Box::new(kicad::KicadAdapter::new()));
        
        registry
    }
    
    /// Register a new adapter
    pub fn register(&mut self, adapter: Box<dyn CircuitAdapter + Send + Sync>) {
        self.adapters.push(adapter);
    }
    
    /// Find an adapter that can handle a file
    pub fn find_adapter(&self, path: &Path) -> Option<&(dyn CircuitAdapter + Send + Sync)> {
        self.adapters
            .iter()
            .find(|a| a.can_handle(path))
            .map(|a| a.as_ref())
    }
    
    /// Parse a file using the appropriate adapter
    pub fn parse_file(&self, path: &Path) -> Result<UnifiedCircuitSchema, AdapterError> {
        let adapter = self.find_adapter(path)
            .ok_or_else(|| {
                let ext = path.extension()
                    .and_then(|e| e.to_str())
                    .unwrap_or("unknown");
                AdapterError::UnsupportedFormat(format!("No adapter for .{} files", ext))
            })?;
        
        adapter.parse_file(path)
    }
    
    /// Parse a file to Circuit using the appropriate adapter
    pub fn parse_to_circuit(&self, path: &Path) -> Result<Circuit, AdapterError> {
        let adapter = self.find_adapter(path)
            .ok_or_else(|| {
                let ext = path.extension()
                    .and_then(|e| e.to_str())
                    .unwrap_or("unknown");
                AdapterError::UnsupportedFormat(format!("No adapter for .{} files", ext))
            })?;
        
        adapter.parse_to_circuit(path)
    }
    
    /// Get all supported file extensions
    pub fn supported_extensions(&self) -> Vec<&str> {
        self.adapters
            .iter()
            .flat_map(|a| a.supported_extensions().iter().copied())
            .collect()
    }
}

impl Default for AdapterRegistry {
    fn default() -> Self {
        Self::new()
    }
}

// Re-export adapters
pub use kicad::KicadAdapter;

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    
    #[test]
    fn test_registry_creation() {
        let registry = AdapterRegistry::new();
        assert!(!registry.supported_extensions().is_empty());
    }
    
    #[test]
    fn test_find_adapter() {
        let registry = AdapterRegistry::new();
        
        let kicad_path = PathBuf::from("test.kicad_sch");
        assert!(registry.find_adapter(&kicad_path).is_some());
        
        let unknown_path = PathBuf::from("test.unknown");
        assert!(registry.find_adapter(&unknown_path).is_none());
    }
}
