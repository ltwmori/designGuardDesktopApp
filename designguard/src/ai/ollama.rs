//! Ollama Client for Local AI
//!
//! Provides integration with Ollama for offline, local AI analysis.

use async_trait::async_trait;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::time::Duration;

use crate::ai::claude::{AIAnalysis, ComponentRecommendation};
use crate::ai::provider::{AIProvider, ModelInfo, SchematicContext};
use crate::ai::AIError;

const DEFAULT_OLLAMA_URL: &str = "http://localhost:11434";
const DEFAULT_MODEL: &str = "llama3.1:8b";
const REQUEST_TIMEOUT_SECS: u64 = 120;

/// Client for interacting with Ollama
pub struct OllamaClient {
    client: Client,
    base_url: String,
    model: String,
}

#[derive(Debug, Serialize)]
struct OllamaRequest {
    model: String,
    prompt: String,
    stream: bool,
    options: OllamaOptions,
}

#[derive(Debug, Serialize)]
struct OllamaOptions {
    temperature: f32,
    num_predict: i32,    // max tokens
    top_p: f32,
}

#[derive(Debug, Deserialize)]
struct OllamaResponse {
    #[allow(dead_code)]
    model: String,
    response: String,
    #[allow(dead_code)]
    done: bool,
    #[allow(dead_code)]
    total_duration: Option<u64>,
    #[allow(dead_code)]
    eval_count: Option<u32>,
}

#[derive(Debug, Deserialize)]
struct OllamaModelList {
    models: Vec<OllamaModel>,
}

#[derive(Debug, Deserialize)]
struct OllamaModel {
    name: String,
    #[allow(dead_code)]
    size: Option<u64>,
    #[allow(dead_code)]
    digest: Option<String>,
    #[allow(dead_code)]
    modified_at: Option<String>,
}

