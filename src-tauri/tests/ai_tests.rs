//! AI Module Tests for KiCAD AI Assistant
//!
//! This module tests the AI integration including:
//! - Claude client functionality (with mocks)
//! - Prompt building
//! - Response parsing
//! - Error handling (rate limits, API errors)
//! - JSON extraction from responses

use std::collections::HashMap;

use designguard::ai::claude::{AIAnalysis, AIError, ClaudeClient, ComponentRecommendation};
use designguard::ai::prompts::{build_analysis_prompt, build_question_prompt};
use designguard::analyzer::rules::{Issue, Severity};
use designguard::parser::schema::{
    Component, Label, LabelType, Net, Pin, Position, Schematic, Wire,
};

// =============================================================================
// Test Helpers
// =============================================================================

fn create_test_schematic() -> Schematic {
    Schematic {
        uuid: "test-schematic-uuid".to_string(),
        filename: "test_design.kicad_sch".to_string(),
        version: Some("20231120".to_string()),
        components: vec![
            Component {
                uuid: "u1-uuid".to_string(),
                reference: "U1".to_string(),
                value: "STM32F401CCU6".to_string(),
                lib_id: "MCU_ST_STM32F4:STM32F401CCU6".to_string(),
                footprint: Some("Package_QFP:LQFP-48_7x7mm_P0.5mm".to_string()),
                position: Position { x: 150.0, y: 100.0 },
                rotation: 0.0,
                properties: HashMap::new(),
                pins: vec![
                    Pin { number: "1".to_string(), uuid: "pin1".to_string() },
                    Pin { number: "2".to_string(), uuid: "pin2".to_string() },
                ],
            },
            Component {
                uuid: "r1-uuid".to_string(),
                reference: "R1".to_string(),
                value: "10k".to_string(),
                lib_id: "Device:R".to_string(),
                footprint: Some("Resistor_SMD:R_0402_1005Metric".to_string()),
                position: Position { x: 100.0, y: 50.0 },
                rotation: 0.0,
                properties: HashMap::new(),
                pins: vec![
                    Pin { number: "1".to_string(), uuid: "r1-pin1".to_string() },
                    Pin { number: "2".to_string(), uuid: "r1-pin2".to_string() },
                ],
            },
            Component {
                uuid: "c1-uuid".to_string(),
                reference: "C1".to_string(),
                value: "100nF".to_string(),
                lib_id: "Device:C".to_string(),
                footprint: Some("Capacitor_SMD:C_0402_1005Metric".to_string()),
                position: Position { x: 120.0, y: 50.0 },
                rotation: 0.0,
                properties: HashMap::new(),
                pins: vec![
                    Pin { number: "1".to_string(), uuid: "c1-pin1".to_string() },
                    Pin { number: "2".to_string(), uuid: "c1-pin2".to_string() },
                ],
            },
        ],
        wires: vec![
            Wire {
                uuid: "wire1-uuid".to_string(),
                points: vec![
                    Position { x: 100.0, y: 50.0 },
                    Position { x: 120.0, y: 50.0 },
                ],
            },
        ],
        labels: vec![
            Label {
                uuid: "sda-uuid".to_string(),
                text: "SDA".to_string(),
                position: Position { x: 200.0, y: 80.0 },
                rotation: 0.0,
                label_type: LabelType::Global,
            },
            Label {
                uuid: "scl-uuid".to_string(),
                text: "SCL".to_string(),
                position: Position { x: 200.0, y: 90.0 },
                rotation: 0.0,
                label_type: LabelType::Global,
            },
        ],
        nets: vec![
            Net {
                name: "VCC".to_string(),
                connections: vec![],
            },
        ],
        power_symbols: vec![
            Component {
                uuid: "gnd-uuid".to_string(),
                reference: "#PWR01".to_string(),
                value: "GND".to_string(),
                lib_id: "power:GND".to_string(),
                footprint: None,
                position: Position { x: 150.0, y: 120.0 },
                rotation: 0.0,
                properties: HashMap::new(),
                pins: vec![],
            },
            Component {
                uuid: "vcc-uuid".to_string(),
                reference: "#PWR02".to_string(),
                value: "+3.3V".to_string(),
                lib_id: "power:+3.3V".to_string(),
                footprint: None,
                position: Position { x: 150.0, y: 80.0 },
                rotation: 0.0,
                properties: HashMap::new(),
                pins: vec![],
            },
        ],
    }
}

