use async_trait::async_trait;
use crate::ai::prompts;
use crate::ai::provider::{AIProvider, ModelInfo, SchematicContext};
use crate::analyzer::rules::Issue;
use crate::parser::schema::Schematic;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::time::Duration;
use thiserror::Error;
use tokio::time::sleep;

const CLAUDE_API_URL: &str = "https://api.anthropic.com/v1/messages";
const CLAUDE_API_VERSION: &str = "2023-06-01";
const DEFAULT_MODEL: &str = "claude-sonnet-4-20250514";
const MAX_TOKENS: u32 = 4096;
const MAX_RETRIES: u32 = 3;
const INITIAL_RETRY_DELAY_MS: u64 = 1000;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AIAnalysis {
    #[serde(default)]
    pub summary: String,
    #[serde(default)]
    pub circuit_description: String,
    #[serde(default)]
    pub potential_issues: Vec<String>,
    #[serde(default)]
    pub improvement_suggestions: Vec<String>,
    #[serde(default)]
    pub component_recommendations: Vec<ComponentRecommendation>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComponentRecommendation {
    pub component: String,
    pub current_value: String,
    pub suggested_value: Option<String>,
    pub reason: String,
}

#[derive(Debug, Error)]
pub enum AIError {
    #[error("API request failed: {0}")]
    RequestFailed(#[from] reqwest::Error),
    #[error("API error: {status} - {message}")]
    ApiError { status: u16, message: String },
    #[error("Failed to parse response: {0}")]
    ParseError(String),
    #[error("Rate limited. Retry after {retry_after} seconds")]
    RateLimited { retry_after: u64 },
    #[error("Missing API key or no provider available")]
    MissingApiKey,
    #[error("Invalid response format: {0}")]
    InvalidResponse(String),
}

pub struct ClaudeClient {
    client: Client,
    api_key: String,
    model: String,
}

impl ClaudeClient {
    pub fn new(api_key: String) -> Self {
        Self {
            client: Client::new(),
            api_key,
            model: DEFAULT_MODEL.to_string(),
        }
    }

    pub fn with_model(mut self, model: String) -> Self {
        self.model = model;
        self
    }

    pub async fn analyze_schematic(
        &self,
        schematic: &Schematic,
        existing_issues: &[Issue],
    ) -> Result<AIAnalysis, AIError> {
        let prompt = prompts::build_analysis_prompt(schematic, existing_issues);
        let response_text = self.send_request(&prompt).await?;
        self.parse_analysis_response(&response_text)
    }

    pub async fn ask_question(
        &self,
        schematic: &Schematic,
        question: &str,
    ) -> Result<String, AIError> {
        let prompt = prompts::build_question_prompt(schematic, question);
        let response_text = self.send_request(&prompt).await?;
        Ok(response_text)
    }

    async fn send_request(&self, prompt: &str) -> Result<String, AIError> {
        if self.api_key.is_empty() {
            return Err(AIError::MissingApiKey);
        }

        let request_body = ClaudeRequest {
            model: self.model.clone(),
            max_tokens: MAX_TOKENS,
            messages: vec![Message {
                role: "user".to_string(),
                content: prompt.to_string(),
            }],
        };

        let mut retry_count = 0;
        let mut delay_ms = INITIAL_RETRY_DELAY_MS;

        loop {
            let response = self
                .client
                .post(CLAUDE_API_URL)
                .header("x-api-key", &self.api_key)
                .header("anthropic-version", CLAUDE_API_VERSION)
                .header("content-type", "application/json")
                .json(&request_body)
                .send()
                .await;

            match response {
                Ok(resp) => {
                    let status = resp.status();
                    
                    if status.is_success() {
                        let claude_resp: ClaudeResponse = resp
                            .json()
                            .await
                            .map_err(|e| AIError::ParseError(format!("Failed to parse JSON: {}", e)))?;

                        // Extract text from content array
                        if let Some(content) = claude_resp.content.first() {
                            return Ok(content.text.clone());
                        } else {
                            return Err(AIError::InvalidResponse(
                                "Empty content array in response".to_string(),
                            ));
                        }
                    } else if status.as_u16() == 429 {
                        // Rate limited
                        let retry_after = resp
                            .headers()
                            .get("retry-after")
                            .and_then(|h| h.to_str().ok())
                            .and_then(|s| s.parse::<u64>().ok())
                            .unwrap_or(delay_ms / 1000);

                        if retry_count < MAX_RETRIES {
                            retry_count += 1;
                            tracing::warn!(
                                "Rate limited. Retrying after {} seconds (attempt {}/{})",
                                retry_after,
                                retry_count,
                                MAX_RETRIES
                            );
                            sleep(Duration::from_secs(retry_after)).await;
                            delay_ms *= 2; // Exponential backoff
                            continue;
                        } else {
                            return Err(AIError::RateLimited { retry_after });
                        }
                    } else {
                        // Other error
                        let error_text = resp
                            .text()
                            .await
                            .unwrap_or_else(|_| "Unknown error".to_string());
                        
                        return Err(AIError::ApiError {
                            status: status.as_u16(),
                            message: error_text,
                        });
                    }
                }
                Err(e) => {
                    if retry_count < MAX_RETRIES {
                        retry_count += 1;
                        tracing::warn!(
                            "Request failed: {}. Retrying in {}ms (attempt {}/{})",
                            e,
                            delay_ms,
                            retry_count,
                            MAX_RETRIES
                        );
                        sleep(Duration::from_millis(delay_ms)).await;
                        delay_ms *= 2; // Exponential backoff
                        continue;
                    } else {
                        return Err(AIError::RequestFailed(e));
                    }
                }
            }
        }
    }

    fn parse_analysis_response(&self, response_text: &str) -> Result<AIAnalysis, AIError> {
        // Try to extract JSON from the response
        // Claude might wrap JSON in markdown code blocks or add extra text
        let json_text = extract_json_from_text(response_text);

        // Parse JSON
        let analysis: AIAnalysis = serde_json::from_str(&json_text)
            .map_err(|e| AIError::ParseError(format!("Failed to parse AI response: {}", e)))?;

        Ok(analysis)
    }
    
    /// Build context-aware prompt for SchematicContext
    fn build_context_prompt(&self, context: &SchematicContext) -> String {
        let issues_text = if context.detected_issues.is_empty() {
            "None detected yet.".to_string()
        } else {
            context.detected_issues
                .iter()
                .map(|i| format!("- {}", i.message))
                .collect::<Vec<_>>()
                .join("\n")
        };
        
        format!(r#"You are an expert PCB design reviewer analyzing a KiCAD schematic.

SCHEMATIC SUMMARY:
{}

COMPONENTS: {} total
POWER RAILS: {}
SIGNAL NETS: {}

DETECTED ISSUES:
{}

Analyze this schematic and provide your assessment in the following JSON format:
{{
  "summary": "Brief one-sentence summary of the circuit",
  "circuit_description": "2-3 sentence description of the circuit topology",
  "potential_issues": ["issue 1", "issue 2"],
  "improvement_suggestions": ["suggestion 1", "suggestion 2"],
  "component_recommendations": [
    {{"component": "U1", "current_value": "STM32F4", "suggested_value": null, "reason": "explanation"}}
  ]
}}

Focus on:
1. Missing decoupling capacitors
2. Missing pull-up/pull-down resistors
3. Power supply issues
4. Signal integrity concerns
5. Component value recommendations

Return ONLY valid JSON, no additional text."#,
            context.components_summary,
            context.component_count,
            if context.power_rails.is_empty() { "Unknown".to_string() } else { context.power_rails.join(", ") },
            if context.signal_nets.is_empty() { "Unknown".to_string() } else { context.signal_nets.iter().take(10).cloned().collect::<Vec<_>>().join(", ") },
            issues_text
        )
    }
    
    /// Build question prompt for SchematicContext
    fn build_context_question_prompt(&self, context: &SchematicContext, question: &str) -> String {
        format!(r#"You are an expert PCB design reviewer.

SCHEMATIC CONTEXT:
- {} components
- Power rails: {}
- Signal nets: {}

QUESTION: {}

Provide a clear, technical answer. Reference specific components when relevant."#,
            context.component_count,
            if context.power_rails.is_empty() { "Unknown".to_string() } else { context.power_rails.join(", ") },
            if context.signal_nets.is_empty() { "Unknown".to_string() } else { context.signal_nets.iter().take(10).cloned().collect::<Vec<_>>().join(", ") },
            question
        )
    }
}

#[async_trait]
impl AIProvider for ClaudeClient {
    fn name(&self) -> &str {
        "claude"
    }
    
    async fn is_available(&self) -> bool {
        !self.api_key.is_empty()
    }
    
    async fn analyze_schematic(
        &self,
        context: &SchematicContext,
    ) -> Result<AIAnalysis, AIError> {
        let prompt = self.build_context_prompt(context);
        let response_text = self.send_request(&prompt).await?;
        self.parse_analysis_response(&response_text)
    }
    
    async fn ask_question(
        &self,
        context: &SchematicContext,
        question: &str,
    ) -> Result<String, AIError> {
        let prompt = self.build_context_question_prompt(context, question);
        self.send_request(&prompt).await
    }
    
    fn model_info(&self) -> ModelInfo {
        ModelInfo {
            provider: "claude".to_string(),
            model_name: self.model.clone(),
            is_local: false,
            context_window: 200000, // Claude has large context
            supports_json: true,
        }
    }
}

fn extract_json_from_text(text: &str) -> String {
    // Try to find JSON object in the text
    // Look for { ... } pattern
    let text = text.trim();
    
    // Check if wrapped in markdown code block
    if let Some(start) = text.find("```json") {
        if let Some(end) = text.rfind("```") {
            return text[start + 7..end].trim().to_string();
        }
    }
    
    // Check if wrapped in regular code block
    if let Some(start) = text.find("```") {
        if let Some(end) = text.rfind("```") {
            let content = &text[start + 3..end];
            // Check if it looks like JSON
            if content.trim().starts_with('{') {
                return content.trim().to_string();
            }
        }
    }
    
    // Try to find JSON object boundaries
    if let Some(start) = text.find('{') {
        if let Some(end) = text.rfind('}') {
            return text[start..=end].to_string();
        }
    }
    
    // If no JSON found, return original text (will fail parsing but that's ok)
    text.to_string()
}

#[derive(Debug, Serialize)]
struct ClaudeRequest {
    model: String,
    max_tokens: u32,
    messages: Vec<Message>,
}

#[derive(Debug, Serialize)]
struct Message {
    role: String,
    content: String,
}

#[derive(Debug, Deserialize)]
struct ClaudeResponse {
    content: Vec<Content>,
}

#[derive(Debug, Deserialize)]
struct Content {
    #[serde(rename = "type")]
    content_type: Option<String>,
    text: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_json_from_text() {
        let text = r#"Here's the analysis:
```json
{"summary": "Test", "circuit_description": "Test circuit"}
```
"#;
        let json = extract_json_from_text(text);
        assert!(json.contains("summary"));
    }

    #[test]
    fn test_extract_json_direct() {
        let text = r#"{"summary": "Test", "circuit_description": "Test circuit"}"#;
        let json = extract_json_from_text(text);
        assert_eq!(json, text);
    }
}
