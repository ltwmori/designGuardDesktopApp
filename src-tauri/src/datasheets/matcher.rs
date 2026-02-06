//! Datasheet Matcher
//!
//! Matches components from schematics to their datasheet requirements.

use crate::datasheets::schema::{DatasheetDatabase, DatasheetRequirements};
use crate::parser::schema::Component;
use std::path::Path;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum MatcherError {
    #[error("Failed to read datasheet file: {0}")]
    IoError(#[from] std::io::Error),
    #[error("Failed to parse datasheet JSON: {0}")]
    ParseError(#[from] serde_json::Error),
    #[error("Invalid datasheet directory: {0}")]
    InvalidDirectory(String),
}

/// Matches components to their datasheet requirements
pub struct DatasheetMatcher {
    database: DatasheetDatabase,
}

impl DatasheetMatcher {
    /// Create a new matcher with an empty database
    pub fn new() -> Self {
        Self {
            database: DatasheetDatabase::new(),
        }
    }
    
    /// Create a matcher with built-in datasheets
    /// This loads from external JSON files first, then falls back to embedded
    pub fn with_builtin_datasheets() -> Self {
        let mut matcher = Self::new();
        matcher.load_all_datasheets();
        matcher
    }
    
    /// Load all datasheets (external + embedded fallbacks)
    pub fn load_all_datasheets(&mut self) {
        let datasheets = crate::datasheets::builtin::load_all_datasheets();
        for ds in datasheets {
            self.database.add(ds);
        }
        
        tracing::info!("Loaded {} total datasheets", self.database.count());
    }
    
    /// Load built-in datasheets only (embedded in binary)
    pub fn load_builtin_datasheets(&mut self) {
        let datasheets = crate::datasheets::builtin::get_all_datasheets();
        for ds in datasheets {
            self.database.add(ds);
        }
        
        tracing::info!("Loaded {} embedded datasheets", self.database.count());
    }
    
    /// Load datasheets from a directory of JSON files
    pub fn load_from_directory(&mut self, dir: &Path) -> Result<usize, MatcherError> {
        if !dir.is_dir() {
            return Err(MatcherError::InvalidDirectory(
                dir.to_string_lossy().to_string()
            ));
        }
        
        let mut count = 0;
        
        for entry in std::fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();
            
            if path.extension().map(|e| e == "json").unwrap_or(false) {
                match self.load_from_file(&path) {
                    Ok(_) => count += 1,
                    Err(e) => {
                        tracing::warn!("Failed to load datasheet from {:?}: {}", path, e);
                    }
                }
            }
        }
        
        tracing::info!("Loaded {} datasheets from {:?}", count, dir);
        Ok(count)
    }
    
    /// Load a single datasheet from a JSON file
    pub fn load_from_file(&mut self, path: &Path) -> Result<(), MatcherError> {
        let content = std::fs::read_to_string(path)?;
        let requirements: DatasheetRequirements = serde_json::from_str(&content)?;
        self.database.add(requirements);
        Ok(())
    }
    
    /// Try to match a component to a datasheet
    pub fn match_component(&self, component: &Component) -> Option<&DatasheetRequirements> {
        // Try matching by value (most common - contains part number)
        if let Some(req) = self.database.get(&component.value) {
            return Some(req);
        }
        
        // Try matching by lib_id (library name often contains part info)
        // Extract the part name from lib_id (format: "Library:PartName")
        if let Some(part_name) = component.lib_id.split(':').last() {
            if let Some(req) = self.database.get(part_name) {
                return Some(req);
            }
        }
        
        // Try full lib_id
        if let Some(req) = self.database.get(&component.lib_id) {
            return Some(req);
        }
        
        // Try matching by properties (some schematics store part number in properties)
        for (key, value) in &component.properties {
            let key_lower = key.to_lowercase();
            if key_lower.contains("part") || key_lower.contains("mpn") || key_lower == "pn" {
                if let Some(req) = self.database.get(value) {
                    return Some(req);
                }
            }
        }
        
        None
    }
    
    /// Get all components that match any datasheet
    pub fn match_all_components<'a>(
        &'a self,
        components: &'a [Component],
    ) -> Vec<(&'a Component, &'a DatasheetRequirements)> {
        components
            .iter()
            .filter_map(|c| self.match_component(c).map(|req| (c, req)))
            .collect()
    }
    
    /// Get the database for direct access
    pub fn database(&self) -> &DatasheetDatabase {
        &self.database
    }
    
    /// Get count of loaded datasheets
    pub fn datasheet_count(&self) -> usize {
        self.database.count()
    }
}

impl Default for DatasheetMatcher {
    fn default() -> Self {
        Self::with_builtin_datasheets()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use crate::parser::schema::Position;
    
    fn create_test_component(reference: &str, value: &str, lib_id: &str) -> Component {
        Component {
            uuid: "test".to_string(),
            reference: reference.to_string(),
            value: value.to_string(),
            lib_id: lib_id.to_string(),
            footprint: None,
            position: Position { x: 0.0, y: 0.0 },
            rotation: 0.0,
            properties: HashMap::new(),
            pins: vec![],
        }
    }
    
    #[test]
    fn test_matcher_with_builtin() {
        let matcher = DatasheetMatcher::with_builtin_datasheets();
        assert!(matcher.datasheet_count() > 0);
    }
    
    #[test]
    fn test_match_by_value() {
        let matcher = DatasheetMatcher::with_builtin_datasheets();
        
        let component = create_test_component("U1", "STM32F411CEU6", "MCU:STM32");
        let result = matcher.match_component(&component);
        
        assert!(result.is_some());
        assert_eq!(result.unwrap().manufacturer, "STMicroelectronics");
    }
    
    #[test]
    fn test_match_by_partial() {
        let matcher = DatasheetMatcher::with_builtin_datasheets();
        
        // Should match even with partial part number
        let component = create_test_component("U1", "NE555", "Timer:NE555");
        let result = matcher.match_component(&component);
        
        assert!(result.is_some());
    }
}
