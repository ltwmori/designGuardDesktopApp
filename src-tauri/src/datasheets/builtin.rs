//! Built-in and External Datasheet Requirements
//!
//! This module loads datasheet requirements from:
//! 1. External JSON files in the datasheets/ directory (user-editable)
//! 2. Embedded JSON files compiled into the binary (fallback)
//!
//! Users can add their own component definitions by creating JSON files
//! in the datasheets/ directory without recompiling the app.

use crate::datasheets::schema::*;
use std::path::Path;

// Embed the default JSON files into the binary as fallbacks
const EMBEDDED_STM32F411: &str = include_str!("../../datasheets/stm32f411.json");
const EMBEDDED_ESP32: &str = include_str!("../../datasheets/esp32-wroom-32.json");
const EMBEDDED_ATMEGA328P: &str = include_str!("../../datasheets/atmega328p.json");
const EMBEDDED_RP2040: &str = include_str!("../../datasheets/rp2040.json");
const EMBEDDED_LM1117: &str = include_str!("../../datasheets/lm1117-3.3.json");
const EMBEDDED_AMS1117: &str = include_str!("../../datasheets/ams1117-3.3.json");
const EMBEDDED_CH340G: &str = include_str!("../../datasheets/ch340g.json");
const EMBEDDED_CP2102: &str = include_str!("../../datasheets/cp2102.json");
const EMBEDDED_NE555: &str = include_str!("../../datasheets/ne555.json");
const EMBEDDED_LM7805: &str = include_str!("../../datasheets/lm7805.json");

/// Get all datasheet requirements from embedded JSON files
pub fn get_all_datasheets() -> Vec<DatasheetRequirements> {
    let embedded_jsons = [
        EMBEDDED_STM32F411,
        EMBEDDED_ESP32,
        EMBEDDED_ATMEGA328P,
        EMBEDDED_RP2040,
        EMBEDDED_LM1117,
        EMBEDDED_AMS1117,
        EMBEDDED_CH340G,
        EMBEDDED_CP2102,
        EMBEDDED_NE555,
        EMBEDDED_LM7805,
    ];
    
    let mut datasheets = Vec::new();
    
    for json_str in embedded_jsons {
        match serde_json::from_str::<DatasheetRequirements>(json_str) {
            Ok(ds) => datasheets.push(ds),
            Err(e) => {
                tracing::warn!("Failed to parse embedded datasheet: {}", e);
            }
        }
    }
    
    datasheets
}

/// Load datasheets from a directory of JSON files
/// Returns both successfully loaded datasheets and any errors encountered
pub fn load_datasheets_from_directory(dir: &Path) -> (Vec<DatasheetRequirements>, Vec<String>) {
    let mut datasheets = Vec::new();
    let mut errors = Vec::new();
    
    if !dir.exists() || !dir.is_dir() {
        return (datasheets, errors);
    }
    
    let entries = match std::fs::read_dir(dir) {
        Ok(entries) => entries,
        Err(e) => {
            errors.push(format!("Failed to read directory {:?}: {}", dir, e));
            return (datasheets, errors);
        }
    };
    
    for entry in entries.flatten() {
        let path = entry.path();
        
        // Skip non-JSON files and README
        if path.extension().map(|e| e != "json").unwrap_or(true) {
            continue;
        }
        
        match load_datasheet_from_file(&path) {
            Ok(ds) => {
                tracing::info!(
                    "Loaded datasheet for {} from {:?}",
                    ds.part_numbers.first().unwrap_or(&"Unknown".to_string()),
                    path.file_name()
                );
                datasheets.push(ds);
            }
            Err(e) => {
                let error_msg = format!("Failed to load {:?}: {}", path.file_name(), e);
                tracing::warn!("{}", error_msg);
                errors.push(error_msg);
            }
        }
    }
    
    (datasheets, errors)
}

/// Load a single datasheet from a JSON file
pub fn load_datasheet_from_file(path: &Path) -> Result<DatasheetRequirements, String> {
    let content = std::fs::read_to_string(path)
        .map_err(|e| format!("Failed to read file: {}", e))?;
    
    serde_json::from_str(&content)
        .map_err(|e| format!("Failed to parse JSON: {}", e))
}

/// Get the default datasheets directory path relative to the executable
pub fn get_datasheets_directory() -> Option<std::path::PathBuf> {
    // Try to find the datasheets directory relative to the executable
    if let Ok(exe_path) = std::env::current_exe() {
        // For development: check src-tauri/datasheets
        if let Some(parent) = exe_path.parent() {
            // macOS app bundle: Contents/MacOS/executable -> Contents/Resources/datasheets
            let resources_path = parent.parent()
                .map(|p| p.join("Resources").join("datasheets"));
            if let Some(ref path) = resources_path {
                if path.exists() {
                    return Some(path.clone());
                }
            }
            
            // Development: target/debug/executable -> src-tauri/datasheets
            let dev_path = parent.parent()
                .and_then(|p| p.parent())
                .map(|p| p.join("datasheets"));
            if let Some(ref path) = dev_path {
                if path.exists() {
                    return Some(path.clone());
                }
            }
            
            // Also try relative to current directory
            let cwd_path = std::path::PathBuf::from("datasheets");
            if cwd_path.exists() {
                return Some(cwd_path);
            }
            
            // Try src-tauri/datasheets from current directory
            let src_tauri_path = std::path::PathBuf::from("src-tauri/datasheets");
            if src_tauri_path.exists() {
                return Some(src_tauri_path);
            }
        }
    }
    
    None
}