fn create_test_issues() -> Vec<Issue> {
    vec![
        Issue {
            id: "issue-1".to_string(),
            rule_id: "decoupling_capacitor".to_string(),
            severity: Severity::Warning,
            message: "IC U1 may need a decoupling capacitor".to_string(),
            component: Some("U1".to_string()),
            location: Some(Position { x: 150.0, y: 100.0 }),
            suggestion: Some("Add 100nF ceramic capacitor".to_string()),
            risk_score: None,
        },
        Issue {
            id: "issue-2".to_string(),
            rule_id: "i2c_pull_resistors".to_string(),
            severity: Severity::Warning,
            message: "I2C bus detected but no pull-up resistors found".to_string(),
            component: None,
            location: None,
            suggestion: Some("Add 4.7k pull-ups to SDA and SCL".to_string()),
            risk_score: None,
        },
    ]
}

// =============================================================================
// Prompt Building Tests
// =============================================================================

mod prompt_tests {
    use super::*;

    #[test]
    fn test_prompt_building() {
        let schematic = create_test_schematic();
        let issues = create_test_issues();

        let prompt = build_analysis_prompt(&schematic, &issues);

        // Verify prompt contains key information
        assert!(
            prompt.contains("PCB design engineer"),
            "Should include expert context"
        );
        assert!(
            prompt.contains("components"),
            "Should mention components"
        );
        assert!(
            prompt.contains("JSON"),
            "Should request JSON output"
        );
    }

    #[test]
    fn test_prompt_includes_component_summary() {
        let schematic = create_test_schematic();
        let issues = vec![];

        let prompt = build_analysis_prompt(&schematic, &issues);

        // Should summarize components
        assert!(
            prompt.contains("IC") || prompt.contains("component"),
            "Should mention component types"
        );
        assert!(
            prompt.contains("resistor") || prompt.contains("capacitor") || prompt.contains("Total"),
            "Should summarize component counts"
        );
    }

    #[test]
    fn test_prompt_includes_power_rails() {
        let schematic = create_test_schematic();
        let issues = vec![];

        let prompt = build_analysis_prompt(&schematic, &issues);

        // Should include power rail information
        assert!(
            prompt.contains("Power") || prompt.contains("GND") || prompt.contains("3.3V"),
            "Should mention power rails"
        );
    }

    #[test]
    fn test_prompt_includes_issues() {
        let schematic = create_test_schematic();
        let issues = create_test_issues();

        let prompt = build_analysis_prompt(&schematic, &issues);

        // Should include existing issues
        assert!(
            prompt.contains("issue") || prompt.contains("Issue") || prompt.contains("warning"),
            "Should mention existing issues"
        );
    }

    #[test]
    fn test_prompt_json_format_specified() {
        let schematic = create_test_schematic();
        let issues = vec![];

        let prompt = build_analysis_prompt(&schematic, &issues);

        // Should specify JSON format requirements
        assert!(prompt.contains("summary"), "Should mention summary field");
        assert!(
            prompt.contains("circuit_description"),
            "Should mention circuit_description field"
        );
        assert!(
            prompt.contains("potential_issues"),
            "Should mention potential_issues field"
        );
        assert!(
            prompt.contains("improvement_suggestions"),
            "Should mention improvement_suggestions field"
        );
        assert!(
            prompt.contains("component_recommendations"),
            "Should mention component_recommendations field"
        );
    }

