use std::path::{Path, PathBuf};
use tauri::{State, Manager};
use crate::state::{AppState, Settings};
use crate::watcher::ProjectWatcher;
use crate::parser::schema::Schematic;
use crate::parser::pcb::PcbParser;
use crate::parser::netlist::NetlistBuilder;
use crate::analyzer::rules::{Issue, RulesEngine, RuleContext};
use crate::analyzer::explanations::DetailedIssue;
use crate::analyzer::drs::{DRSAnalyzer, ICRiskScore, PathAnalysis, PathError};
use crate::ai::claude::AIAnalysis;
use crate::ai::provider::{ProviderStatus, SchematicContext, ComponentDetail};
use crate::datasheets::checker::DatasheetChecker;
use crate::ucs::{
    UnifiedCircuitSchema, 
    adapters::{AdapterRegistry, KicadAdapter, CircuitAdapter},
    analysis::{self, AiCircuitSummary},
};
use serde::{Deserialize, Serialize};
use chrono::Utc;

#[derive(Debug, Serialize, Deserialize)]
pub struct ProjectInfo {
    pub path: String,
    pub name: String,
    pub last_analyzed: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AnalysisResult {
    pub project_path: String,
    pub timestamp: String,
    pub issues: Vec<String>,
    pub suggestions: Vec<String>,
    pub ai_analysis: Option<String>,
}

/// Helper function to detect file format
#[allow(dead_code)]
fn detect_file_format(path: &Path) -> Result<bool, String> {
    let content = std::fs::read_to_string(path)
        .map_err(|e| format!("Failed to read file {}: {}", path.display(), e))?;
    
    // New KiCad 6+ format starts with (kicad_sch or (kicad_pcb
    let is_new_format = content.trim_start().starts_with("(kicad_sch") || 
                       content.trim_start().starts_with("(kicad_pcb");
    
    Ok(is_new_format)
}

/// Recursively search for KiCAD files in a directory
/// Returns (schematics, pcbs, project_file, errors)
fn find_kicad_files_recursive(dir: &Path) -> (Vec<PathBuf>, Vec<PathBuf>, Option<PathBuf>, Vec<String>) {
    let mut schematics = Vec::new();
    let mut pcbs = Vec::new();
    let mut project_file: Option<PathBuf> = None;
    let mut errors = Vec::new();
    
    fn walk_dir(
        dir: &Path, 
        schematics: &mut Vec<PathBuf>, 
        pcbs: &mut Vec<PathBuf>, 
        project_file: &mut Option<PathBuf>,
        errors: &mut Vec<String>,
        depth: usize,
    ) {
        // Limit recursion depth to prevent infinite loops and performance issues
        if depth > 20 {
            errors.push(format!("Maximum recursion depth reached at: {}", dir.display()));
            return;
        }
        
        match std::fs::read_dir(dir) {
            Ok(entries) => {
                for entry in entries {
                    match entry {
                        Ok(entry) => {
                            let entry_path = entry.path();
                            
                            // Skip hidden directories and common non-project directories
                            if entry_path.is_dir() {
                                let dir_name = entry_path.file_name()
                                    .and_then(|n| n.to_str())
                                    .unwrap_or("");
                                
                                // Skip common non-project directories to improve performance
                                if dir_name.starts_with('.') || 
                                   dir_name == "node_modules" || 
                                   dir_name == "target" ||
                                   dir_name == "build" ||
                                   dir_name == ".git" {
                                    continue;
                                }
                                
                                // Recursively search subdirectories
                                walk_dir(&entry_path, schematics, pcbs, project_file, errors, depth + 1);
                            } else if let Some(ext) = entry_path.extension().and_then(|s| s.to_str()) {
                                match ext {
                                    "kicad_sch" | "sch" => {
                                        // Support both new (.kicad_sch) and old (.sch) KiCad formats
                                        schematics.push(entry_path);
                                    }
                                    "kicad_pcb" | "brd" => {
                                        // Support both new (.kicad_pcb) and old (.brd) KiCad formats
                                        pcbs.push(entry_path);
                                    }
                                    "kicad_pro" => {
                                        *project_file = Some(entry_path);
                                    }
                                    _ => {}
                                }
                            }
                        }
                        Err(e) => {
                            errors.push(format!("Failed to read directory entry in {}: {}", dir.display(), e));
                        }
                    }
                }
            }
            Err(e) => {
                let error_msg = format!("Failed to read directory {}: {}", dir.display(), e);
                errors.push(error_msg.clone());
                tracing::warn!("{}", error_msg);
            }
        }
    }
    
    walk_dir(dir, &mut schematics, &mut pcbs, &mut project_file, &mut errors, 0);
    
    (schematics, pcbs, project_file, errors)
}

// New commands

#[tauri::command]
pub async fn open_project(
    path: String,
    state: State<'_, AppState>,
) -> Result<ProjectInfo, String> {
    use crate::parser::kicad::KicadParser;
    
    let project_path = PathBuf::from(&path);
    
    // Validate path exists
    if !project_path.exists() {
        return Err(format!("Project path does not exist: {}", path));
    }
    
    // Find schematic and PCB files
    let (schematic_path, pcb_path) = if project_path.is_file() {
        let ext = project_path.extension().and_then(|s| s.to_str());
        match ext {
            Some("kicad_sch") | Some("sch") => {
                // Schematic file selected - find PCB in same directory (try both new and old formats)
                let dir = project_path.parent().unwrap_or(&project_path);
                let base_name = project_path.file_stem().and_then(|s| s.to_str()).unwrap_or("");
                
                // Try new format first, then old format
                let pcb_new = dir.join(format!("{}.kicad_pcb", base_name));
                let pcb_old = dir.join(format!("{}.brd", base_name));
                
                let pcb = if pcb_new.exists() {
                    Some(pcb_new)
                } else if pcb_old.exists() {
                    Some(pcb_old)
                } else {
                    None
                };
                
                (project_path.clone(), pcb)
            }
            Some("kicad_pcb") | Some("brd") => {
                // PCB file selected - find schematic in same directory (try both new and old formats)
                let dir = project_path.parent().unwrap_or(&project_path);
                let base_name = project_path.file_stem().and_then(|s| s.to_str()).unwrap_or("");
                
                // Try new format first, then old format
                let schematic_new = dir.join(format!("{}.kicad_sch", base_name));
                let schematic_old = dir.join(format!("{}.sch", base_name));
                
                let schematic = if schematic_new.exists() {
                    schematic_new
                } else if schematic_old.exists() {
                    schematic_old
                } else {
                    return Err(format!("No schematic file found for PCB (searched .kicad_sch and .sch): {}", path));
                };
                
                (schematic, Some(project_path.clone()))
            }
            Some("kicad_pro") => {
                // Project file selected - find schematic and PCB in same directory (try both formats)
                let dir = project_path.parent().unwrap_or(&project_path);
                let base_name = project_path.file_stem().and_then(|s| s.to_str()).unwrap_or("");
                
                // Try new format first
                let schematic_new = dir.join(format!("{}.kicad_sch", base_name));
                let pcb_new = dir.join(format!("{}.kicad_pcb", base_name));
                
                // Try old format
                let schematic_old = dir.join(format!("{}.sch", base_name));
                let pcb_old = dir.join(format!("{}.brd", base_name));
                
                let schematic = if schematic_new.exists() {
                    schematic_new
                } else if schematic_old.exists() {
                    schematic_old
                } else {
                    return Err(format!("No schematic file found for project (searched .kicad_sch and .sch): {}", path));
                };
                
                let pcb = if pcb_new.exists() {
                    Some(pcb_new)
                } else if pcb_old.exists() {
                    Some(pcb_old)
                } else {
                    None
                };
                
                (schematic, pcb)
            }
            _ => return Err(format!("Unsupported file type. Please select a .kicad_pro, .kicad_sch, .sch, .kicad_pcb, or .brd file, or a directory containing KiCad files: {}", path))
        }
    } else if project_path.is_dir() {
        // Directory selected - recursively search for KiCAD files
        let (schematics, pcbs, project_file, search_errors) = find_kicad_files_recursive(&project_path);
        
        // Report any errors encountered during search
        if !search_errors.is_empty() {
            tracing::warn!("Encountered {} errors while searching directory: {:?}", search_errors.len(), search_errors);
            // Don't fail immediately - continue if we found files despite errors
        }
        
        // Check if we found any schematics
        if schematics.is_empty() {
            // Provide helpful error with suggestions
            let mut error_msg = format!(
                "❌ No KiCad schematic files found\n\n\
                Searched directory (and subdirectories): {}\n\n\
                The app is looking for files with extensions:\n\
                • .kicad_sch (KiCad 6+ format)\n\
                • .sch (KiCad 5 format, or KiCad 6+ with old extension)\n\n",
                path
            );
            
            // Report search errors if any
            if !search_errors.is_empty() {
                error_msg.push_str(&format!(
                    "⚠️ Encountered {} error(s) while searching:\n",
                    search_errors.len()
                ));
                for err in search_errors.iter().take(5) {
                    error_msg.push_str(&format!("  • {}\n", err));
                }
                if search_errors.len() > 5 {
                    error_msg.push_str(&format!("  ... and {} more errors\n", search_errors.len() - 5));
                }
                error_msg.push_str("\n");
            }
            
            // Check if there are any files at all
            let mut file_count = 0;
            let mut sample_files = Vec::new();
            if let Ok(entries) = std::fs::read_dir(&project_path) {
                for entry in entries.flatten() {
                    if entry.path().is_file() {
                        file_count += 1;
                        if sample_files.len() < 5 {
                            if let Some(name) = entry.path().file_name().and_then(|n| n.to_str()) {
                                sample_files.push(name.to_string());
                            }
                        }
                    }
                }
            }
            
            if file_count == 0 {
                error_msg.push_str("The directory appears to be empty or contains only subdirectories.\n");
            } else {
                error_msg.push_str(&format!("Found {} file(s) in the directory", file_count));
                if !sample_files.is_empty() {
                    error_msg.push_str(&format!(" (sample: {})", sample_files.join(", ")));
                }
                error_msg.push_str(".\n\n");
                error_msg.push_str("💡 Tips:\n");
                error_msg.push_str("• Make sure your schematic files are in this directory or a subdirectory\n");
                error_msg.push_str("• Check that files have the correct extensions (.kicad_sch or .sch)\n");
                error_msg.push_str("• The app supports KiCad versions 4-9 - both legacy (.sch) and modern (.kicad_sch) formats\n");
                error_msg.push_str("• Check file permissions - the app may not have read access to some files\n");
            }
            
            return Err(error_msg);
        }
        
        // Smart matching strategy
        let (schematic_path, pcb_path) = {
            // Strategy 0: If there's exactly one schematic and one PCB, match them regardless of name
            if schematics.len() == 1 && pcbs.len() == 1 {
                (schematics[0].clone(), Some(pcbs[0].clone()))
            }
            // Strategy 1: If .kicad_pro exists, use its base name to find matching files
            else if let Some(ref pro_path) = project_file {
                let pro_base = pro_path.file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or("");
                
                let matching_sch = schematics.iter()
                    .find(|sch| sch.file_stem().and_then(|s| s.to_str()) == Some(pro_base));
                let matching_pcb = pcbs.iter()
                    .find(|pcb| pcb.file_stem().and_then(|s| s.to_str()) == Some(pro_base));
                
                if let Some(sch) = matching_sch {
                    (sch.clone(), matching_pcb.cloned())
                } else {
                    // Strategy 2: Match schematic and PCB by base name (exact or fuzzy)
                    let mut best_match: Option<(PathBuf, Option<PathBuf>)> = None;
                    let mut best_score = 0;
                    
                    for sch in &schematics {
                        let sch_base = sch.file_stem().and_then(|s| s.to_str()).unwrap_or("");
                        
                        // Try exact match first
                        let matching_pcb = pcbs.iter()
                            .find(|pcb| {
                                let pcb_base = pcb.file_stem().and_then(|s| s.to_str()).unwrap_or("");
                                pcb_base == sch_base
                            });
                        
                        // If no exact match, try fuzzy matching (check if one name contains the other)
                        let fuzzy_pcb = if matching_pcb.is_none() {
                            pcbs.iter().find(|pcb| {
                                let pcb_base = pcb.file_stem().and_then(|s| s.to_str()).unwrap_or("");
                                // Check if either name contains a significant portion of the other
                                (sch_base.len() > 10 && pcb_base.contains(&sch_base[..sch_base.len().min(20)])) ||
                                (pcb_base.len() > 10 && sch_base.contains(&pcb_base[..pcb_base.len().min(20)]))
                            })
                        } else {
                            None
                        };
                        
                        let final_pcb = matching_pcb.or(fuzzy_pcb);
                        let score = if final_pcb.is_some() { 2 } else { 1 };
                        
                        // Also check if directory name matches
                        let dir_name = project_path.file_name()
                            .and_then(|s| s.to_str())
                            .unwrap_or("");
                        let dir_match_bonus = if sch_base == dir_name { 1 } else { 0 };
                        let total_score = score + dir_match_bonus;
                        
                        if total_score > best_score {
                            best_score = total_score;
                            best_match = Some((sch.clone(), final_pcb.cloned()));
                        }
                    }
                    
                    // Strategy 3: Fallback to first schematic found, and first PCB if available
                    if let Some((sch, pcb)) = best_match {
                        (sch, pcb)
                    } else {
                        let first_sch = schematics[0].clone();
                        // If there's only one PCB, use it even if names don't match
                        let matching_pcb = if pcbs.len() == 1 {
                            Some(pcbs[0].clone())
                        } else {
                            // Try to find a PCB with matching base name
                            let sch_base = first_sch.file_stem().and_then(|s| s.to_str()).unwrap_or("");
                            pcbs.iter()
                                .find(|pcb| pcb.file_stem().and_then(|s| s.to_str()) == Some(sch_base))
                                .cloned()
                        };
                        (first_sch, matching_pcb)
                    }
                }
            } else {
                // Strategy 2: Match schematic and PCB by base name (exact or fuzzy)
                let mut best_match: Option<(PathBuf, Option<PathBuf>)> = None;
                let mut best_score = 0;
                
                for sch in &schematics {
                    let sch_base = sch.file_stem().and_then(|s| s.to_str()).unwrap_or("");
                    
                    // Try exact match first
                    let matching_pcb = pcbs.iter()
                        .find(|pcb| {
                            let pcb_base = pcb.file_stem().and_then(|s| s.to_str()).unwrap_or("");
                            pcb_base == sch_base
                        });
                    
                    // If no exact match, try fuzzy matching (check if one name contains the other)
                    let fuzzy_pcb = if matching_pcb.is_none() {
                        pcbs.iter().find(|pcb| {
                            let pcb_base = pcb.file_stem().and_then(|s| s.to_str()).unwrap_or("");
                            // Check if either name contains a significant portion of the other
                            (sch_base.len() > 10 && pcb_base.contains(&sch_base[..sch_base.len().min(20)])) ||
                            (pcb_base.len() > 10 && sch_base.contains(&pcb_base[..pcb_base.len().min(20)]))
                        })
                    } else {
                        None
                    };
                    
                    let final_pcb = matching_pcb.or(fuzzy_pcb);
                    let score = if final_pcb.is_some() { 2 } else { 1 };
                    
                    // Also check if directory name matches
                    let dir_name = project_path.file_name()
                        .and_then(|s| s.to_str())
                        .unwrap_or("");
                    let dir_match_bonus = if sch_base == dir_name { 1 } else { 0 };
                    let total_score = score + dir_match_bonus;
                    
                    if total_score > best_score {
                        best_score = total_score;
                        best_match = Some((sch.clone(), final_pcb.cloned()));
                    }
                }
                
                // Strategy 3: Fallback to first schematic found, and first PCB if available
                if let Some((sch, pcb)) = best_match {
                    (sch, pcb)
                } else {
                    let first_sch = schematics[0].clone();
                    // If there's only one PCB, use it even if names don't match
                    let matching_pcb = if pcbs.len() == 1 {
                        Some(pcbs[0].clone())
                    } else {
                        // Try to find a PCB with matching base name
                        let sch_base = first_sch.file_stem().and_then(|s| s.to_str()).unwrap_or("");
                        pcbs.iter()
                            .find(|pcb| pcb.file_stem().and_then(|s| s.to_str()) == Some(sch_base))
                            .cloned()
                    };
                    (first_sch, matching_pcb)
                }
            }
        };
        
        // Provide helpful logging about what was found
        tracing::info!(
            "Found {} schematic(s) and {} PCB(s) in directory: {}",
            schematics.len(),
            pcbs.len(),
            path
        );
        
        if !schematics.is_empty() {
            let sch_names: Vec<String> = schematics.iter()
                .map(|p| p.file_name().and_then(|n| n.to_str()).unwrap_or("unknown").to_string())
                .collect();
            tracing::info!("Schematic files found: {:?}", sch_names);
        }
        
        if !pcbs.is_empty() {
            let pcb_names: Vec<String> = pcbs.iter()
                .map(|p| p.file_name().and_then(|n| n.to_str()).unwrap_or("unknown").to_string())
                .collect();
            tracing::info!("PCB files found: {:?}", pcb_names);
        }
        
        // Provide helpful error message if multiple schematics but no clear match
        if schematics.len() > 1 && pcb_path.is_none() {
            let sch_names: Vec<String> = schematics.iter()
                .map(|p| p.file_name().and_then(|n| n.to_str()).unwrap_or("unknown").to_string())
                .collect();
            tracing::warn!(
                "Found multiple schematic files in directory: {:?}. Selected: {}. No matching PCB found.",
                sch_names,
                schematic_path.file_name().and_then(|n| n.to_str()).unwrap_or("unknown")
            );
        }
        
        // Log the final selection
        tracing::info!(
            "Selected schematic: {}, PCB: {}",
            schematic_path.file_name().and_then(|n| n.to_str()).unwrap_or("unknown"),
            pcb_path.as_ref()
                .and_then(|p| p.file_name())
                .and_then(|n| n.to_str())
                .unwrap_or("none")
        );
        
        (schematic_path, pcb_path)
    } else {
        return Err(format!("Invalid project path: {}", path));
    };
    
    // Parse schematic - format detection and routing is handled by the parser
    tracing::info!("Attempting to parse schematic: {}", schematic_path.display());
    let schematic = KicadParser::parse_schematic(&schematic_path)
        .map_err(|e| {
            format!(
                "❌ Failed to parse schematic\n\n\
                File: {}\n\
                Error: {}\n\n\
                The parser supports KiCad versions 4-9. If parsing fails, please check:\n\
                • File is not corrupted\n\
                • File is a valid KiCad schematic file\n\
                • File permissions allow reading",
                schematic_path.display(),
                e
            )
        })?;
    
    tracing::info!("Successfully parsed schematic with {} components", schematic.components.len());
    
    // Parse using UCS adapter for new graph-based representation
    // Note: UCS adapter currently only supports modern S-expression format
    // Legacy formats will use the parsed Schematic directly
    let circuit = {
        let adapter = KicadAdapter::new();
        match adapter.parse_to_circuit(&schematic_path) {
            Ok(circ) => Some(circ),
            Err(e) => {
                tracing::warn!("UCS adapter failed (may be legacy format): {}. Using Schematic representation.", e);
                None
            }
        }
    };
    
    // Parse PCB if available
    if let Some(ref pcb_path) = pcb_path {
        use crate::parser::pcb::PcbParser;
        match PcbParser::parse_pcb(pcb_path) {
            Ok(pcb) => {
                let mut current_pcb = state.current_pcb.lock()
                    .map_err(|e| format!("Failed to lock current_pcb: {}", e))?;
                *current_pcb = Some(pcb);
                tracing::info!("Loaded PCB file: {}", pcb_path.display());
            }
            Err(e) => {
                tracing::warn!("Failed to parse PCB file {}: {}. DRS analysis will not be available.", pcb_path.display(), e);
            }
        }
    } else {
        tracing::info!("No PCB file found. DRS analysis will not be available.");
    }
    
    // Update state - use schematic path as project path
    {
        let mut current_path = state.project_path.lock()
            .map_err(|e| format!("Failed to lock project_path: {}", e))?;
        *current_path = Some(schematic_path.parent().unwrap_or(&schematic_path).to_path_buf());
    }
    
    {
        let mut current_schematic = state.current_schematic.lock()
            .map_err(|e| format!("Failed to lock current_schematic: {}", e))?;
        *current_schematic = Some(schematic);
    }
    
    {
        let mut current_circuit = state.current_circuit.lock()
            .map_err(|e| format!("Failed to lock current_circuit: {}", e))?;
        *current_circuit = circuit;
    }
    
    // Start watching if auto-analyze is enabled
    let should_watch = {
        let settings = state.settings.lock()
            .map_err(|e| format!("Failed to lock settings: {}", e))?;
        settings.auto_analyze
    };
    
    if should_watch {
        let project_path_str = project_path.to_string_lossy().to_string();
        // Start watching directly
        let mut watchers = state.watchers.write().await;
        if !watchers.contains_key(&project_path_str) {
            let mut watcher = ProjectWatcher::new();
            if let Err(e) = watcher.watch(project_path.clone()).await {
                tracing::warn!("Failed to start watching project: {}", e);
            } else {
                watchers.insert(project_path_str, watcher);
            }
        }
    }
    
    // Get project name from schematic file
    let name = schematic_path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("Unknown")
        .to_string();
    
    // Save to database - use schematic path
    let project_info = ProjectInfo {
        path: schematic_path.to_string_lossy().to_string(),
        name: name.clone(),
        last_analyzed: Some(Utc::now().to_rfc3339()),
    };
    
    // Store in database (you may want to add a method for this)
    tracing::info!(
        "Successfully opened project: {} (schematic: {}, PCB: {})",
        name,
        schematic_path.display(),
        pcb_path.as_ref()
            .map(|p| p.display().to_string())
            .unwrap_or_else(|| "none".to_string())
    );
    
    Ok(project_info)
}

#[tauri::command]
pub async fn close_project(
    state: State<'_, AppState>,
) -> Result<(), String> {
    // Stop watching
    let project_path = {
        let current_path = state.project_path.lock()
            .map_err(|e| format!("Failed to lock project_path: {}", e))?;
        current_path.clone()
    };
    
    if let Some(path) = project_path {
        let path_str = path.to_string_lossy().to_string();
        // Stop watching directly
        let mut watchers = state.watchers.write().await;
        if let Some(mut watcher) = watchers.remove(&path_str) {
            if let Err(e) = watcher.unwatch().await {
                tracing::warn!("Failed to stop watching: {}", e);
            }
        }
    }
    
    // Clear state
    {
        let mut current_path = state.project_path.lock()
            .map_err(|e| format!("Failed to lock project_path: {}", e))?;
        *current_path = None;
    }
    
    {
        let mut current_schematic = state.current_schematic.lock()
            .map_err(|e| format!("Failed to lock current_schematic: {}", e))?;
        *current_schematic = None;
    }
    
    {
        let mut current_circuit = state.current_circuit.lock()
            .map_err(|e| format!("Failed to lock current_circuit: {}", e))?;
        *current_circuit = None;
    }
    
    {
        let mut issues = state.issues.lock()
            .map_err(|e| format!("Failed to lock issues: {}", e))?;
        issues.clear();
    }
    
    tracing::info!("Closed project");
    Ok(())
}

#[tauri::command]
pub async fn analyze_design(
    state: State<'_, AppState>,
) -> Result<Vec<Issue>, String> {
    // Get current schematic
    let schematic = {
        let current_schematic = state.current_schematic.lock()
            .map_err(|e| format!("Failed to lock current_schematic: {}", e))?;
        current_schematic.clone()
    };
    
    let schematic = schematic.ok_or_else(|| "No schematic loaded. Please open a project first.".to_string())?;
    
    // Run design rule checks with enhanced capacitor classification
    let engine = RulesEngine::with_default_rules();
    
    // Get PCB if available
    let pcb = {
        let current_pcb = state.current_pcb.lock()
            .map_err(|e| format!("Failed to lock current_pcb: {}", e))?;
        current_pcb.clone()
    };
    
    // Build enhanced context if possible
    let context = {
        // Get Circuit from state (preferred) or build from schematic
        let circuit = {
            let current_circuit = state.current_circuit.lock()
                .map_err(|e| format!("Failed to lock current_circuit: {}", e))?;
            current_circuit.clone()
        };
        
        // Build netlist
        let pin_to_net = crate::parser::netlist::NetlistBuilder::build_netlist(&schematic);
        
        // Build power net registry
        let power_registry = crate::compliance::power_net_registry::PowerNetRegistry::new(&schematic);
        
        // Classify capacitors
        let classifications = crate::analyzer::capacitor_classifier::CapacitorClassifier::classify_capacitors(
            &schematic,
            &power_registry,
            &pin_to_net,
        );
        
        // Build decoupling groups - use Circuit if available, otherwise fall back to Schematic
        let decoupling_groups = if let Some(ref circ) = circuit {
            // Use unified Circuit-based analysis
            crate::analyzer::decoupling_groups::DecouplingGroupsAnalyzer::build_groups_from_circuit(
                circ,
                &classifications,
            )
        } else {
            // Fall back to legacy Schematic-based analysis
            crate::analyzer::decoupling_groups::DecouplingGroupsAnalyzer::build_groups(
                &schematic,
                &power_registry,
                &classifications,
                &pin_to_net,
            )
        };
        
        Some(RuleContext {
            capacitor_classifications: classifications,
            decoupling_groups,
            power_registry,
            pcb,
        })
    };
    
    let issues = if let Some(ctx) = &context {
        engine.analyze_enhanced(&schematic, Some(ctx))
    } else {
        engine.analyze(&schematic)
    };
    
    // Update state
    {
        let mut current_issues = state.issues.lock()
            .map_err(|e| format!("Failed to lock issues: {}", e))?;
        *current_issues = issues.clone();
    }
    
    tracing::info!("Analyzed design, found {} issues", issues.len());
    Ok(issues)
}

// Legacy ai_analyze and ask_ai functions removed - use ai_analyze_with_router and ask_ai_with_router instead

#[tauri::command]
pub async fn set_api_key(
    key: String,
    state: State<'_, AppState>,
) -> Result<(), String> {
    if key.is_empty() {
        return Err("API key cannot be empty".to_string());
    }
    
    // Use router as single source of truth
    {
        let mut router = state.ai_router.write().await;
        router.set_claude_api_key(key);
    }
    
    tracing::info!("Claude API key set");
    Ok(())
}

#[tauri::command]
pub async fn get_settings(
    state: State<'_, AppState>,
) -> Result<Settings, String> {
    let settings = state.settings.lock()
        .map_err(|e| format!("Failed to lock settings: {}", e))?;
    Ok(settings.clone())
}

#[tauri::command]
pub async fn update_settings(
    settings: Settings,
    state: State<'_, AppState>,
) -> Result<(), String> {
    {
        let mut current_settings = state.settings.lock()
            .map_err(|e| format!("Failed to lock settings: {}", e))?;
        *current_settings = settings;
    }
    
    tracing::info!("Settings updated");
    Ok(())
}


// Updated versions of existing commands that work with new state

#[tauri::command]
pub async fn watch_project(
    project_path: String,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let path = PathBuf::from(&project_path);
    
    let mut watchers = state.watchers.write().await;
    
    // Check if already watching
    if watchers.contains_key(&project_path) {
        return Ok(()); // Already watching
    }

    // Create new watcher
    let mut watcher = ProjectWatcher::new();
    watcher.watch(path)
        .await
        .map_err(|e| format!("Failed to start watching: {}", e))?;

    // Store watcher
    watchers.insert(project_path.clone(), watcher);

    tracing::info!("Started watching project: {}", project_path);
    Ok(())
}

#[tauri::command]
pub async fn stop_watching(
    project_path: String,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let mut watchers = state.watchers.write().await;
    
    if let Some(mut watcher) = watchers.remove(&project_path) {
        watcher.unwatch()
            .await
            .map_err(|e| format!("Failed to stop watching: {}", e))?;
        tracing::info!("Stopped watching project: {}", project_path);
    }
    
    Ok(())
}

#[tauri::command]
pub async fn parse_schematic(
    schematic_path: String,
    _state: State<'_, AppState>,
) -> Result<Schematic, String> {
    use crate::parser::kicad::KicadParser;
    KicadParser::parse_schematic(Path::new(&schematic_path))
        .map_err(|e| e.to_string())
}

/// Get the currently loaded schematic from state (avoids re-parsing)
#[tauri::command]
pub async fn get_current_schematic(
    state: State<'_, AppState>,
) -> Result<Schematic, String> {
    let schematic = {
        let current_schematic = state.current_schematic.lock()
            .map_err(|e| format!("Failed to lock current_schematic: {}", e))?;
        current_schematic.clone()
    };
    
    schematic.ok_or_else(|| "No schematic loaded. Please open a project first.".to_string())
}

#[tauri::command]
pub async fn run_drc(
    schematic_path: String,
    _state: State<'_, AppState>,
) -> Result<Vec<Issue>, String> {
    use crate::parser::kicad::KicadParser;
    
    // Parse the schematic first
    let schematic = KicadParser::parse_schematic(Path::new(&schematic_path))
        .map_err(|e| format!("Failed to parse schematic: {}", e))?;
    
    // Run design rule checks
    let engine = RulesEngine::with_default_rules();
    Ok(engine.analyze(&schematic))
}

// Legacy analyze_with_ai and ask_ai_question functions removed - use ai_analyze_with_router and ask_ai_with_router instead

#[tauri::command]
pub async fn get_project_history(
    state: State<'_, AppState>,
) -> Result<Vec<ProjectInfo>, String> {
    state.db.get_project_history()
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_analysis_results(
    project_path: String,
    state: State<'_, AppState>,
) -> Result<Vec<AnalysisResult>, String> {
    state.db.get_analysis_results(&project_path)
        .map_err(|e| e.to_string())
}

// ============================================================================
// NEW FEATURES: Datasheet Checking, Ollama Integration, Detailed Explanations
// ============================================================================

/// Run datasheet-aware design checks
#[tauri::command]
pub async fn run_datasheet_check(
    state: State<'_, AppState>,
) -> Result<Vec<Issue>, String> {
    // Get current schematic
    let schematic = {
        let current_schematic = state.current_schematic.lock()
            .map_err(|e| format!("Failed to lock current_schematic: {}", e))?;
        current_schematic.clone()
    };
    
    let schematic = schematic.ok_or_else(|| "No schematic loaded. Please open a project first.".to_string())?;
    
    // Run datasheet checker
    let checker = DatasheetChecker::new();
    let issues = checker.check_as_issues(&schematic);
    
    tracing::info!("Datasheet check found {} issues", issues.len());
    Ok(issues)
}

/// Get detailed explanation for an issue
#[tauri::command]
pub async fn get_issue_details(
    issue: Issue,
) -> Result<DetailedIssue, String> {
    Ok(DetailedIssue::from(issue))
}

/// Get detailed explanations for all issues
#[tauri::command]
pub async fn get_all_issue_details(
    state: State<'_, AppState>,
) -> Result<Vec<DetailedIssue>, String> {
    let issues = {
        let current_issues = state.issues.lock()
            .map_err(|e| format!("Failed to lock issues: {}", e))?;
        current_issues.clone()
    };
    
    Ok(issues.into_iter().map(DetailedIssue::from).collect())
}

/// Run full analysis (DRC + Datasheet checks)
#[tauri::command]
pub async fn run_full_analysis(
    state: State<'_, AppState>,
) -> Result<Vec<DetailedIssue>, String> {
    // Get current schematic
    let schematic = {
        let current_schematic = state.current_schematic.lock()
            .map_err(|e| format!("Failed to lock current_schematic: {}", e))?;
        current_schematic.clone()
    };
    
    let schematic = schematic.ok_or_else(|| "No schematic loaded. Please open a project first.".to_string())?;
    
    // Run standard DRC
    let engine = RulesEngine::with_default_rules();
    let mut issues = engine.analyze(&schematic);
    
    // Run datasheet checks
    let checker = DatasheetChecker::new();
    issues.extend(checker.check_as_issues(&schematic));
    
    // Update state
    {
        let mut current_issues = state.issues.lock()
            .map_err(|e| format!("Failed to lock issues: {}", e))?;
        *current_issues = issues.clone();
    }
    
    // Convert to detailed issues
    let detailed: Vec<DetailedIssue> = issues.into_iter().map(DetailedIssue::from).collect();
    
    tracing::info!("Full analysis found {} issues", detailed.len());
    Ok(detailed)
}

// ============================================================================
// Ollama Integration Commands
// ============================================================================

/// Configure Ollama for local AI
#[tauri::command]
pub async fn configure_ollama(
    url: Option<String>,
    model: String,
    state: State<'_, AppState>,
) -> Result<bool, String> {
    let mut router = state.ai_router.write().await;
    router.set_ollama_config(url, Some(model));
    
    // Test connection
    let available = router.get_status().await.ollama_available;
    
    tracing::info!("Ollama configured, available: {}", available);
    Ok(available)
}

/// List available Ollama models
#[tauri::command]
pub async fn list_ollama_models(
    url: Option<String>,
) -> Result<Vec<String>, String> {
    use crate::ai::ollama::OllamaClient;
    
    let client = OllamaClient::new(url, None);
    client.list_models()
        .await
        .map_err(|e| format!("Failed to list Ollama models: {}", e))
}

/// Set the preferred AI provider
#[tauri::command]
pub async fn set_ai_provider(
    provider: String,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let router = state.ai_router.read().await;
    router.set_preferred_provider(&provider).await;
    
    tracing::info!("AI provider set to: {}", provider);
    Ok(())
}

/// Get AI provider status
#[tauri::command]
pub async fn get_ai_status(
    state: State<'_, AppState>,
) -> Result<ProviderStatus, String> {
    let router = state.ai_router.read().await;
    Ok(router.get_status().await)
}

/// Analyze with the best available AI provider
#[tauri::command]
pub async fn ai_analyze_with_router(
    state: State<'_, AppState>,
) -> Result<AIAnalysis, String> {
    // Get current schematic
    let schematic = {
        let current_schematic = state.current_schematic.lock()
            .map_err(|e| format!("Failed to lock current_schematic: {}", e))?;
        current_schematic.clone()
    };
    
    let schematic = schematic.ok_or_else(|| "No schematic loaded. Please open a project first.".to_string())?;
    
    // Get existing issues
    let existing_issues = {
        let issues = state.issues.lock()
            .map_err(|e| format!("Failed to lock issues: {}", e))?;
        issues.clone()
    };
    
    // Build context
    let context = build_schematic_context(&schematic, &existing_issues);
    
    // Use router to analyze
    let router = state.ai_router.read().await;
    router.analyze_schematic(&context)
        .await
        .map_err(|e| format!("AI analysis failed: {}", e))
}

/// Ask a question using the best available AI provider
#[tauri::command]
pub async fn ask_ai_with_router(
    question: String,
    state: State<'_, AppState>,
) -> Result<String, String> {
    // Get current schematic
    let schematic = {
        let current_schematic = state.current_schematic.lock()
            .map_err(|e| format!("Failed to lock current_schematic: {}", e))?;
        current_schematic.clone()
    };
    
    let schematic = schematic.ok_or_else(|| "No schematic loaded. Please open a project first.".to_string())?;
    
    // Get existing issues
    let existing_issues = {
        let issues = state.issues.lock()
            .map_err(|e| format!("Failed to lock issues: {}", e))?;
        issues.clone()
    };
    
    // Build context
    let context = build_schematic_context(&schematic, &existing_issues);
    
    // Use router to ask
    let router = state.ai_router.read().await;
    router.ask_question(&context, &question)
        .await
        .map_err(|e| format!("AI question failed: {}", e))
}

/// Configure Claude API key through the router
#[tauri::command]
pub async fn configure_claude(
    api_key: String,
    state: State<'_, AppState>,
) -> Result<(), String> {
    // Set in router (single source of truth)
    {
        let mut router = state.ai_router.write().await;
        router.set_claude_api_key(api_key);
    }
    
    tracing::info!("Claude API key configured");
    Ok(())
}

/// Get list of supported ICs with datasheet information
#[tauri::command]
pub async fn get_supported_datasheets() -> Result<Vec<DatasheetInfo>, String> {
    use crate::datasheets::builtin::get_all_datasheets;
    
    let datasheets = get_all_datasheets();
    let info: Vec<DatasheetInfo> = datasheets.iter().map(|ds| {
        DatasheetInfo {
            part_numbers: ds.part_numbers.clone(),
            manufacturer: ds.manufacturer.clone(),
            category: format!("{:?}", ds.category),
            datasheet_url: ds.datasheet_url.clone(),
        }
    }).collect();
    
    Ok(info)
}

/// Upload a user datasheet file
#[tauri::command]
pub async fn upload_datasheet(
    file_path: String,
    app_handle: tauri::AppHandle,
) -> Result<String, String> {
    use crate::datasheets::builtin::load_datasheet_from_file;
    use std::fs;
    
    // Get app data directory
    let app_data_dir = app_handle.path().app_data_dir()
        .map_err(|e| format!("Failed to get app data directory: {}", e))?;
    let user_datasheets_dir = app_data_dir.join("datasheets");
    
    // Create directory if it doesn't exist
    fs::create_dir_all(&user_datasheets_dir)
        .map_err(|e| format!("Failed to create datasheets directory: {}", e))?;
    
    // Read and validate the file
    let source_path = Path::new(&file_path);
    if !source_path.exists() {
        return Err(format!("File does not exist: {}", file_path));
    }
    
    // Validate JSON structure
    let datasheet = load_datasheet_from_file(source_path)
        .map_err(|e| format!("Invalid datasheet JSON: {}", e))?;
    
    // Generate filename from first part number or use original filename
    let filename = if let Some(first_part) = datasheet.part_numbers.first() {
        format!("{}.json", first_part.replace(" ", "_").replace("/", "_"))
    } else {
        source_path.file_name()
            .and_then(|n| n.to_str())
            .map(|s| s.to_string())
            .unwrap_or_else(|| "datasheet.json".to_string())
    };
    
    // Copy file to user datasheets directory
    let dest_path = user_datasheets_dir.join(&filename);
    fs::copy(source_path, &dest_path)
        .map_err(|e| format!("Failed to copy file: {}", e))?;
    
    tracing::info!("Uploaded datasheet: {} -> {}", file_path, dest_path.display());
    
    Ok(format!("Datasheet uploaded successfully: {}", filename))
}

/// Get list of user-uploaded datasheets
#[tauri::command]
pub async fn get_user_datasheets(
    app_handle: tauri::AppHandle,
) -> Result<Vec<UserDatasheetInfo>, String> {
    use crate::datasheets::builtin::load_datasheets_from_directory;
    use std::fs;
    
    // Get app data directory
    let app_data_dir = app_handle.path().app_data_dir()
        .map_err(|e| format!("Failed to get app data directory: {}", e))?;
    let user_datasheets_dir = app_data_dir.join("datasheets");
    
    // Create directory if it doesn't exist
    fs::create_dir_all(&user_datasheets_dir)
        .map_err(|e| format!("Failed to create datasheets directory: {}", e))?;
    
    // Load datasheets from user directory
    let (datasheets, errors) = load_datasheets_from_directory(&user_datasheets_dir);
    
    // Log any errors but don't fail
    for error in &errors {
        tracing::warn!("{}", error);
    }
    
    // Build a map of part numbers to filenames by reading directory
    let mut part_to_filename: std::collections::HashMap<String, String> = std::collections::HashMap::new();
    if let Ok(entries) = fs::read_dir(&user_datasheets_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().map(|e| e == "json").unwrap_or(false) {
                if let Ok(content) = fs::read_to_string(&path) {
                    if let Ok(ds) = serde_json::from_str::<crate::datasheets::schema::DatasheetRequirements>(&content) {
                        let filename = path.file_name()
                            .and_then(|n| n.to_str())
                            .map(|s| s.to_string())
                            .unwrap_or_default();
                        for pn in &ds.part_numbers {
                            part_to_filename.insert(pn.clone(), filename.clone());
                        }
                    }
                }
            }
        }
    }
    
    let info: Vec<UserDatasheetInfo> = datasheets.iter().map(|ds| {
        let filename = ds.part_numbers.first()
            .and_then(|pn| part_to_filename.get(pn))
            .cloned()
            .unwrap_or_default();
        
        UserDatasheetInfo {
            filename,
            part_numbers: ds.part_numbers.clone(),
            manufacturer: ds.manufacturer.clone(),
            category: format!("{:?}", ds.category),
            datasheet_url: ds.datasheet_url.clone(),
        }
    }).collect();
    
    Ok(info)
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UserDatasheetInfo {
    pub filename: String,
    pub part_numbers: Vec<String>,
    pub manufacturer: String,
    pub category: String,
    pub datasheet_url: Option<String>,
}

/// Delete a user-uploaded datasheet
#[tauri::command]
pub async fn delete_datasheet(
    filename: String,
    app_handle: tauri::AppHandle,
) -> Result<String, String> {
    use std::fs;
    use std::path::Path;
    
    // Get app data directory
    let app_data_dir = app_handle.path().app_data_dir()
        .map_err(|e| format!("Failed to get app data directory: {}", e))?;
    let user_datasheets_dir = app_data_dir.join("datasheets");
    
    // Security: Ensure filename doesn't contain path traversal
    let filename_clean = Path::new(&filename)
        .file_name()
        .and_then(|n| n.to_str())
        .ok_or_else(|| "Invalid filename".to_string())?;
    
    // Only allow .json files
    if !filename_clean.ends_with(".json") {
        return Err("Only JSON files are allowed".to_string());
    }
    
    let file_path = user_datasheets_dir.join(filename_clean);
    
    // Security: Ensure file is within user datasheets directory
    if !file_path.starts_with(&user_datasheets_dir) {
        return Err("Invalid file path".to_string());
    }
    
    // Delete the file
    fs::remove_file(&file_path)
        .map_err(|e| format!("Failed to delete file: {}", e))?;
    
    tracing::info!("Deleted datasheet: {}", file_path.display());
    
    Ok(format!("Datasheet deleted: {}", filename_clean))
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DatasheetInfo {
    pub part_numbers: Vec<String>,
    pub manufacturer: String,
    pub category: String,
    pub datasheet_url: Option<String>,
}

// Helper function to build schematic context for AI
fn build_schematic_context(schematic: &Schematic, issues: &[Issue]) -> SchematicContext {
    // Build components summary
    let components_summary = format!(
        "Schematic: {} with {} components, {} wires, {} labels",
        schematic.filename,
        schematic.components.len(),
        schematic.wires.len(),
        schematic.labels.len()
    );
    
    // Extract power rails from labels and power symbols
    let mut power_rails: Vec<String> = schematic.power_symbols
        .iter()
        .map(|p| p.value.clone())
        .collect();
    
    for label in &schematic.labels {
        let upper = label.text.to_uppercase();
        if upper.contains("VCC") || upper.contains("VDD") || upper.contains("GND") || 
           upper.contains("3V3") || upper.contains("5V") || upper.contains("12V") {
            if !power_rails.contains(&label.text) {
                power_rails.push(label.text.clone());
            }
        }
    }
    power_rails.sort();
    power_rails.dedup();
    
    // Extract signal nets from labels
    let signal_nets: Vec<String> = schematic.labels
        .iter()
        .filter(|l| {
            let upper = l.text.to_uppercase();
            !upper.contains("VCC") && !upper.contains("VDD") && !upper.contains("GND")
        })
        .map(|l| l.text.clone())
        .collect();
    
    // Build component details
    let component_details: Vec<ComponentDetail> = schematic.components
        .iter()
        .map(|c| ComponentDetail {
            reference: c.reference.clone(),
            value: c.value.clone(),
            lib_id: c.lib_id.clone(),
        })
        .collect();
    
    SchematicContext {
        components_summary,
        power_rails,
        signal_nets,
        detected_issues: issues.to_vec(),
        component_count: schematic.components.len(),
        component_details,
    }
}

// ============================================================================
// UCS (Unified Circuit Schema) Commands
// ============================================================================

/// Get the current circuit as UCS JSON
#[tauri::command]
pub async fn get_circuit_ucs(
    state: State<'_, AppState>,
) -> Result<UnifiedCircuitSchema, String> {
    let circuit = {
        let current_circuit = state.current_circuit.lock()
            .map_err(|e| format!("Failed to lock current_circuit: {}", e))?;
        current_circuit.clone()
    };
    
    let circuit = circuit.ok_or_else(|| "No circuit loaded. Please open a project first.".to_string())?;
    
    Ok(circuit.to_ucs())
}

/// Get circuit statistics
#[tauri::command]
pub async fn get_circuit_stats(
    state: State<'_, AppState>,
) -> Result<crate::ucs::circuit::CircuitStats, String> {
    let circuit = {
        let current_circuit = state.current_circuit.lock()
            .map_err(|e| format!("Failed to lock current_circuit: {}", e))?;
        current_circuit.clone()
    };
    
    let circuit = circuit.ok_or_else(|| "No circuit loaded. Please open a project first.".to_string())?;
    
    Ok(circuit.stats())
}

/// Get AI-friendly circuit summary
#[tauri::command]
pub async fn get_circuit_ai_summary(
    state: State<'_, AppState>,
) -> Result<AiCircuitSummary, String> {
    let circuit = {
        let current_circuit = state.current_circuit.lock()
            .map_err(|e| format!("Failed to lock current_circuit: {}", e))?;
        current_circuit.clone()
    };
    
    let circuit = circuit.ok_or_else(|| "No circuit loaded. Please open a project first.".to_string())?;
    
    Ok(analysis::create_ai_summary(&circuit))
}

/// Get a filtered UCS slice for specific components (for AI analysis)
#[tauri::command]
pub async fn get_circuit_slice(
    component_refs: Vec<String>,
    state: State<'_, AppState>,
) -> Result<UnifiedCircuitSchema, String> {
    let circuit = {
        let current_circuit = state.current_circuit.lock()
            .map_err(|e| format!("Failed to lock current_circuit: {}", e))?;
        current_circuit.clone()
    };
    
    let circuit = circuit.ok_or_else(|| "No circuit loaded. Please open a project first.".to_string())?;
    
    let refs: Vec<&str> = component_refs.iter().map(|s| s.as_str()).collect();
    Ok(circuit.create_ai_slice(&refs))
}

/// Analyze decoupling capacitors in the circuit
#[tauri::command]
pub async fn analyze_circuit_decoupling(
    max_distance_mm: Option<f64>,
    state: State<'_, AppState>,
) -> Result<DecouplingAnalysisResponse, String> {
    let circuit = {
        let current_circuit = state.current_circuit.lock()
            .map_err(|e| format!("Failed to lock current_circuit: {}", e))?;
        current_circuit.clone()
    };
    
    let circuit = circuit.ok_or_else(|| "No circuit loaded. Please open a project first.".to_string())?;
    
    let result = analysis::analyze_decoupling(&circuit, max_distance_mm.unwrap_or(20.0));
    
    Ok(DecouplingAnalysisResponse {
        ics_with_decoupling: result.ic_decoupling.len(),
        ics_missing_decoupling: result.missing_decoupling.len(),
        missing_details: result.missing_decoupling.iter().map(|m| {
            MissingDecouplingInfo {
                ic_ref: m.ic_ref.clone(),
                ic_value: m.ic_value.clone(),
                recommendation: m.recommendation.clone(),
            }
        }).collect(),
    })
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DecouplingAnalysisResponse {
    pub ics_with_decoupling: usize,
    pub ics_missing_decoupling: usize,
    pub missing_details: Vec<MissingDecouplingInfo>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct MissingDecouplingInfo {
    pub ic_ref: String,
    pub ic_value: Option<String>,
    pub recommendation: String,
}

/// Analyze circuit connectivity
#[tauri::command]
pub async fn analyze_circuit_connectivity(
    state: State<'_, AppState>,
) -> Result<ConnectivityAnalysisResponse, String> {
    let circuit = {
        let current_circuit = state.current_circuit.lock()
            .map_err(|e| format!("Failed to lock current_circuit: {}", e))?;
        current_circuit.clone()
    };
    
    let circuit = circuit.ok_or_else(|| "No circuit loaded. Please open a project first.".to_string())?;
    
    let result = analysis::analyze_connectivity(&circuit);
    
    Ok(ConnectivityAnalysisResponse {
        floating_components: result.floating_components,
        single_connection_nets: result.single_connection_nets,
        power_rail_count: result.power_connections.len(),
        ground_rail_count: result.ground_connections.len(),
    })
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ConnectivityAnalysisResponse {
    pub floating_components: Vec<String>,
    pub single_connection_nets: Vec<String>,
    pub power_rail_count: usize,
    pub ground_rail_count: usize,
}

/// Analyze signal integrity concerns
#[tauri::command]
pub async fn analyze_circuit_signals(
    state: State<'_, AppState>,
) -> Result<SignalAnalysisResponse, String> {
    let circuit = {
        let current_circuit = state.current_circuit.lock()
            .map_err(|e| format!("Failed to lock current_circuit: {}", e))?;
        current_circuit.clone()
    };
    
    let circuit = circuit.ok_or_else(|| "No circuit loaded. Please open a project first.".to_string())?;
    
    let result = analysis::analyze_signal_integrity(&circuit);
    
    Ok(SignalAnalysisResponse {
        i2c_buses: result.i2c_without_pullups.iter().map(|i| {
            I2cBusResponse {
                sda_net: i.sda_net.clone(),
                scl_net: i.scl_net.clone(),
                has_pullups: i.has_pullups,
                device_count: i.connected_devices.len(),
            }
        }).collect(),
        unterminated_signal_count: result.unterminated_signals.len(),
        unterminated_signals: result.unterminated_signals.iter().map(|s| s.net_name.clone()).collect(),
    })
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SignalAnalysisResponse {
    pub i2c_buses: Vec<I2cBusResponse>,
    pub unterminated_signal_count: usize,
    pub unterminated_signals: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct I2cBusResponse {
    pub sda_net: Option<String>,
    pub scl_net: Option<String>,
    pub has_pullups: bool,
    pub device_count: usize,
}

/// Get components connected to a specific net.
/// Uses UCS circuit when available; otherwise falls back to schematic netlist.
#[tauri::command]
pub async fn get_net_components(
    net_name: String,
    state: State<'_, AppState>,
) -> Result<Vec<ComponentInfo>, String> {
    let (circuit, schematic) = {
        let current_circuit = state.current_circuit.lock()
            .map_err(|e| format!("Failed to lock current_circuit: {}", e))?;
        let current_schematic = state.current_schematic.lock()
            .map_err(|e| format!("Failed to lock current_schematic: {}", e))?;
        (current_circuit.clone(), current_schematic.clone())
    };

    if let Some(ref circ) = circuit {
        let components = circ.components_on_net(&net_name);
        // Only use UCS result when it has components; otherwise fall back to schematic netlist
        if !components.is_empty() {
            return Ok(components.iter().map(|c| ComponentInfo {
                ref_des: c.ref_des.clone(),
                value: c.value.clone(),
                mpn: c.mpn.clone(),
                is_ic: c.is_ic(),
            }).collect());
        }
    }

    let schematic = schematic.ok_or_else(|| "No project loaded. Please open a project first.".to_string())?;

    let pin_to_net = NetlistBuilder::build_netlist(&schematic);
    let mut refs: std::collections::HashSet<String> = std::collections::HashSet::new();
    for (_key, conns) in &pin_to_net {
        for c in conns {
            if c.net_name.eq_ignore_ascii_case(&net_name) {
                refs.insert(c.component_ref.clone());
            }
        }
    }
    if refs.is_empty() {
        return Err(format!("Net '{}' not found or has no components.", net_name));
    }

    let mut out = Vec::new();
    for ref_des in refs {
        let comp = schematic.components.iter()
            .chain(schematic.power_symbols.iter())
            .find(|c| c.reference.eq_ignore_ascii_case(&ref_des) || c.reference.ends_with(&format!(":{}", ref_des)) || ref_des.ends_with(&format!(":{}", c.reference)));
        let (value, mpn) = comp
            .map(|c| (Some(c.value.clone()), c.properties.get("Datasheet").cloned()))
            .unwrap_or((None, None));
        let is_ic = ref_des.starts_with('U') || ref_des.starts_with("IC") || ref_des.starts_with("Q");
        out.push(ComponentInfo {
            ref_des,
            value: if value.as_deref().unwrap_or("").is_empty() { None } else { value },
            mpn,
            is_ic,
        });
    }
    out.sort_by(|a, b| a.ref_des.cmp(&b.ref_des));
    Ok(out)
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ComponentInfo {
    pub ref_des: String,
    pub value: Option<String>,
    pub mpn: Option<String>,
    pub is_ic: bool,
}

/// Get nets connected to a specific component.
/// Uses UCS circuit when available; otherwise falls back to schematic netlist.
#[tauri::command]
pub async fn get_component_nets(
    ref_des: String,
    state: State<'_, AppState>,
) -> Result<Vec<NetInfo>, String> {
    let (circuit, schematic) = {
        let current_circuit = state.current_circuit.lock()
            .map_err(|e| format!("Failed to lock current_circuit: {}", e))?;
        let current_schematic = state.current_schematic.lock()
            .map_err(|e| format!("Failed to lock current_schematic: {}", e))?;
        (current_circuit.clone(), current_schematic.clone())
    };

    if let Some(ref circ) = circuit {
        let nets = circ.nets_for_component(&ref_des);
        // Only use UCS result when it has nets; otherwise fall back to schematic netlist
        // (UCS graph often has no component-net edges because schematic.nets has empty connections)
        if !nets.is_empty() {
            return Ok(nets.iter().map(|n| NetInfo {
                name: n.net_name.clone(),
                voltage_level: n.voltage_level,
                is_power_rail: n.is_power_rail,
                signal_type: format!("{:?}", n.signal_type),
                connection_count: n.connections.len(),
            }).collect());
        }
    }

    let schematic = schematic.ok_or_else(|| "No project loaded. Please open a project first.".to_string())?;

    let pin_to_net = NetlistBuilder::build_netlist(&schematic);
    let mut net_names: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
    // Key is "comp_ref:pin_number" (comp_ref may contain ':' in hierarchical refs e.g. "Sheet1:PWR1")
    fn comp_ref_from_key(key: &str) -> &str {
        key.rsplitn(2, ':').next().unwrap_or("")
    }
    fn comp_ref_matches(stored: &str, query: &str) -> bool {
        stored.eq_ignore_ascii_case(query)
            || stored.ends_with(&format!(":{}", query))
            || query.ends_with(&format!(":{}", stored))
    }
    let ref_trim = ref_des.trim();
    for (key, conns) in &pin_to_net {
        let comp_ref = comp_ref_from_key(key);
        // Match by key prefix so "PWR1" matches "PWR1:1"; normalize spaces (schematic may have "PWR 1")
        let key_matches = key.starts_with(&format!("{}:", ref_trim))
            || key.starts_with(&format!("{}:", ref_trim.replace(' ', "")))
            || key.eq_ignore_ascii_case(ref_trim);
        if key_matches || comp_ref_matches(comp_ref, ref_trim) {
            for c in conns {
                *net_names.entry(c.net_name.clone()).or_insert(0) += 1;
            }
        }
    }
    let mut out: Vec<NetInfo> = net_names.into_iter().map(|(name, count)| {
        let name_upper = name.to_uppercase();
        let is_power_rail = name_upper.contains("GND") || name_upper.contains("VCC") || name_upper.contains("VDD") || name_upper.starts_with("V+");
        NetInfo {
            name,
            voltage_level: None,
            is_power_rail,
            signal_type: if is_power_rail { "Power".to_string() } else { "Signal".to_string() },
            connection_count: count,
        }
    }).collect();
    out.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(out)
}

#[derive(Debug, Serialize, Deserialize)]
pub struct NetInfo {
    pub name: String,
    pub voltage_level: Option<f64>,
    pub is_power_rail: bool,
    pub signal_type: String,
    pub connection_count: usize,
}

/// Parse a file using the UCS adapter registry
#[tauri::command]
pub async fn parse_file_to_ucs(
    file_path: String,
) -> Result<UnifiedCircuitSchema, String> {
    let registry = AdapterRegistry::new();
    let path = Path::new(&file_path);
    
    registry.parse_file(path)
        .map_err(|e| format!("Failed to parse file: {}", e))
}

/// Get list of supported file formats
#[tauri::command]
pub async fn get_supported_formats() -> Result<Vec<String>, String> {
    let registry = AdapterRegistry::new();
    Ok(registry.supported_extensions().iter().map(|s| s.to_string()).collect())
}

// ============================================================================
// Component Role Classification Commands (Phi-3 via Ollama)
// ============================================================================

use crate::ai::{ComponentRoleClassifier, ComponentInput, ClassificationResult, ComponentRole};

/// Classify a single component's role using Phi-3
#[tauri::command]
pub async fn classify_component_role(
    ref_des: String,
    part_number: String,
    lib_id: Option<String>,
) -> Result<ClassificationResult, String> {
    let classifier = ComponentRoleClassifier::new();
    
    // Check if Ollama/Phi-3 is available
    if !classifier.is_available().await {
        return Err("Phi-3 model not available. Please ensure Ollama is running with phi3 model installed. Run: ollama pull phi3".to_string());
    }
    
    let mut input = ComponentInput::new(&ref_des, &part_number);
    if let Some(lib) = lib_id {
        input = input.with_lib_id(lib);
    }
    
    classifier.classify(&input)
        .await
        .map_err(|e| format!("Classification failed: {}", e))
}

/// Classify multiple components' roles in batch
#[tauri::command]
pub async fn classify_components_batch(
    components: Vec<ComponentInputDto>,
) -> Result<Vec<ClassificationResult>, String> {
    let classifier = ComponentRoleClassifier::new();
    
    // Check if Ollama/Phi-3 is available
    if !classifier.is_available().await {
        return Err("Phi-3 model not available. Please ensure Ollama is running with phi3 model installed. Run: ollama pull phi3".to_string());
    }
    
    let inputs: Vec<ComponentInput> = components.into_iter().map(|c| {
        let mut input = ComponentInput::new(&c.ref_des, &c.part_number);
        if let Some(lib) = c.lib_id {
            input = input.with_lib_id(lib);
        }
        if let Some(fp) = c.footprint {
            input = input.with_footprint(fp);
        }
        input
    }).collect();
    
    classifier.classify_batch(&inputs)
        .await
        .map_err(|e| format!("Batch classification failed: {}", e))
}

/// DTO for component input from frontend
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComponentInputDto {
    pub ref_des: String,
    pub part_number: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub lib_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub footprint: Option<String>,
}

/// Classify all components in the current schematic
#[tauri::command]
pub async fn classify_schematic_components(
    state: State<'_, AppState>,
) -> Result<Vec<ClassificationResult>, String> {
    let classifier = ComponentRoleClassifier::new();
    
    // Check if Ollama/Phi-3 is available
    if !classifier.is_available().await {
        // Get available models to provide a helpful error message
        let available_models = classifier.list_available_models().await
            .unwrap_or_else(|_| vec![]);
        
        let mut error_msg = format!(
            "Phi-3 model not available. Looking for model: '{}'",
            classifier.model()
        );
        
        if !available_models.is_empty() {
            error_msg.push_str(&format!(
                "\n\nAvailable models: {}\n\nIf you have a phi3 variant installed, try configuring it manually.",
                available_models.join(", ")
            ));
        } else {
            error_msg.push_str("\n\nNo models found in Ollama. Please ensure Ollama is running and install a model:\n  ollama pull phi3");
        }
        
        return Err(error_msg);
    }
    
    // Get current schematic
    let schematic = {
        let current_schematic = state.current_schematic.lock()
            .map_err(|e| format!("Failed to lock current_schematic: {}", e))?;
        current_schematic.clone()
    };
    
    let schematic = schematic.ok_or_else(|| "No schematic loaded. Please open a project first.".to_string())?;
    
    // Build component inputs from schematic
    let inputs: Vec<ComponentInput> = schematic.components
        .iter()
        .map(|c| {
            ComponentInput::new(&c.reference, &c.value)
                .with_lib_id(&c.lib_id)
        })
        .collect();
    
    if inputs.is_empty() {
        return Ok(vec![]);
    }
    
    tracing::info!("Classifying {} components using Phi-3", inputs.len());
    
    classifier.classify_batch(&inputs)
        .await
        .map_err(|e| format!("Classification failed: {}", e))
}

/// Configure the classifier with a custom Ollama URL and model
#[tauri::command]
pub async fn configure_classifier(
    url: Option<String>,
    model: Option<String>,
) -> Result<ClassifierConfig, String> {
    let classifier = ComponentRoleClassifier::with_config(url.clone(), model.clone());
    
    let available = classifier.is_available().await;
    
    Ok(ClassifierConfig {
        url: url.unwrap_or_else(|| "http://localhost:11434".to_string()),
        model: model.unwrap_or_else(|| "phi3".to_string()),
        available,
    })
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ClassifierConfig {
    pub url: String,
    pub model: String,
    pub available: bool,
}

/// Check if the classifier (Phi-3 via Ollama) is available
#[tauri::command]
pub async fn check_classifier_available() -> Result<bool, String> {
    let classifier = ComponentRoleClassifier::new();
    Ok(classifier.is_available().await)
}

/// Get all available component role categories
#[tauri::command]
pub async fn get_component_role_categories() -> Result<Vec<RoleCategoryInfo>, String> {
    let categories = vec![
        ("Power Management", vec![
            ComponentRole::BuckRegulator,
            ComponentRole::BoostRegulator,
            ComponentRole::LDORegulator,
            ComponentRole::DecouplingCapacitor,
            ComponentRole::BulkCapacitor,
        ]),
        ("Microcontrollers & Processors", vec![
            ComponentRole::MCU,
            ComponentRole::MCU_GPIO,
            ComponentRole::MCU_I2C,
            ComponentRole::MCU_SPI,
            ComponentRole::FPGA,
        ]),
        ("Communication", vec![
            ComponentRole::I2C_Slave,
            ComponentRole::I2C_Master,
            ComponentRole::SPI_Slave,
            ComponentRole::UART_Transceiver,
            ComponentRole::USB_Controller,
            ComponentRole::CAN_Transceiver,
        ]),
        ("Analog", vec![
            ComponentRole::OpAmp,
            ComponentRole::Comparator,
            ComponentRole::ADC,
            ComponentRole::DAC,
            ComponentRole::VoltageReference,
        ]),
        ("Timing & Oscillators", vec![
            ComponentRole::Crystal,
            ComponentRole::Oscillator,
            ComponentRole::RTC,
            ComponentRole::TimerIC,
        ]),
        ("Sensors", vec![
            ComponentRole::TemperatureSensor,
            ComponentRole::AccelerometerGyro,
            ComponentRole::CurrentSensor,
        ]),
        ("Memory", vec![
            ComponentRole::EEPROM,
            ComponentRole::Flash,
            ComponentRole::SRAM,
        ]),
        ("Protection", vec![
            ComponentRole::TVS_Diode,
            ComponentRole::Fuse,
            ComponentRole::ESD_Protection,
        ]),
        ("Passive Components", vec![
            ComponentRole::PullUpResistor,
            ComponentRole::PullDownResistor,
            ComponentRole::VoltageDivider,
            ComponentRole::TerminationResistor,
        ]),
        ("Display & Indicators", vec![
            ComponentRole::LED_Indicator,
            ComponentRole::LED_Driver,
            ComponentRole::LCD_Display,
        ]),
    ];
    
    Ok(categories.into_iter().map(|(name, roles)| {
        RoleCategoryInfo {
            category: name.to_string(),
            roles: roles.into_iter().map(|r| RoleInfo {
                name: format!("{}", r),
                description: r.description().to_string(),
            }).collect(),
        }
    }).collect())
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RoleCategoryInfo {
    pub category: String,
    pub roles: Vec<RoleInfo>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RoleInfo {
    pub name: String,
    pub description: String,
}

// ============================================================================
// PCB Compliance Commands - IPC-2221, EMI Analysis, Custom Rules
// ============================================================================
use crate::compliance::{
    ipc2221::{Ipc2221Calculator, CurrentCapacityReport, CurrentCapacityIssue, check_power_traces, generate_current_report},
    emi::{EmiIssue, EmiReport, generate_emi_report},
    net_classifier::{NetClassificationSummary, generate_classification_summary},
    rules::{CustomRulesEngine, RuleSet, RuleViolation, generate_sample_rules},
};

/// Open and parse a PCB file (.kicad_pcb)
#[tauri::command]
pub async fn open_pcb(
    path: String,
    state: State<'_, AppState>,
) -> Result<PcbSummary, String> {
    let pcb_path = PathBuf::from(&path);
    
    if !pcb_path.exists() {
        return Err(format!("PCB file does not exist: {}", path));
    }
    
    let pcb = PcbParser::parse_pcb(&pcb_path)
        .map_err(|e| format!("Failed to parse PCB: {}", e))?;
    
    let summary = PcbSummary {
        filename: pcb.filename.clone(),
        trace_count: pcb.traces.len(),
        via_count: pcb.vias.len(),
        zone_count: pcb.zones.len(),
        footprint_count: pcb.footprints.len(),
        net_count: pcb.nets.len(),
        layer_count: pcb.layers.len(),
        board_thickness_mm: pcb.general.thickness,
    };
    
    // Store PCB in state
    {
        let mut current_pcb = state.current_pcb.lock()
            .map_err(|e| format!("Failed to lock current_pcb: {}", e))?;
        *current_pcb = Some(pcb);
    }
    
    tracing::info!("Opened PCB: {} ({} traces, {} vias)", summary.filename, summary.trace_count, summary.via_count);
    Ok(summary)
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PcbSummary {
    pub filename: String,
    pub trace_count: usize,
    pub via_count: usize,
    pub zone_count: usize,
    pub footprint_count: usize,
    pub net_count: usize,
    pub layer_count: usize,
    pub board_thickness_mm: f64,
}

/// Run IPC-2221 current capacity analysis on all traces
#[tauri::command]
pub async fn analyze_ipc2221(
    temp_rise_c: Option<f64>,
    state: State<'_, AppState>,
) -> Result<CurrentCapacityReport, String> {
    let pcb = {
        let current_pcb = state.current_pcb.lock()
            .map_err(|e| format!("Failed to lock current_pcb: {}", e))?;
        current_pcb.clone()
    };
    
    let pcb = pcb.ok_or_else(|| "No PCB loaded. Please open a PCB file first.".to_string())?;
    
    let report = generate_current_report(&pcb, temp_rise_c.unwrap_or(10.0));
    
    tracing::info!("IPC-2221 analysis complete: {} traces analyzed", report.trace_analyses.len());
    Ok(report)
}

/// Check power traces against expected currents
#[tauri::command]
pub async fn check_power_trace_capacity(
    power_nets: Vec<PowerNetSpec>,
    temp_rise_c: Option<f64>,
    state: State<'_, AppState>,
) -> Result<Vec<CurrentCapacityIssue>, String> {
    let pcb = {
        let current_pcb = state.current_pcb.lock()
            .map_err(|e| format!("Failed to lock current_pcb: {}", e))?;
        current_pcb.clone()
    };
    
    let pcb = pcb.ok_or_else(|| "No PCB loaded. Please open a PCB file first.".to_string())?;
    
    let net_currents: Vec<(String, f64)> = power_nets
        .into_iter()
        .map(|p| (p.net_name, p.expected_current_a))
        .collect();
    
    let issues = check_power_traces(&pcb, &net_currents, temp_rise_c.unwrap_or(10.0));
    
    tracing::info!("Power trace check: {} issues found", issues.len());
    Ok(issues)
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PowerNetSpec {
    pub net_name: String,
    pub expected_current_a: f64,
}

/// Calculate required trace width for a given current
#[tauri::command]
pub async fn calculate_trace_width(
    current_a: f64,
    copper_oz: Option<f64>,
    temp_rise_c: Option<f64>,
    is_external: Option<bool>,
) -> Result<TraceWidthResult, String> {
    let calculator = Ipc2221Calculator::default();
    let copper_thickness = copper_oz.unwrap_or(1.0) * 0.035;
    let temp_rise = temp_rise_c.unwrap_or(10.0);
    let external = is_external.unwrap_or(true);
    
    let width_mm = calculator.calculate_required_width(
        current_a,
        copper_thickness,
        temp_rise,
        external,
    );
    
    Ok(TraceWidthResult {
        required_width_mm: width_mm,
        required_width_mils: width_mm / 0.0254,
        current_a,
        copper_oz: copper_oz.unwrap_or(1.0),
        temp_rise_c: temp_rise,
        is_external: external,
    })
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TraceWidthResult {
    pub required_width_mm: f64,
    pub required_width_mils: f64,
    pub current_a: f64,
    pub copper_oz: f64,
    pub temp_rise_c: f64,
    pub is_external: bool,
}

/// Run EMI analysis on the PCB
#[tauri::command]
pub async fn analyze_emi(
    state: State<'_, AppState>,
) -> Result<EmiReport, String> {
    let pcb = {
        let current_pcb = state.current_pcb.lock()
            .map_err(|e| format!("Failed to lock current_pcb: {}", e))?;
        current_pcb.clone()
    };
    
    let pcb = pcb.ok_or_else(|| "No PCB loaded. Please open a PCB file first.".to_string())?;
    
    let report = generate_emi_report(&pcb);
    
    tracing::info!("EMI analysis complete: {} issues ({} critical)", 
        report.total_issues, report.critical_count);
    Ok(report)
}

/// Classify all nets in the PCB
#[tauri::command]
pub async fn classify_pcb_nets(
    state: State<'_, AppState>,
) -> Result<NetClassificationSummary, String> {
    let pcb = {
        let current_pcb = state.current_pcb.lock()
            .map_err(|e| format!("Failed to lock current_pcb: {}", e))?;
        current_pcb.clone()
    };
    
    let pcb = pcb.ok_or_else(|| "No PCB loaded. Please open a PCB file first.".to_string())?;
    
    let summary = generate_classification_summary(&pcb);
    
    tracing::info!("Net classification: {} high-speed, {} clock signals", 
        summary.high_speed_count, summary.clock_count);
    Ok(summary)
}

/// Load custom rules from JSON
#[tauri::command]
pub async fn load_custom_rules(
    rules_json: String,
    state: State<'_, AppState>,
) -> Result<RulesLoadResult, String> {
    let mut engine = CustomRulesEngine::new();
    engine.load_rules_str(&rules_json)?;
    
    // Store engine in state
    {
        let mut rules_engine = state.custom_rules.lock()
            .map_err(|e| format!("Failed to lock custom_rules: {}", e))?;
        *rules_engine = Some(engine);
    }
    
    // Parse to get rule count
    let rule_set: RuleSet = serde_json::from_str(&rules_json)
        .map_err(|e| format!("Failed to parse rules: {}", e))?;
    
    Ok(RulesLoadResult {
        name: rule_set.name,
        version: rule_set.version,
        rule_count: rule_set.rules.len(),
        enabled_count: rule_set.rules.iter().filter(|r| r.enabled).count(),
    })
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RulesLoadResult {
    pub name: String,
    pub version: String,
    pub rule_count: usize,
    pub enabled_count: usize,
}

/// Run custom rules check on the PCB
#[tauri::command]
pub async fn check_custom_rules(
    state: State<'_, AppState>,
) -> Result<Vec<RuleViolation>, String> {
    let pcb = {
        let current_pcb = state.current_pcb.lock()
            .map_err(|e| format!("Failed to lock current_pcb: {}", e))?;
        current_pcb.clone()
    };
    
    let pcb = pcb.ok_or_else(|| "No PCB loaded. Please open a PCB file first.".to_string())?;
    
    let engine = {
        let rules_engine = state.custom_rules.lock()
            .map_err(|e| format!("Failed to lock custom_rules: {}", e))?;
        rules_engine.clone()
    };
    
    let engine = engine.ok_or_else(|| "No custom rules loaded. Please load rules first.".to_string())?;
    
    let violations = engine.check(&pcb);
    
    tracing::info!("Custom rules check: {} violations found", violations.len());
    Ok(violations)
}

/// Get sample rules.json template
#[tauri::command]
pub async fn get_sample_rules() -> Result<String, String> {
    let rules = generate_sample_rules();
    serde_json::to_string_pretty(&rules)
        .map_err(|e| format!("Failed to serialize rules: {}", e))
}

/// Run full PCB compliance audit (IPC-2221 + EMI + Custom Rules)
#[tauri::command]
pub async fn run_pcb_compliance_audit(
    temp_rise_c: Option<f64>,
    state: State<'_, AppState>,
) -> Result<ComplianceAuditReport, String> {
    let pcb = {
        let current_pcb = state.current_pcb.lock()
            .map_err(|e| format!("Failed to lock current_pcb: {}", e))?;
        current_pcb.clone()
    };
    
    let pcb = pcb.ok_or_else(|| "No PCB loaded. Please open a PCB file first.".to_string())?;
    
    // Run IPC-2221 analysis
    let ipc_report = generate_current_report(&pcb, temp_rise_c.unwrap_or(10.0));
    
    // Run EMI analysis
    let emi_report = generate_emi_report(&pcb);
    
    // Run net classification
    let net_summary = generate_classification_summary(&pcb);
    
    // Run custom rules if loaded
    let custom_violations = {
        let rules_engine = state.custom_rules.lock()
            .map_err(|e| format!("Failed to lock custom_rules: {}", e))?;
        
        if let Some(engine) = rules_engine.as_ref() {
            engine.check(&pcb)
        } else {
            Vec::new()
        }
    };
    
    let total_issues = emi_report.total_issues + custom_violations.len();
    let critical_issues = emi_report.critical_count + 
        custom_violations.iter().filter(|v| v.severity == crate::compliance::rules::RuleSeverity::Error).count();
    
    Ok(ComplianceAuditReport {
        pcb_filename: pcb.filename.clone(),
        total_issues,
        critical_issues,
        ipc2221_summary: Ipc2221Summary {
            traces_analyzed: ipc_report.trace_analyses.len(),
            nets_analyzed: ipc_report.net_summaries.len(),
            temp_rise_c: ipc_report.temp_rise_c,
            copper_oz: ipc_report.outer_copper_oz,
        },
        emi_summary: EmiSummary {
            total_issues: emi_report.total_issues,
            critical_count: emi_report.critical_count,
            high_count: emi_report.high_count,
            recommendations: emi_report.recommendations,
        },
        net_summary: NetSummary {
            total_nets: net_summary.total_nets,
            high_speed_count: net_summary.high_speed_count,
            clock_count: net_summary.clock_count,
            high_speed_nets: net_summary.high_speed_nets,
            clock_nets: net_summary.clock_nets,
        },
        custom_rule_violations: custom_violations.len(),
        emi_issues: emi_report.issues,
        custom_violations,
    })
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ComplianceAuditReport {
    pub pcb_filename: String,
    pub total_issues: usize,
    pub critical_issues: usize,
    pub ipc2221_summary: Ipc2221Summary,
    pub emi_summary: EmiSummary,
    pub net_summary: NetSummary,
    pub custom_rule_violations: usize,
    pub emi_issues: Vec<EmiIssue>,
    pub custom_violations: Vec<RuleViolation>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Ipc2221Summary {
    pub traces_analyzed: usize,
    pub nets_analyzed: usize,
    pub temp_rise_c: f64,
    pub copper_oz: f64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct EmiSummary {
    pub total_issues: usize,
    pub critical_count: usize,
    pub high_count: usize,
    pub recommendations: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct NetSummary {
    pub total_nets: usize,
    pub high_speed_count: usize,
    pub clock_count: usize,
    pub high_speed_nets: Vec<String>,
    pub clock_nets: Vec<String>,
}

/// Run Decoupling Risk Scoring (DRS) analysis
/// Requires both schematic and PCB files to be loaded
#[tauri::command]
pub async fn run_drs_analysis(
    state: State<'_, AppState>,
) -> Result<Vec<ICRiskScore>, String> {
    // Get schematic from state
    let schematic = {
        let current_schematic = state.current_schematic.lock()
            .map_err(|e| format!("Failed to lock current_schematic: {}", e))?;
        current_schematic.clone()
    };
    
    let schematic = schematic.ok_or_else(|| {
        "No schematic loaded. Please open a schematic file first.".to_string()
    })?;
    
    // Get PCB from state
    let pcb = {
        let current_pcb = state.current_pcb.lock()
            .map_err(|e| format!("Failed to lock current_pcb: {}", e))?;
        current_pcb.clone()
    };
    
    let pcb = pcb.ok_or_else(|| {
        "No PCB loaded. Please open a PCB file first.".to_string()
    })?;
    
    // Run DRS analysis
    let analyzer = DRSAnalyzer::new();
    let results = analyzer.analyze(&schematic, &pcb);
    
    tracing::info!("DRS analysis completed: {} ICs analyzed", results.len());
    
    Ok(results)
}

/// Run DRS analysis with explicit file paths
#[tauri::command]
pub async fn run_drs_analysis_from_files(
    schematic_path: String,
    pcb_path: String,
) -> Result<Vec<ICRiskScore>, String> {
    use crate::parser::kicad::KicadParser;
    
    // Parse schematic
    let schematic = KicadParser::parse_schematic(Path::new(&schematic_path))
        .map_err(|e| format!("Failed to parse schematic: {}", e))?;
    
    // Parse PCB
    let pcb = PcbParser::parse_pcb(Path::new(&pcb_path))
        .map_err(|e| format!("Failed to parse PCB: {}", e))?;
    
    // Run DRS analysis
    let analyzer = DRSAnalyzer::new();
    let results = analyzer.analyze(&schematic, &pcb);
    
    tracing::info!("DRS analysis completed: {} ICs analyzed", results.len());
    
    Ok(results)
}

/// Trace physical path from capacitor pad to IC power pin
#[tauri::command]
pub async fn trace_capacitor_to_ic_path(
    state: State<'_, AppState>,
    capacitor_ref: String,
    ic_ref: String,
    net_name: String,
) -> Result<PathAnalysis, String> {
    // Get schematic from state
    let schematic = {
        let current_schematic = state.current_schematic.lock()
            .map_err(|e| format!("Failed to lock current_schematic: {}", e))?;
        current_schematic.clone()
    };
    
    let schematic = schematic.ok_or_else(|| {
        "No schematic loaded. Please open a schematic file first.".to_string()
    })?;
    
    // Get PCB from state
    let pcb = {
        let current_pcb = state.current_pcb.lock()
            .map_err(|e| format!("Failed to lock current_pcb: {}", e))?;
        current_pcb.clone()
    };
    
    let pcb = pcb.ok_or_else(|| {
        "No PCB loaded. Please open a PCB file first.".to_string()
    })?;
    
    // Run path tracing
    let analyzer = DRSAnalyzer::new();
    analyzer.trace_capacitor_to_ic_path(&capacitor_ref, &ic_ref, &net_name, &pcb, &schematic)
        .map_err(|e| match e {
            PathError::ComponentNotFound(ref msg) => format!("Component not found: {}", msg),
            PathError::NetNotFound(ref msg) => format!("Net not found: {}", msg),
            PathError::NoPathFound => "No physical path found between capacitor and IC".to_string(),
            PathError::MultiplePathsFound => "Multiple paths found (not yet supported)".to_string(),
            PathError::InvalidNetConnection => "Invalid net connection".to_string(),
        })
}

/// Find all capacitor-to-IC paths for a given net
#[tauri::command]
pub async fn find_all_capacitor_ic_paths(
    state: State<'_, AppState>,
    net_name: String,
) -> Result<Vec<PathAnalysis>, String> {
    // Get schematic from state
    let schematic = {
        let current_schematic = state.current_schematic.lock()
            .map_err(|e| format!("Failed to lock current_schematic: {}", e))?;
        current_schematic.clone()
    };
    
    let schematic = schematic.ok_or_else(|| {
        "No schematic loaded. Please open a schematic file first.".to_string()
    })?;
    
    // Get PCB from state
    let pcb = {
        let current_pcb = state.current_pcb.lock()
            .map_err(|e| format!("Failed to lock current_pcb: {}", e))?;
        current_pcb.clone()
    };
    
    let pcb = pcb.ok_or_else(|| {
        "No PCB loaded. Please open a PCB file first.".to_string()
    })?;
    
    // Find all paths
    let analyzer = DRSAnalyzer::new();
    analyzer.find_all_capacitor_ic_paths(&net_name, &pcb, &schematic)
        .map_err(|e| match e {
            PathError::ComponentNotFound(ref msg) => format!("Component not found: {}", msg),
            PathError::NetNotFound(ref msg) => format!("Net not found: {}", msg),
            PathError::NoPathFound => "No paths found".to_string(),
            PathError::MultiplePathsFound => "Multiple paths found (not yet supported)".to_string(),
            PathError::InvalidNetConnection => "Invalid net connection".to_string(),
        })
}
