use std::collections::HashMap;

use agent_sdk::{Constraints, ModelConfig, OutputSchema, SkillManifest};
use tool_registry::{RegistryError, ToolEntry, ToolExists, ToolRegistry};

fn make_entry(name: &str) -> ToolEntry {
    ToolEntry {
        name: name.to_string(),
        version: "1.0.0".to_string(),
        endpoint: "http://localhost:8080".to_string(),
    }
}

fn make_manifest(tools: Vec<String>) -> SkillManifest {
    SkillManifest {
        name: "test-skill".to_string(),
        version: "0.1.0".to_string(),
        description: "test".to_string(),
        model: ModelConfig {
            provider: "anthropic".to_string(),
            name: "claude".to_string(),
            temperature: 0.0,
        },
        preamble: String::new(),
        tools,
        constraints: Constraints {
            max_turns: 1,
            confidence_threshold: 0.5,
            escalate_to: None,
            allowed_actions: vec![],
        },
        output: OutputSchema {
            format: "json".to_string(),
            schema: HashMap::new(),
        },
    }
}

#[test]
fn register_and_get() {
    let registry = ToolRegistry::new();
    let entry = make_entry("web_search");

    registry.register(entry.clone()).unwrap();

    let retrieved = registry.get("web_search");
    assert_eq!(retrieved, Some(entry));
}

#[test]
fn assert_exists_returns_ok_for_registered_tool() {
    let registry = ToolRegistry::new();
    registry.register(make_entry("file_read")).unwrap();

    let result = registry.assert_exists("file_read");
    assert_eq!(result, Ok(()));
}

#[test]
fn assert_exists_returns_error_for_missing_tool() {
    let registry = ToolRegistry::new();

    let result = registry.assert_exists("nonexistent");
    assert_eq!(
        result,
        Err(RegistryError::ToolNotFound {
            name: "nonexistent".into()
        })
    );
}

#[test]
fn register_duplicate_returns_error() {
    let registry = ToolRegistry::new();
    registry.register(make_entry("web_search")).unwrap();

    let result = registry.register(make_entry("web_search"));
    assert_eq!(
        result,
        Err(RegistryError::DuplicateEntry {
            name: "web_search".into()
        })
    );
}

#[test]
fn resolve_for_skill_returns_matching_entries() {
    let registry = ToolRegistry::new();
    registry.register(make_entry("web_search")).unwrap();
    registry.register(make_entry("file_read")).unwrap();
    registry.register(make_entry("shell_exec")).unwrap();

    let manifest = make_manifest(vec!["web_search".to_string(), "file_read".to_string()]);

    let mut entries = registry.resolve_for_skill(&manifest).unwrap();
    entries.sort_by(|a, b| a.name.cmp(&b.name));

    assert_eq!(entries.len(), 2);
    assert_eq!(entries[0], make_entry("file_read"));
    assert_eq!(entries[1], make_entry("web_search"));
}

#[test]
fn resolve_for_skill_fails_on_missing_tool() {
    let registry = ToolRegistry::new();
    registry.register(make_entry("web_search")).unwrap();

    let manifest = make_manifest(vec!["web_search".to_string(), "missing_tool".to_string()]);

    let result = registry.resolve_for_skill(&manifest);
    assert_eq!(
        result,
        Err(RegistryError::ToolNotFound {
            name: "missing_tool".into()
        })
    );
}

#[test]
fn get_returns_none_for_missing_tool() {
    let registry = ToolRegistry::new();

    let result = registry.get("nonexistent");
    assert_eq!(result, None);
}

#[test]
fn tool_exists_trait_impl() {
    let registry = ToolRegistry::new();
    registry.register(make_entry("web_search")).unwrap();

    let checker: &dyn ToolExists = &registry;
    assert!(checker.tool_exists("web_search"));
    assert!(!checker.tool_exists("nonexistent"));
}