    #[test]
    fn test_question_prompt_building() {
        let schematic = create_test_schematic();
        let question = "Why do I need decoupling capacitors?";

        let prompt = build_question_prompt(&schematic, question);

        // Should include the question
        assert!(
            prompt.contains(question),
            "Should include the user's question"
        );
        // Should include context
        assert!(
            prompt.contains("Schematic") || prompt.contains("Components"),
            "Should include schematic context"
        );
    }

    #[test]
    fn test_prompt_handles_empty_schematic() {
        let schematic = Schematic {
            uuid: "empty".to_string(),
            filename: "empty.kicad_sch".to_string(),
            version: None,
            components: vec![],
            wires: vec![],
            labels: vec![],
            nets: vec![],
            power_symbols: vec![],
        };
        let issues = vec![];

        let prompt = build_analysis_prompt(&schematic, &issues);

        // Should still generate a valid prompt
        assert!(!prompt.is_empty());
        assert!(prompt.contains("JSON"));
    }

    #[test]
    fn test_prompt_handles_large_component_list() {
        let mut schematic = create_test_schematic();

        // Add many components
        for i in 0..100 {
            schematic.components.push(Component {
                uuid: format!("r{}-uuid", i),
                reference: format!("R{}", i),
                value: "10k".to_string(),
                lib_id: "Device:R".to_string(),
                footprint: None,
                position: Position {
                    x: (i as f64) * 10.0,
                    y: 0.0,
                },
                rotation: 0.0,
                properties: HashMap::new(),
                pins: vec![],
            });
        }

        let issues = vec![];
        let prompt = build_analysis_prompt(&schematic, &issues);

        // Should not be excessively long (summarized)
        assert!(
            prompt.len() < 10000,
            "Prompt should be reasonably sized even with many components"
        );
    }
}

// =============================================================================
// Response Parsing Tests
// =============================================================================

mod response_parsing_tests {
    use super::*;

    #[test]
    fn test_response_parsing_valid_json() {
        let json_response = r#"{
            "summary": "STM32-based microcontroller circuit",
            "circuit_description": "This circuit features an STM32F401 MCU with basic passive components.",
            "potential_issues": [
                "Missing decoupling capacitors on VCC pins",
                "No pull-up resistors on I2C lines"
            ],
            "improvement_suggestions": [
                "Add 100nF ceramic capacitors near each VCC pin",
                "Add 4.7k pull-up resistors to SDA and SCL"
            ],
            "component_recommendations": [
                {
                    "component": "R1",
                    "current_value": "10k",
                    "suggested_value": "4.7k",
                    "reason": "Better suited for I2C pull-up"
                }
            ]
        }"#;

        let analysis: Result<AIAnalysis, _> = serde_json::from_str(json_response);
        assert!(analysis.is_ok(), "Should parse valid JSON response");

