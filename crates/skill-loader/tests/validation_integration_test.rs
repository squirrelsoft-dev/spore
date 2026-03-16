use std::sync::Arc;

use skill_loader::{AllToolsExist, SkillError, SkillLoader};
use tempfile::tempdir;
use tokio::fs;
use tool_registry::ToolRegistry;

fn make_loader(dir: &std::path::Path) -> SkillLoader {
    let registry = Arc::new(ToolRegistry::new());
    SkillLoader::new(dir.to_path_buf(), registry, Box::new(AllToolsExist))
}

const VALID_FRONTMATTER: &str = "\
---
name: test-skill
version: \"1.0\"
description: A test skill
model:
  provider: anthropic
  name: claude-3-haiku
  temperature: 0.3
tools:
  - web_search
constraints:
  max_turns: 5
  confidence_threshold: 0.85
  allowed_actions:
    - search
output:
  format: json
  schema:
    result: string
---
You are a helpful test assistant.";

#[tokio::test]
async fn load_invalid_confidence_threshold_returns_validation_error() {
    let dir = tempdir().unwrap();
    let content = VALID_FRONTMATTER.replace("confidence_threshold: 0.85", "confidence_threshold: 2.0");

    fs::write(dir.path().join("test-skill.md"), content)
        .await
        .unwrap();

    let loader = make_loader(dir.path());
    let result = loader.load("test-skill").await;
    assert!(result.is_err());

    let err = result.unwrap_err();
    match err {
        SkillError::ValidationError { skill_name, reasons } => {
            assert_eq!(skill_name, "test-skill");
            assert!(
                reasons.iter().any(|r| r.contains("confidence_threshold")),
                "expected reason about confidence_threshold, got: {:?}",
                reasons
            );
        }
        other => panic!("expected ValidationError, got: {:?}", other),
    }
}

#[tokio::test]
async fn load_negative_confidence_threshold_returns_validation_error() {
    let dir = tempdir().unwrap();
    let content =
        VALID_FRONTMATTER.replace("confidence_threshold: 0.85", "confidence_threshold: -0.5");

    fs::write(dir.path().join("test-skill.md"), content)
        .await
        .unwrap();

    let loader = make_loader(dir.path());
    let result = loader.load("test-skill").await;
    assert!(result.is_err());

    let err = result.unwrap_err();
    match err {
        SkillError::ValidationError { skill_name, reasons } => {
            assert_eq!(skill_name, "test-skill");
            assert!(
                reasons.iter().any(|r| r.contains("confidence_threshold")),
                "expected reason about confidence_threshold, got: {:?}",
                reasons
            );
        }
        other => panic!("expected ValidationError, got: {:?}", other),
    }
}

#[tokio::test]
async fn load_multiple_violations_returns_all_reasons() {
    let dir = tempdir().unwrap();
    let content = VALID_FRONTMATTER
        .replace("confidence_threshold: 0.85", "confidence_threshold: 2.0")
        .replace("max_turns: 5", "max_turns: 0")
        .replace("name: test-skill", "name: \"\"");

    fs::write(dir.path().join("test-skill.md"), content)
        .await
        .unwrap();

    let loader = make_loader(dir.path());
    let result = loader.load("test-skill").await;
    assert!(result.is_err());

    let err = result.unwrap_err();
    match err {
        SkillError::ValidationError { skill_name, reasons } => {
            assert_eq!(skill_name, "");
            assert!(
                reasons.len() >= 3,
                "expected at least 3 reasons, got {}: {:?}",
                reasons.len(),
                reasons
            );
            assert!(
                reasons.iter().any(|r| r.contains("confidence_threshold")),
                "expected reason about confidence_threshold, got: {:?}",
                reasons
            );
            assert!(
                reasons.iter().any(|r| r.contains("max_turns")),
                "expected reason about max_turns, got: {:?}",
                reasons
            );
            assert!(
                reasons.iter().any(|r| r.contains("name")),
                "expected reason about name, got: {:?}",
                reasons
            );
        }
        other => panic!("expected ValidationError, got: {:?}", other),
    }
}

