use std::collections::HashMap;

use agent_sdk::{Constraints, ModelConfig, OutputSchema, SkillManifest};
use skill_loader::{validate, AllToolsExist, SkillError, ToolExists};

fn valid_manifest() -> SkillManifest {
    SkillManifest {
        name: "test-skill".to_string(),
        version: "1.0".to_string(),
        description: "A test skill".to_string(),
        model: ModelConfig {
            provider: "anthropic".to_string(),
            name: "claude-3-haiku".to_string(),
            temperature: 0.5,
        },
        preamble: "You are a test assistant.".to_string(),
        tools: vec!["web_search".to_string()],
        constraints: Constraints {
            max_turns: 5,
            confidence_threshold: 0.8,
            escalate_to: None,
            allowed_actions: vec!["search".to_string()],
        },
        output: OutputSchema {
            format: "json".to_string(),
            schema: HashMap::from([("result".to_string(), "string".to_string())]),
        },
    }
}

struct RejectTools {
    rejected: Vec<String>,
}

impl ToolExists for RejectTools {
    fn tool_exists(&self, name: &str) -> bool {
        !self.rejected.iter().any(|r| r == name)
    }
}

fn expect_validation_error(result: Result<(), SkillError>) -> Vec<String> {
    match result {
        Ok(()) => panic!("expected ValidationError, got Ok(())"),
        Err(SkillError::ValidationError { reasons, .. }) => reasons,
        Err(other) => panic!("expected ValidationError, got: {:?}", other),
    }
}

#[test]
fn valid_manifest_passes() {
    let m = valid_manifest();
    assert!(validate(&m, &AllToolsExist).is_ok());
}

#[test]
fn confidence_threshold_above_one_fails() {
    let mut m = valid_manifest();
    m.constraints.confidence_threshold = 1.5;
    let reasons = expect_validation_error(validate(&m, &AllToolsExist));
    assert!(reasons.iter().any(|r| r.contains("confidence_threshold")));
}

#[test]
fn confidence_threshold_negative_fails() {
    let mut m = valid_manifest();
    m.constraints.confidence_threshold = -0.1;
    let reasons = expect_validation_error(validate(&m, &AllToolsExist));
    assert!(reasons.iter().any(|r| r.contains("confidence_threshold")));
}

#[test]
fn confidence_threshold_boundary_zero_passes() {
    let mut m = valid_manifest();
    m.constraints.confidence_threshold = 0.0;
    assert!(validate(&m, &AllToolsExist).is_ok());
}

#[test]
fn confidence_threshold_boundary_one_passes() {
    let mut m = valid_manifest();
    m.constraints.confidence_threshold = 1.0;
    assert!(validate(&m, &AllToolsExist).is_ok());
}

#[test]
fn max_turns_zero_fails() {
    let mut m = valid_manifest();
    m.constraints.max_turns = 0;
    let reasons = expect_validation_error(validate(&m, &AllToolsExist));
    assert!(reasons.iter().any(|r| r.contains("max_turns")));
}

#[test]
fn max_turns_one_passes() {
    let mut m = valid_manifest();
    m.constraints.max_turns = 1;
    assert!(validate(&m, &AllToolsExist).is_ok());
}

#[test]
fn unknown_tool_name_fails() {
    let mut m = valid_manifest();
    m.tools.push("nonexistent_tool".to_string());
    let checker = RejectTools {
        rejected: vec!["nonexistent_tool".to_string()],
    };
    let reasons = expect_validation_error(validate(&m, &checker));
    assert!(reasons.iter().any(|r| r.contains("nonexistent_tool")));
}

#[test]
fn empty_name_fails() {
    let mut m = valid_manifest();
    m.name = "".to_string();
    let reasons = expect_validation_error(validate(&m, &AllToolsExist));
    assert!(reasons.iter().any(|r| r.contains("name")));
}

#[test]
fn empty_version_fails() {
    let mut m = valid_manifest();
    m.version = "".to_string();
    let reasons = expect_validation_error(validate(&m, &AllToolsExist));
    assert!(reasons.iter().any(|r| r.contains("version")));
}

#[test]
fn empty_preamble_fails() {
    let mut m = valid_manifest();
    m.preamble = "".to_string();
    let reasons = expect_validation_error(validate(&m, &AllToolsExist));
    assert!(reasons.iter().any(|r| r.contains("preamble")));
}

#[test]
fn empty_model_provider_fails() {
    let mut m = valid_manifest();
    m.model.provider = "".to_string();
    let reasons = expect_validation_error(validate(&m, &AllToolsExist));
    assert!(reasons.iter().any(|r| r.contains("provider")));
}

#[test]
fn empty_model_name_fails() {
    let mut m = valid_manifest();
    m.model.name = "".to_string();
    let reasons = expect_validation_error(validate(&m, &AllToolsExist));
    assert!(reasons.iter().any(|r| r.contains("model.name")));
}

#[test]
fn unrecognized_output_format_fails() {
    let mut m = valid_manifest();
    m.output.format = "raw".to_string();
    let reasons = expect_validation_error(validate(&m, &AllToolsExist));
    assert!(reasons.iter().any(|r| r.contains("format")));
}

#[test]
fn recognized_output_formats_pass() {
    for fmt in &["json", "structured_json", "text"] {
        let mut m = valid_manifest();
        m.output.format = fmt.to_string();
        assert!(
            validate(&m, &AllToolsExist).is_ok(),
            "expected format '{fmt}' to pass validation"
        );
    }
}

#[test]
fn multiple_violations_collected() {
    let mut m = valid_manifest();
    m.name = "".to_string();
    m.constraints.confidence_threshold = 2.0;
    let reasons = expect_validation_error(validate(&m, &AllToolsExist));
    assert!(
        reasons.len() >= 2,
        "expected at least 2 reasons, got {}: {:?}",
        reasons.len(),
        reasons
    );
    assert!(reasons.iter().any(|r| r.contains("name")));
    assert!(reasons.iter().any(|r| r.contains("confidence")));
}

#[test]
fn escalate_to_empty_string_fails() {
    let mut m = valid_manifest();
    m.constraints.escalate_to = Some("".to_string());
    let reasons = expect_validation_error(validate(&m, &AllToolsExist));
    assert!(reasons.iter().any(|r| r.contains("escalate_to")));
}

#[test]
fn escalate_to_none_passes() {
    let mut m = valid_manifest();
    m.constraints.escalate_to = None;
    assert!(validate(&m, &AllToolsExist).is_ok());
}