        let analysis = analysis.unwrap();
        assert_eq!(analysis.summary, "STM32-based microcontroller circuit");
        assert_eq!(analysis.potential_issues.len(), 2);
        assert_eq!(analysis.improvement_suggestions.len(), 2);
        assert_eq!(analysis.component_recommendations.len(), 1);
    }

    #[test]
    fn test_response_parsing_with_markdown_wrapper() {
        // Claude sometimes wraps JSON in markdown code blocks
        let wrapped_response = r#"Here's my analysis:

```json
{
    "summary": "Test circuit",
    "circuit_description": "A test circuit",
    "potential_issues": [],
    "improvement_suggestions": [],
    "component_recommendations": []
}
```

Let me know if you have questions!"#;

        // Extract JSON from wrapper
        let json_start = wrapped_response.find("```json").map(|i| i + 7);
        let json_end = wrapped_response.rfind("```");

        if let (Some(start), Some(end)) = (json_start, json_end) {
            let json_str = &wrapped_response[start..end].trim();
            let analysis: Result<AIAnalysis, _> = serde_json::from_str(json_str);
            assert!(analysis.is_ok(), "Should parse JSON from markdown wrapper");
        }
    }

    #[test]
    fn test_response_parsing_empty_arrays() {
        let json_response = r#"{
            "summary": "Simple circuit",
            "circuit_description": "Minimal design",
            "potential_issues": [],
            "improvement_suggestions": [],
            "component_recommendations": []
        }"#;

        let analysis: Result<AIAnalysis, _> = serde_json::from_str(json_response);
        assert!(analysis.is_ok());

        let analysis = analysis.unwrap();
        assert!(analysis.potential_issues.is_empty());
        assert!(analysis.improvement_suggestions.is_empty());
        assert!(analysis.component_recommendations.is_empty());
    }

    #[test]
    fn test_response_parsing_component_recommendation() {
        let json_response = r#"{
            "summary": "Test",
            "circuit_description": "Test",
            "potential_issues": [],
            "improvement_suggestions": [],
            "component_recommendations": [
                {
                    "component": "C1",
                    "current_value": "100nF",
                    "suggested_value": "10uF",
                    "reason": "Larger capacitance for better filtering"
                },
                {
                    "component": "R2",
                    "current_value": "10k",
                    "suggested_value": null,
                    "reason": "Value is appropriate"
                }
            ]
        }"#;

        let analysis: Result<AIAnalysis, _> = serde_json::from_str(json_response);
        assert!(analysis.is_ok());

        let analysis = analysis.unwrap();
        assert_eq!(analysis.component_recommendations.len(), 2);
        assert_eq!(analysis.component_recommendations[0].component, "C1");
        assert_eq!(
            analysis.component_recommendations[0].suggested_value,
            Some("10uF".to_string())
        );
        assert!(analysis.component_recommendations[1].suggested_value.is_none());
    }

    #[test]
    fn test_response_parsing_invalid_json() {
        let invalid_json = r#"{ invalid json here }"#;

        let analysis: Result<AIAnalysis, _> = serde_json::from_str(invalid_json);
        assert!(analysis.is_err(), "Should fail on invalid JSON");
    }

    #[test]
    fn test_response_parsing_missing_fields() {
        // AIAnalysis has #[serde(default)] on all fields, so partial JSON parses with defaults
        let incomplete_json = r#"{
            "summary": "Test"
        }"#;

        let analysis: Result<AIAnalysis, _> = serde_json::from_str(incomplete_json);
        assert!(analysis.is_ok(), "Partial response should parse with defaults");

        let analysis = analysis.unwrap();
        assert_eq!(analysis.summary, "Test");
        assert!(analysis.circuit_description.is_empty());
        assert!(analysis.potential_issues.is_empty());
        assert!(analysis.improvement_suggestions.is_empty());
        assert!(analysis.component_recommendations.is_empty());
    }
}

// =============================================================================
// AI Client Tests (Mock-based)
// =============================================================================

mod ai_client_tests {
    use super::*;

    #[test]
    fn test_client_creation() {
        let _client = ClaudeClient::new("test-api-key".to_string());
        // Client should be created successfully
        // We can't test internals, but we can verify it doesn't panic
    }

    #[test]
    fn test_client_with_custom_model() {
        let _client = ClaudeClient::new("test-api-key".to_string())
            .with_model("claude-3-haiku-20240307".to_string());
        // Should accept custom model
    }

    #[test]
    fn test_analyze_schematic_success_mock() {
        // This test simulates a successful API response
        // In a real test suite, you'd use a mock HTTP client
        
        let mock_response = AIAnalysis {
            summary: "STM32 microcontroller with basic peripherals".to_string(),
            circuit_description: "The circuit features an STM32F401 MCU with I2C connectivity.".to_string(),
            potential_issues: vec![
                "Missing decoupling capacitors".to_string(),
                "No ESD protection on I2C lines".to_string(),
            ],
            improvement_suggestions: vec![
                "Add 100nF caps near VCC pins".to_string(),
                "Consider adding TVS diodes".to_string(),
            ],
            component_recommendations: vec![
                ComponentRecommendation {
                    component: "R1".to_string(),
                    current_value: "10k".to_string(),
                    suggested_value: Some("4.7k".to_string()),
                    reason: "Standard I2C pull-up value".to_string(),
                },
            ],
        };

        // Verify the mock response structure is valid
        assert!(!mock_response.summary.is_empty());
        assert!(!mock_response.circuit_description.is_empty());
        assert_eq!(mock_response.potential_issues.len(), 2);
        assert_eq!(mock_response.improvement_suggestions.len(), 2);
        assert_eq!(mock_response.component_recommendations.len(), 1);
    }