/// Get user datasheets directory from app data directory
/// Returns None if app data directory cannot be determined
/// This checks common app data locations based on OS
/// Uses identifier: com.kicadai.assistant (from tauri.conf.json)
fn get_user_datasheets_directory() -> Option<std::path::PathBuf> {
    // Try common app data locations
    #[cfg(target_os = "macos")]
    {
        if let Some(home) = std::env::var_os("HOME") {
            let app_data = std::path::PathBuf::from(home)
                .join("Library/Application Support/com.kicadai.assistant/datasheets");
            if app_data.exists() {
                return Some(app_data);
            }
        }
    }
    
    #[cfg(target_os = "windows")]
    {
        if let Some(app_data) = std::env::var_os("APPDATA") {
            let app_data = std::path::PathBuf::from(app_data)
                .join("com.kicadai.assistant/datasheets");
            if app_data.exists() {
                return Some(app_data);
            }
        }
    }
    
    #[cfg(target_os = "linux")]
    {
        if let Some(home) = std::env::var_os("HOME") {
            let app_data = std::path::PathBuf::from(home)
                .join(".local/share/com.kicadai.assistant/datasheets");
            if app_data.exists() {
                return Some(app_data);
            }
        }
    }
    
    None
}

/// Load all datasheets: first from user directory, then external directory, then fill in with embedded
pub fn load_all_datasheets() -> Vec<DatasheetRequirements> {
    let mut all_datasheets = Vec::new();
    let mut loaded_part_numbers: std::collections::HashSet<String> = std::collections::HashSet::new();
    
    // Priority 1: Load from user datasheets directory (app_data_dir/datasheets)
    if let Some(user_dir) = get_user_datasheets_directory() {
        let (user_datasheets, errors) = load_datasheets_from_directory(&user_dir);
        
        for error in errors {
            tracing::warn!("User datasheet loading error: {}", error);
        }
        
        for ds in user_datasheets {
            // Track which part numbers we've loaded
            for pn in &ds.part_numbers {
                loaded_part_numbers.insert(pn.to_uppercase());
            }
            all_datasheets.push(ds);
        }
        
        tracing::info!("Loaded {} datasheets from user directory: {:?}", all_datasheets.len(), user_dir);
    }
    
    // Priority 2: Load from external directory (relative to executable)
    if let Some(dir) = get_datasheets_directory() {
        let (external, errors) = load_datasheets_from_directory(&dir);
        
        for error in errors {
            tracing::warn!("Datasheet loading error: {}", error);
        }
        
        for ds in external {
            // Only add if not already loaded from user directory
            let already_loaded = ds.part_numbers.iter()
                .any(|pn| loaded_part_numbers.contains(&pn.to_uppercase()));
            
            if !already_loaded {
                for pn in &ds.part_numbers {
                    loaded_part_numbers.insert(pn.to_uppercase());
                }
                all_datasheets.push(ds);
            }
        }
        
        tracing::info!("Loaded {} total datasheets (including external)", all_datasheets.len());
    }
    
    // Priority 3: Add embedded datasheets for any that weren't loaded externally
    let embedded = get_all_datasheets();
    for ds in embedded {
        // Check if any of this datasheet's part numbers are already loaded
        let already_loaded = ds.part_numbers.iter()
            .any(|pn| loaded_part_numbers.contains(&pn.to_uppercase()));
        
        if !already_loaded {
            for pn in &ds.part_numbers {
                loaded_part_numbers.insert(pn.to_uppercase());
            }
            all_datasheets.push(ds);
        }
    }
    
    tracing::info!("Total datasheets available: {}", all_datasheets.len());
    all_datasheets
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_embedded_datasheets_parse() {
        let datasheets = get_all_datasheets();
        assert_eq!(datasheets.len(), 10);
        
        // Verify each has required fields
        for ds in &datasheets {
            assert!(!ds.part_numbers.is_empty());
            assert!(!ds.manufacturer.is_empty());
        }
    }
    
    #[test]
    fn test_stm32_datasheet() {
        let ds: DatasheetRequirements = serde_json::from_str(EMBEDDED_STM32F411).unwrap();
        assert!(ds.part_numbers.contains(&"STM32F411CEU6".to_string()));
        assert_eq!(ds.manufacturer, "STMicroelectronics");
        assert!(!ds.decoupling_requirements.is_empty());
    }
    
    #[test]
    fn test_esp32_datasheet() {
        let ds: DatasheetRequirements = serde_json::from_str(EMBEDDED_ESP32).unwrap();
        assert!(ds.part_numbers.contains(&"ESP32-WROOM-32".to_string()));
        assert_eq!(ds.manufacturer, "Espressif");
    }
    
    #[test]
    fn test_ne555_datasheet() {
        let ds: DatasheetRequirements = serde_json::from_str(EMBEDDED_NE555).unwrap();
        assert!(ds.part_numbers.contains(&"NE555".to_string()));
        assert!(ds.part_numbers.contains(&"555".to_string()));
    }
}
