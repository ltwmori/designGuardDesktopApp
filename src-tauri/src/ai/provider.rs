//! AI Provider Trait
//!
//! Defines a common interface for AI providers (Claude, Ollama, etc.)

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use crate::ai::claude::AIAnalysis;
use crate::ai::AIError;
use crate::analyzer::rules::Issue;

/// Context about the schematic for AI analysis
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchematicContext {
    /// Summary of components in the schematic
    pub components_summary: String,
    
    /// List of power rails detected
    pub power_rails: Vec<String>,
    
    /// List of signal nets detected
    pub signal_nets: Vec<String>,
    
    /// Issues already detected by rule-based analysis
    pub detected_issues: Vec<Issue>,
    
    /// Total component count
    pub component_count: usize,
    
    /// Component details for more context
    pub component_details: Vec<ComponentDetail>,
}

/// Detail about a single component
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComponentDetail {
    pub reference: String,
    pub value: String,
    pub lib_id: String,
}

/// Information about an AI model
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelInfo {
    /// Provider name (e.g., "claude", "ollama")
    pub provider: String,
    
    /// Model name (e.g., "claude-sonnet-4-20250514", "llama3.1:8b")
    pub model_name: String,
    
    /// Whether this is a local model
    pub is_local: bool,
    
    /// Context window size in tokens
    pub context_window: usize,
    
    /// Whether the model reliably outputs JSON
    pub supports_json: bool,
}

/// Common trait for all AI providers
#[async_trait]
pub trait AIProvider: Send + Sync {
    /// Get the provider name
    fn name(&self) -> &str;
    
    /// Check if the provider is available/configured
    async fn is_available(&self) -> bool;
    
    /// Analyze a schematic
    async fn analyze_schematic(
        &self,
        context: &SchematicContext,
    ) -> Result<AIAnalysis, AIError>;
    
    /// Answer a question about the design
    async fn ask_question(
        &self,
        context: &SchematicContext,
        question: &str,
    ) -> Result<String, AIError>;
    
    /// Get model info
    fn model_info(&self) -> ModelInfo;
}

/// Status of AI providers
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderStatus {
    pub claude_available: bool,
    pub claude_configured: bool,
    pub ollama_available: bool,
    pub ollama_models: Vec<String>,
    pub preferred: String,
    pub active_provider: Option<String>,
}

impl Default for ProviderStatus {
    fn default() -> Self {
        Self {
            claude_available: false,
            claude_configured: false,
            ollama_available: false,
            ollama_models: vec![],
            preferred: "claude".to_string(),
            active_provider: None,
        }
    }
}