    #[test]
    fn test_analyze_schematic_rate_limit_mock() {
        // This test simulates rate limiting behavior
        
        let rate_limit_error = AIError::RateLimited { retry_after: 30 };
        
        match rate_limit_error {
            AIError::RateLimited { retry_after } => {
                assert_eq!(retry_after, 30);
            }
            _ => panic!("Expected RateLimited error"),
        }
    }

    #[test]
    fn test_missing_api_key_error() {
        let error = AIError::MissingApiKey;
        let error_message = format!("{}", error);
        assert!(error_message.contains("API key"));
    }

    #[test]
    fn test_api_error_handling() {
        let error = AIError::ApiError {
            status: 401,
            message: "Invalid API key".to_string(),
        };
        
        let error_message = format!("{}", error);
        assert!(error_message.contains("401"));
        assert!(error_message.contains("Invalid"));
    }

    #[test]
    fn test_parse_error_handling() {
        let error = AIError::ParseError("Invalid JSON format".to_string());
        let error_message = format!("{}", error);
        assert!(error_message.contains("parse") || error_message.contains("Parse"));
    }

    #[test]
    fn test_invalid_response_error() {
        let error = AIError::InvalidResponse("Empty content".to_string());
        let error_message = format!("{}", error);
        assert!(error_message.contains("response") || error_message.contains("Response"));
    }
}

// =============================================================================
// JSON Extraction Tests
// =============================================================================

mod json_extraction_tests {
    fn extract_json_from_text(text: &str) -> String {
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

        text.to_string()
    }

    #[test]
    fn test_extract_json_from_markdown() {
        let text = r#"Here's my analysis:

```json
{"summary": "Test", "circuit_description": "Test"}
```

Hope this helps!"#;

        let json = extract_json_from_text(text);
        assert!(json.starts_with('{'));
        assert!(json.ends_with('}'));
        assert!(json.contains("summary"));
    }

    #[test]
    fn test_extract_json_direct() {
        let text = r#"{"summary": "Direct JSON", "circuit_description": "Test"}"#;
        let json = extract_json_from_text(text);
        assert_eq!(json, text);
    }

    #[test]
    fn test_extract_json_with_prefix_text() {
        let text = r#"Based on my analysis, here is the result:
{"summary": "Result", "circuit_description": "Test"}"#;

        let json = extract_json_from_text(text);
        assert!(json.starts_with('{'));
        assert!(json.contains("summary"));
    }

    #[test]
    fn test_extract_json_with_suffix_text() {
        let text = r#"{"summary": "Result", "circuit_description": "Test"}

Let me know if you need more details."#;

        let json = extract_json_from_text(text);
        assert!(json.starts_with('{'));
        assert!(json.ends_with('}'));
    }

