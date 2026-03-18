use std::collections::HashMap;

use agent_sdk::{Constraints, ModelConfig, OutputSchema, SkillManifest};
use tool_registry::{RegistryError, ToolEntry, ToolExists, ToolRegistry};

fn make_entry(name: &str) -> ToolEntry {
    ToolEntry {
        name: name.to_string(),
        version: "1.0.0".to_string(),
        endpoint: "http://localhost:8080".to_string(),
        action_type: None,
        handle: None,
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

// --- allowed_actions filtering tests ---

fn make_entry_with_action(name: &str, action_type: Option<&str>) -> ToolEntry {
    ToolEntry {
        name: name.to_string(),
        version: "1.0.0".to_string(),
        endpoint: "http://localhost:8080".to_string(),
        action_type: action_type.map(|s| s.to_string()),
        handle: None,
    }
}

/// Mirrors the allowed_actions filtering logic from `resolve_mcp_tools` in
/// `agent-runtime/src/tool_bridge.rs`. When `allowed_actions` is empty, all
/// entries pass through. Otherwise, entries with no `action_type` are always
/// included, and entries whose `action_type` matches any allowed action are
/// included.
fn filter_by_allowed_actions(entries: Vec<ToolEntry>, allowed_actions: &[&str]) -> Vec<ToolEntry> {
    if allowed_actions.is_empty() {
        entries
    } else {
        entries
            .into_iter()
            .filter(|entry| match &entry.action_type {
                None => true,
                Some(t) => allowed_actions.iter().any(|a| a == t),
            })
            .collect()
    }
}

#[test]
fn allowed_actions_excludes_non_matching_action_type() {
    let entries = vec![
        make_entry_with_action("tool_read", Some("read")),
        make_entry_with_action("tool_write", Some("write")),
        make_entry_with_action("tool_query", Some("query")),
    ];

    let result = filter_by_allowed_actions(entries, &["read", "query"]);

    assert_eq!(result.len(), 2);
    assert_eq!(result[0].name, "tool_read");
    assert_eq!(result[1].name, "tool_query");
}

#[test]
fn allowed_actions_includes_tools_with_no_action_type() {
    let entries = vec![
        make_entry_with_action("tool_none", None),
        make_entry_with_action("tool_write", Some("write")),
    ];

    let result = filter_by_allowed_actions(entries, &["read"]);

    assert_eq!(result.len(), 1);
    assert_eq!(result[0].name, "tool_none");
}

#[test]
fn empty_allowed_actions_passes_all_tools() {
    let entries = vec![
        make_entry_with_action("tool_read", Some("read")),
        make_entry_with_action("tool_write", Some("write")),
        make_entry_with_action("tool_none", None),
    ];

    let result = filter_by_allowed_actions(entries, &[]);

    assert_eq!(result.len(), 3);
    assert_eq!(result[0].name, "tool_read");
    assert_eq!(result[1].name, "tool_write");
    assert_eq!(result[2].name, "tool_none");
}

#[test]
fn allowed_actions_excludes_all_when_none_match() {
    let entries = vec![
        make_entry_with_action("tool_write", Some("write")),
        make_entry_with_action("tool_admin", Some("admin")),
    ];

    let result = filter_by_allowed_actions(entries, &["read"]);

    assert_eq!(result.len(), 0);
}