impl OllamaClient {
    /// Create a new Ollama client
    pub fn new(base_url: Option<String>, model: Option<String>) -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(REQUEST_TIMEOUT_SECS))
            .build()
            .unwrap_or_default();
        
        Self {
            client,
            base_url: base_url.unwrap_or_else(|| DEFAULT_OLLAMA_URL.to_string()),
            model: model.unwrap_or_else(|| DEFAULT_MODEL.to_string()),
        }
    }
    
    /// Set the model to use
    pub fn with_model(mut self, model: String) -> Self {
        self.model = model;
        self
    }
    
    /// Set the base URL
    pub fn with_url(mut self, url: String) -> Self {
        self.base_url = url;
        self
    }
    
    /// Check if Ollama is running and the model is available
    pub async fn health_check(&self) -> Result<bool, AIError> {
        let url = format!("{}/api/tags", self.base_url);
        
        match self.client.get(&url).send().await {
            Ok(response) => {
                if response.status().is_success() {
                    let models: OllamaModelList = response.json().await
                        .map_err(|e| AIError::ParseError(e.to_string()))?;
                    Ok(models.models.iter().any(|m| m.name.starts_with(&self.model) || self.model.starts_with(&m.name)))
                } else {
                    Ok(false)
                }
            }
            Err(_) => Ok(false), // Ollama not running
        }
    }
    
    /// List available models
    pub async fn list_models(&self) -> Result<Vec<String>, AIError> {
        let url = format!("{}/api/tags", self.base_url);
        
        let response = self.client.get(&url).send().await
            .map_err(AIError::RequestFailed)?;
        
        if !response.status().is_success() {
            return Err(AIError::ApiError {
                status: response.status().as_u16(),
                message: "Failed to list models".to_string(),
            });
        }
        
        let models: OllamaModelList = response.json().await
            .map_err(|e| AIError::ParseError(e.to_string()))?;
        
        Ok(models.models.into_iter().map(|m| m.name).collect())
    }
    
    /// Generate a completion
    pub async fn generate(&self, prompt: &str) -> Result<String, AIError> {
        let url = format!("{}/api/generate", self.base_url);
        
        let request = OllamaRequest {
            model: self.model.clone(),
            prompt: prompt.to_string(),
            stream: false,
            options: OllamaOptions {
                temperature: 0.3,  // Lower for more consistent technical output
                num_predict: 2000,
                top_p: 0.9,
            },
        };
        
        tracing::debug!("Sending request to Ollama: {}", self.model);
        
        let response = self.client
            .post(&url)
            .json(&request)
            .send()
            .await
            .map_err(AIError::RequestFailed)?;
        
        if !response.status().is_success() {
            let status = response.status().as_u16();
            let message = response.text().await.unwrap_or_default();
            return Err(AIError::ApiError { status, message });
        }
        
        let ollama_response: OllamaResponse = response.json().await
            .map_err(|e| AIError::ParseError(e.to_string()))?;
        
        Ok(ollama_response.response)
    }
    
    /// Build analysis prompt for Ollama
    fn build_analysis_prompt(&self, context: &SchematicContext) -> String {
        let issues_text = if context.detected_issues.is_empty() {
            "None detected yet.".to_string()
        } else {
            context.detected_issues
                .iter()
                .map(|i| format!("- {}", i.message))
                .collect::<Vec<_>>()
                .join("\n")
        };
        
        let components_text = context.component_details
            .iter()
            .take(20) // Limit to avoid token overflow
            .map(|c| format!("- {} ({}): {}", c.reference, c.value, c.lib_id))
            .collect::<Vec<_>>()
            .join("\n");
        
        format!(r#"You are a PCB design expert reviewing an electronic schematic.

SCHEMATIC SUMMARY:
- Total Components: {}
- Power rails: {}
- Signal nets: {} (showing first 10)

COMPONENTS:
{}

ALREADY DETECTED ISSUES:
{}

TASK: Analyze this design and identify any additional issues not in the list above.
Focus on:
1. Missing decoupling capacitors near ICs
2. Missing pull-up/pull-down resistors where needed
3. Incorrect component values for the application
4. Potential signal integrity issues
5. Power supply design problems

Format your response EXACTLY as follows:

SUMMARY: [one sentence description of what this circuit does]

CIRCUIT_DESCRIPTION: [2-3 sentences describing the circuit topology and main functional blocks]

ADDITIONAL ISSUES:
- [issue 1 - be specific about which component]
- [issue 2]
- [issue 3]

RECOMMENDATIONS:
- [improvement suggestion 1]
- [improvement suggestion 2]

COMPONENT_NOTES:
- [component reference]: [suggestion or concern]
"#,
            context.component_count,
            if context.power_rails.is_empty() { "Unknown".to_string() } else { context.power_rails.join(", ") },
            if context.signal_nets.is_empty() { "Unknown".to_string() } else { context.signal_nets.iter().take(10).cloned().collect::<Vec<_>>().join(", ") },
            if components_text.is_empty() { "No components found".to_string() } else { components_text },
            issues_text
        )
    }
    
    /// Build question prompt for Ollama
    fn build_question_prompt(&self, context: &SchematicContext, question: &str) -> String {
        let components_text = context.component_details
            .iter()
            .take(15)
            .map(|c| format!("{} ({})", c.reference, c.value))
            .collect::<Vec<_>>()
            .join(", ");
        
        format!(r#"You are a PCB design expert. Answer the following question about this schematic.

SCHEMATIC:
- {} components total
- Power rails: {}
- Key components: {}

QUESTION: {}

Provide a clear, technical answer. Be specific and reference component designators when relevant.
If you're unsure about something, say so rather than guessing.
"#,
            context.component_count,
            if context.power_rails.is_empty() { "Unknown".to_string() } else { context.power_rails.join(", ") },
            if components_text.is_empty() { "None listed".to_string() } else { components_text },
            question
        )
    }
    
    /// Parse the analysis response from Ollama
    fn parse_analysis_response(&self, response: &str) -> Result<AIAnalysis, AIError> {
        let mut analysis = AIAnalysis {
            summary: String::new(),
            circuit_description: String::new(),
            potential_issues: Vec::new(),
            improvement_suggestions: Vec::new(),
            component_recommendations: Vec::new(),
        };
        
        let mut current_section = "";
        
        for line in response.lines() {
            let line = line.trim();
            
            if line.starts_with("SUMMARY:") {
                analysis.summary = line.trim_start_matches("SUMMARY:").trim().to_string();
                current_section = "";
            } else if line.starts_with("CIRCUIT_DESCRIPTION:") {
                analysis.circuit_description = line.trim_start_matches("CIRCUIT_DESCRIPTION:").trim().to_string();
                current_section = "description";
            } else if line == "ADDITIONAL ISSUES:" || line.starts_with("ADDITIONAL ISSUES") {
                current_section = "issues";
            } else if line == "RECOMMENDATIONS:" || line.starts_with("RECOMMENDATIONS") {
                current_section = "recommendations";
            } else if line == "COMPONENT_NOTES:" || line.starts_with("COMPONENT_NOTES") {
                current_section = "components";
            } else if line.starts_with("- ") || line.starts_with("• ") || line.starts_with("* ") {
                let content = line
                    .trim_start_matches("- ")
                    .trim_start_matches("• ")
                    .trim_start_matches("* ")
                    .to_string();
                
                if content.is_empty() {
                    continue;
                }
                
                match current_section {
                    "issues" => analysis.potential_issues.push(content),
                    "recommendations" => analysis.improvement_suggestions.push(content),
                    "components" => {
                        // Try to parse component recommendation
                        if let Some((comp, suggestion)) = content.split_once(':') {
                            analysis.component_recommendations.push(ComponentRecommendation {
                                component: comp.trim().to_string(),
                                current_value: String::new(),
                                suggested_value: None,
                                reason: suggestion.trim().to_string(),
                            });
                        }
                    }
                    "description" => {
                        // Continuation of description
                        if !analysis.circuit_description.is_empty() {
                            analysis.circuit_description.push(' ');
                        }
                        analysis.circuit_description.push_str(&content);
                    }
                    _ => {}
                }
            } else if !line.is_empty() && current_section == "description" {
                // Continuation of description without bullet
                if !analysis.circuit_description.is_empty() {
                    analysis.circuit_description.push(' ');
                }
                analysis.circuit_description.push_str(line);
            }
        }
        
        // If we didn't parse anything useful, use the raw response
        if analysis.summary.is_empty() && analysis.potential_issues.is_empty() {
            analysis.summary = "Analysis completed".to_string();
            analysis.circuit_description = response.chars().take(500).collect();
        }
        
        Ok(analysis)
    }
    
    /// Get the current model
    pub fn model(&self) -> &str {
        &self.model
    }
    
    /// Get the base URL
    pub fn base_url(&self) -> &str {
        &self.base_url
    }
}

