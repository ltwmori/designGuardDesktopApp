use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Mutex;
use tokio::sync::RwLock;
use crate::db::Database;
use crate::watcher::ProjectWatcher;
use crate::parser::schema::Schematic;
use crate::parser::pcb_schema::PcbDesign;
use crate::analyzer::rules::Issue;
use crate::ai::router::AIRouter;
use crate::ucs::Circuit;
use crate::compliance::rules::CustomRulesEngine;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Settings {
    pub auto_analyze: bool,
    pub ai_provider: String,
    pub theme: String,
    #[serde(default)]
    pub ollama_url: Option<String>,
    #[serde(default)]
    pub ollama_model: Option<String>,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            auto_analyze: false,
            ai_provider: "claude".to_string(),
            theme: "dark".to_string(),
            ollama_url: None,
            ollama_model: Some("llama3.1:8b".to_string()),
        }
    }
}

pub struct AppState {
    pub db: Database,
    pub watchers: RwLock<HashMap<String, ProjectWatcher>>,
    pub project_path: Mutex<Option<PathBuf>>,
    /// Legacy schematic representation (for backwards compatibility)
    pub current_schematic: Mutex<Option<Schematic>>,
    /// New UCS-based circuit representation with petgraph
    pub current_circuit: Mutex<Option<Circuit>>,
    /// PCB design for compliance checking
    pub current_pcb: Mutex<Option<PcbDesign>>,
    pub issues: Mutex<Vec<Issue>>,
    pub settings: Mutex<Settings>,
    pub ai_router: RwLock<AIRouter>,
    /// Custom rules engine for compliance checking
    pub custom_rules: Mutex<Option<CustomRulesEngine>>,
}

impl AppState {
    pub fn new(db: Database) -> Self {
        Self {
            db,
            watchers: RwLock::new(HashMap::new()),
            project_path: Mutex::new(None),
            current_schematic: Mutex::new(None),
            current_circuit: Mutex::new(None),
            current_pcb: Mutex::new(None),
            issues: Mutex::new(Vec::new()),
            settings: Mutex::new(Settings::default()),
            ai_router: RwLock::new(AIRouter::new()),
            custom_rules: Mutex::new(None),
        }
    }
}