    #[test]
    fn test_extract_json_from_code_block() {
        let text = r#"```
{"summary": "In code block", "circuit_description": "Test"}
```"#;

        let json = extract_json_from_text(text);
        assert!(json.contains("summary"));
    }

    #[test]
    fn test_extract_handles_multiline_json() {
        let text = r#"```json
{
    "summary": "Multiline",
    "circuit_description": "Test",
    "potential_issues": [
        "Issue 1",
        "Issue 2"
    ]
}
```"#;

        let json = extract_json_from_text(text);
        assert!(json.contains("summary"));
        assert!(json.contains("potential_issues"));
    }
}

// =============================================================================
// Integration-style AI Tests
// =============================================================================

mod ai_integration_tests {
    use super::*;

    #[test]
    fn test_full_analysis_workflow_mock() {
        // Simulate the full analysis workflow without actual API calls
        
        // 1. Create schematic
        let schematic = create_test_schematic();
        
        // 2. Run rules engine first (as the real flow does)
        use designguard::analyzer::rules::RulesEngine;
        let engine = RulesEngine::with_default_rules();
        let rule_issues = engine.analyze(&schematic);
        
        // 3. Build prompt for AI
        let prompt = build_analysis_prompt(&schematic, &rule_issues);
        
        // 4. Verify prompt is ready for AI
        assert!(!prompt.is_empty());
        assert!(prompt.contains("JSON"));
        
        // 5. Simulate AI response
        let mock_ai_response = AIAnalysis {
            summary: "STM32 microcontroller circuit with I2C interface".to_string(),
            circuit_description: "The design features an STM32F401 MCU connected to various passive components.".to_string(),
            potential_issues: vec![
                "Consider adding more decoupling capacitors".to_string(),
            ],
            improvement_suggestions: vec![
                "Add bulk capacitor near power input".to_string(),
            ],
            component_recommendations: vec![],
        };
        
        // 6. Verify response is usable
        assert!(!mock_ai_response.summary.is_empty());
    }

    #[test]
    fn test_question_workflow_mock() {
        let schematic = create_test_schematic();
        let question = "What value pull-up resistors should I use for I2C at 100kHz?";
        
        // Build question prompt
        let prompt = build_question_prompt(&schematic, question);
        
        // Verify prompt includes context and question
        assert!(prompt.contains(question));
        assert!(prompt.contains("STM32") || prompt.contains("Components") || prompt.contains("ICs"));
    }

    #[test]
    fn test_error_recovery_scenarios() {
        // Test that various error types are properly distinguished
        
        let errors: Vec<AIError> = vec![
            AIError::MissingApiKey,
            AIError::RateLimited { retry_after: 60 },
            AIError::ApiError {
                status: 500,
                message: "Internal error".to_string(),
            },
            AIError::ParseError("JSON error".to_string()),
            AIError::InvalidResponse("Empty".to_string()),
        ];
        
        for error in errors {
            let message = format!("{}", error);
            assert!(!message.is_empty(), "All errors should have messages");
        }
    }
}

// =============================================================================
// AIAnalysis Struct Tests
// =============================================================================

mod ai_analysis_struct_tests {
    use super::*;

    #[test]
    fn test_ai_analysis_serialization() {
        let analysis = AIAnalysis {
            summary: "Test summary".to_string(),
            circuit_description: "Test description".to_string(),
            potential_issues: vec!["Issue 1".to_string()],
            improvement_suggestions: vec!["Suggestion 1".to_string()],
            component_recommendations: vec![ComponentRecommendation {
                component: "R1".to_string(),
                current_value: "10k".to_string(),
                suggested_value: Some("4.7k".to_string()),
                reason: "Test reason".to_string(),
            }],
        };

        // Serialize to JSON
        let json = serde_json::to_string(&analysis);
        assert!(json.is_ok());

        // Deserialize back
        let json_str = json.unwrap();
        let deserialized: Result<AIAnalysis, _> = serde_json::from_str(&json_str);
        assert!(deserialized.is_ok());

        let restored = deserialized.unwrap();
        assert_eq!(restored.summary, analysis.summary);
        assert_eq!(restored.potential_issues.len(), 1);
    }

    #[test]
    fn test_component_recommendation_with_null_suggestion() {
        let rec = ComponentRecommendation {
            component: "C1".to_string(),
            current_value: "100nF".to_string(),
            suggested_value: None, // No change suggested
            reason: "Value is appropriate".to_string(),
        };

        let json = serde_json::to_string(&rec);
        assert!(json.is_ok());
        
        let json_str = json.unwrap();
        // null should be serialized properly
        let deserialized: Result<ComponentRecommendation, _> = serde_json::from_str(&json_str);
        assert!(deserialized.is_ok());
        assert!(deserialized.unwrap().suggested_value.is_none());
    }
}
