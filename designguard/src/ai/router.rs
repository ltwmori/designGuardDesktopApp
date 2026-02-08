//! AI Router
//!
//! Intelligent routing between AI providers with fallback support.

use std::sync::Arc;
use tokio::sync::RwLock;

use crate::ai::claude::{AIAnalysis, ClaudeClient};
use crate::ai::ollama::OllamaClient;
use crate::ai::provider::{AIProvider, ModelInfo, ProviderStatus, SchematicContext};
use crate::ai::AIError;

/// Router that manages multiple AI providers
pub struct AIRouter {
    claude_client: Option<Arc<ClaudeClient>>,
    ollama_client: Option<Arc<OllamaClient>>,
    preferred_provider: RwLock<String>,
}

impl AIRouter {
    /// Create a new router with no providers configured
    pub fn new() -> Self {
        Self {
            claude_client: None,
            ollama_client: None,
            preferred_provider: RwLock::new("claude".to_string()),
        }
    }
    
    /// Configure the Claude client with an API key
    pub fn set_claude_api_key(&mut self, key: String) {
        if !key.is_empty() {
            self.claude_client = Some(Arc::new(ClaudeClient::new(key)));
        } else {
            self.claude_client = None;
        }
    }
    
    /// Configure the Ollama client
    pub fn set_ollama_config(&mut self, url: Option<String>, model: Option<String>) {
        self.ollama_client = Some(Arc::new(OllamaClient::new(url, model)));
    }
    
    /// Set the preferred provider
    pub async fn set_preferred_provider(&self, provider: &str) {
        let mut pref = self.preferred_provider.write().await;
        *pref = provider.to_string();
    }
    
    /// Get the preferred provider name
    pub async fn get_preferred_provider(&self) -> String {
        self.preferred_provider.read().await.clone()
    }
    
    /// Get the best available provider based on preference and availability
    pub async fn get_provider(&self) -> Option<Arc<dyn AIProvider>> {
        let preferred = self.preferred_provider.read().await.clone();
        
        match preferred.as_str() {
            "ollama" => {
                // Try Ollama first
                if let Some(ref client) = self.ollama_client {
                    if client.is_available().await {
                        return Some(client.clone() as Arc<dyn AIProvider>);
                    }
                }
                // Fallback to Claude
                if let Some(ref client) = self.claude_client {
                    return Some(client.clone() as Arc<dyn AIProvider>);
                }
            }
            "claude" | _ => {
                // Try Claude first
                if let Some(ref client) = self.claude_client {
                    return Some(client.clone() as Arc<dyn AIProvider>);
                }
                // Fallback to Ollama
                if let Some(ref client) = self.ollama_client {
                    if client.is_available().await {
                        return Some(client.clone() as Arc<dyn AIProvider>);
                    }
                }
            }
        }
        
        None
    }
    
    /// Analyze a schematic using the best available provider
    pub async fn analyze_schematic(
        &self,
        context: &SchematicContext,
    ) -> Result<AIAnalysis, AIError> {
        let provider = self.get_provider().await
            .ok_or(AIError::MissingApiKey)?;
        
        tracing::info!("Using AI provider: {}", provider.name());
        provider.analyze_schematic(context).await
    }
    
    /// Ask a question using the best available provider
    pub async fn ask_question(
        &self,
        context: &SchematicContext,
        question: &str,
    ) -> Result<String, AIError> {
        let provider = self.get_provider().await
            .ok_or(AIError::MissingApiKey)?;
        
        tracing::info!("Using AI provider: {} for question", provider.name());
        provider.ask_question(context, question).await
    }
    
    /// Get the status of all providers
    pub async fn get_status(&self) -> ProviderStatus {
        let preferred = self.preferred_provider.read().await.clone();
        
        let claude_configured = self.claude_client.is_some();
        let claude_available = claude_configured; // Claude is available if configured
        
        let ollama_available = if let Some(ref client) = self.ollama_client {
            client.is_available().await
        } else {
            false
        };
        
        let ollama_models = if let Some(ref client) = self.ollama_client {
            client.list_models().await.unwrap_or_default()
        } else {
            vec![]
        };
        
        let active_provider = if let Some(provider) = self.get_provider().await {
            Some(provider.name().to_string())
        } else {
            None
        };
        
        ProviderStatus {
            claude_available,
            claude_configured,
            ollama_available,
            ollama_models,
            preferred,
            active_provider,
        }
    }
    
    /// Get model info for the active provider
    pub async fn get_model_info(&self) -> Option<ModelInfo> {
        self.get_provider().await.map(|p| p.model_info())
    }
    
    /// Check if any provider is available
    pub async fn has_provider(&self) -> bool {
        self.get_provider().await.is_some()
    }
    
    /// List available Ollama models
    pub async fn list_ollama_models(&self) -> Result<Vec<String>, AIError> {
        if let Some(ref client) = self.ollama_client {
            client.list_models().await
        } else {
            Ok(vec![])
        }
    }
}

impl Default for AIRouter {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_router_no_providers() {
        let router = AIRouter::new();
        assert!(router.get_provider().await.is_none());
    }
    
    #[tokio::test]
    async fn test_router_with_claude() {
        let mut router = AIRouter::new();
        router.set_claude_api_key("test-key".to_string());
        
        let provider = router.get_provider().await;
        assert!(provider.is_some());
        assert_eq!(provider.unwrap().name(), "claude");
    }
    
    #[tokio::test]
    async fn test_preferred_provider() {
        let router = AIRouter::new();
        
        router.set_preferred_provider("ollama").await;
        assert_eq!(router.get_preferred_provider().await, "ollama");
        
        router.set_preferred_provider("claude").await;
        assert_eq!(router.get_preferred_provider().await, "claude");
    }
    
    #[tokio::test]
    async fn test_status() {
        let mut router = AIRouter::new();
        router.set_claude_api_key("test-key".to_string());
        
        let status = router.get_status().await;
        assert!(status.claude_configured);
        assert!(status.claude_available);
    }
}