#[tokio::test]
async fn load_valid_skill_passes_validation() {
    let dir = tempdir().unwrap();

    fs::write(dir.path().join("test-skill.md"), VALID_FRONTMATTER)
        .await
        .unwrap();

    let loader = make_loader(dir.path());
    let result = loader.load("test-skill").await;
    assert!(result.is_ok(), "expected Ok, got: {:?}", result.unwrap_err());

    let manifest = result.unwrap();
    assert_eq!(manifest.name, "test-skill");
    assert_eq!(manifest.version, "1.0");
    assert_eq!(manifest.description, "A test skill");
    assert_eq!(manifest.model.provider, "anthropic");
    assert_eq!(manifest.model.name, "claude-3-haiku");
    assert!((manifest.model.temperature - 0.3).abs() < f64::EPSILON);
    assert_eq!(manifest.tools, vec!["web_search"]);
    assert_eq!(manifest.constraints.max_turns, 5);
    assert!((manifest.constraints.confidence_threshold - 0.85).abs() < f64::EPSILON);
    assert_eq!(manifest.constraints.escalate_to, None);
    assert_eq!(manifest.constraints.allowed_actions, vec!["search"]);
    assert_eq!(manifest.output.format, "json");
    assert_eq!(manifest.output.schema.get("result").unwrap(), "string");
    assert_eq!(manifest.preamble, "You are a helpful test assistant.");
}

#[tokio::test]
async fn load_invalid_output_format_returns_validation_error() {
    let dir = tempdir().unwrap();
    let content = VALID_FRONTMATTER.replace("format: json", "format: raw");

    fs::write(dir.path().join("test-skill.md"), content)
        .await
        .unwrap();

    let loader = make_loader(dir.path());
    let result = loader.load("test-skill").await;
    assert!(result.is_err());

    let err = result.unwrap_err();
    match err {
        SkillError::ValidationError { skill_name, reasons } => {
            assert_eq!(skill_name, "test-skill");
            assert!(
                reasons.iter().any(|r| r.contains("format")),
                "expected reason about format, got: {:?}",
                reasons
            );
        }
        other => panic!("expected ValidationError, got: {:?}", other),
    }
}

#[tokio::test]
async fn load_zero_max_turns_returns_validation_error() {
    let dir = tempdir().unwrap();
    let content = VALID_FRONTMATTER.replace("max_turns: 5", "max_turns: 0");

    fs::write(dir.path().join("test-skill.md"), content)
        .await
        .unwrap();

    let loader = make_loader(dir.path());
    let result = loader.load("test-skill").await;
    assert!(result.is_err());

    let err = result.unwrap_err();
    match err {
        SkillError::ValidationError { skill_name, reasons } => {
            assert_eq!(skill_name, "test-skill");
            assert!(
                reasons.iter().any(|r| r.contains("max_turns")),
                "expected reason about max_turns, got: {:?}",
                reasons
            );
        }
        other => panic!("expected ValidationError, got: {:?}", other),
    }
}

#[tokio::test]
async fn load_empty_preamble_returns_validation_error() {
    let dir = tempdir().unwrap();
    let content = "\
---
name: test-skill
version: \"1.0\"
description: A test skill
model:
  provider: anthropic
  name: claude-3-haiku
  temperature: 0.3
tools:
  - web_search
constraints:
  max_turns: 5
  confidence_threshold: 0.85
  allowed_actions:
    - search
output:
  format: json
  schema:
    result: string
---";

    fs::write(dir.path().join("test-skill.md"), content)
        .await
        .unwrap();

    let loader = make_loader(dir.path());
    let result = loader.load("test-skill").await;
    assert!(result.is_err());

    let err = result.unwrap_err();
    match err {
        SkillError::ValidationError { skill_name, reasons } => {
            assert_eq!(skill_name, "test-skill");
            assert!(
                reasons.iter().any(|r| r.contains("preamble")),
                "expected reason about preamble, got: {:?}",
                reasons
            );
        }
        other => panic!("expected ValidationError, got: {:?}", other),
    }
}