impl Default for OllamaClient {
    fn default() -> Self {
        Self::new(None, None)
    }
}

#[async_trait]
impl AIProvider for OllamaClient {
    fn name(&self) -> &str {
        "ollama"
    }
    
    async fn is_available(&self) -> bool {
        self.health_check().await.unwrap_or(false)
    }
    
    async fn analyze_schematic(
        &self,
        context: &SchematicContext,
    ) -> Result<AIAnalysis, AIError> {
        let prompt = self.build_analysis_prompt(context);
        let response = self.generate(&prompt).await?;
        self.parse_analysis_response(&response)
    }
    
    async fn ask_question(
        &self,
        context: &SchematicContext,
        question: &str,
    ) -> Result<String, AIError> {
        let prompt = self.build_question_prompt(context, question);
        self.generate(&prompt).await
    }
    
    fn model_info(&self) -> ModelInfo {
        // Estimate context window based on model name
        let context_window = if self.model.contains("70b") {
            8192
        } else if self.model.contains("mixtral") {
            32768
        } else {
            4096
        };
        
        ModelInfo {
            provider: "ollama".to_string(),
            model_name: self.model.clone(),
            is_local: true,
            context_window,
            supports_json: false, // Most Ollama models don't reliably output JSON
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_parse_analysis_response() {
        let client = OllamaClient::new(None, None);
        
        let response = r#"
SUMMARY: This is a 555 timer circuit in astable mode.

CIRCUIT_DESCRIPTION: The circuit uses a NE555 timer IC configured as an astable multivibrator to generate a square wave output.

ADDITIONAL ISSUES:
- U1 (NE555) is missing a decoupling capacitor on pin 8
- Control voltage pin 5 should have a 10nF capacitor to ground

RECOMMENDATIONS:
- Add 100nF ceramic capacitor close to VCC pin
- Consider using a CMOS 555 for lower power consumption

COMPONENT_NOTES:
- R1: Value seems appropriate for the frequency
- C1: Consider using a film capacitor for better stability
"#;
        
        let analysis = client.parse_analysis_response(response).unwrap();
        
        assert!(!analysis.summary.is_empty());
        assert!(analysis.summary.contains("555"));
        assert!(!analysis.potential_issues.is_empty());
        assert!(!analysis.improvement_suggestions.is_empty());
    }
    
    #[test]
    fn test_build_analysis_prompt() {
        let client = OllamaClient::new(None, None);
        
        let context = SchematicContext {
            components_summary: "Test circuit".to_string(),
            power_rails: vec!["VCC".to_string(), "GND".to_string()],
            signal_nets: vec!["SDA".to_string(), "SCL".to_string()],
            detected_issues: vec![],
            component_count: 10,
            component_details: vec![],
        };
        
        let prompt = client.build_analysis_prompt(&context);
        
        assert!(prompt.contains("10"));
        assert!(prompt.contains("VCC"));
        assert!(prompt.contains("SUMMARY"));
    }
}
