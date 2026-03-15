use std::collections::HashMap;

use agent_sdk::{Constraints, ModelConfig, OutputSchema, SkillManifest};

const CANONICAL_YAML: &str = r#"
name: summarize
version: "1.0"
description: Summarize input text
model:
  provider: anthropic
  name: claude-3-haiku
  temperature: 0.3
preamble: You are a summarization assistant.
tools:
  - web_search
  - file_read
constraints:
  max_turns: 5
  confidence_threshold: 0.85
  escalate_to: human_reviewer
  allowed_actions:
    - summarize
    - clarify
output:
  format: json
  schema:
    summary: string
    confidence: number
"#;

#[test]
fn deserialize_readme_skill_file() {
    let manifest: SkillManifest =
        serde_yaml::from_str(CANONICAL_YAML).expect("failed to deserialize canonical YAML");

    assert_eq!(manifest.name, "summarize");
    assert_eq!(manifest.version, "1.0");
    assert_eq!(manifest.description, "Summarize input text");

    assert_eq!(manifest.model.provider, "anthropic");
    assert_eq!(manifest.model.name, "claude-3-haiku");
    assert!((manifest.model.temperature - 0.3).abs() < f64::EPSILON);

    assert_eq!(manifest.preamble, "You are a summarization assistant.");

    assert_eq!(manifest.tools, vec!["web_search", "file_read"]);

    assert_eq!(manifest.constraints.max_turns, 5);
    assert!((manifest.constraints.confidence_threshold - 0.85).abs() < f64::EPSILON);
    assert_eq!(manifest.constraints.escalate_to, "human_reviewer");
    assert_eq!(
        manifest.constraints.allowed_actions,
        vec!["summarize", "clarify"]
    );

    assert_eq!(manifest.output.format, "json");
    assert_eq!(manifest.output.schema.get("summary").unwrap(), "string");
    assert_eq!(manifest.output.schema.get("confidence").unwrap(), "number");
    assert_eq!(manifest.output.schema.len(), 2);
}

#[test]
fn serialize_deserialize_round_trip() {
    let original = SkillManifest {
        name: "analyze".to_string(),
        version: "2.0".to_string(),
        description: "Analyze input data".to_string(),
        model: ModelConfig {
            provider: "openai".to_string(),
            name: "gpt-4".to_string(),
            temperature: 0.7,
        },
        preamble: "You are an analysis assistant.".to_string(),
        tools: vec!["calculator".to_string(), "data_query".to_string()],
        constraints: Constraints {
            max_turns: 10,
            confidence_threshold: 0.9,
            escalate_to: "senior_analyst".to_string(),
            allowed_actions: vec!["analyze".to_string(), "report".to_string()],
        },
        output: OutputSchema {
            format: "json".to_string(),
            schema: HashMap::from([
                ("result".to_string(), "string".to_string()),
                ("score".to_string(), "number".to_string()),
            ]),
        },
    };

    let yaml = serde_yaml::to_string(&original).expect("failed to serialize to YAML");
    let deserialized: SkillManifest =
        serde_yaml::from_str(&yaml).expect("failed to deserialize from YAML");

    assert_eq!(original, deserialized);
}

#[test]
fn deserialize_empty_tools_list() {
    let yaml = r#"
name: minimal
version: "1.0"
description: Minimal skill
model:
  provider: anthropic
  name: claude-3-haiku
  temperature: 0.5
preamble: You are an assistant.
tools: []
constraints:
  max_turns: 1
  confidence_threshold: 0.5
  escalate_to: nobody
  allowed_actions: []
output:
  format: text
  schema:
    body: string
"#;

    let manifest: SkillManifest =
        serde_yaml::from_str(yaml).expect("failed to deserialize YAML with empty tools");

    assert!(manifest.tools.is_empty());
}

#[test]
fn deserialize_empty_schema_map() {
    let yaml = r#"
name: no-schema
version: "1.0"
description: Skill with empty schema
model:
  provider: anthropic
  name: claude-3-haiku
  temperature: 0.5
preamble: You are an assistant.
tools:
  - web_search
constraints:
  max_turns: 3
  confidence_threshold: 0.7
  escalate_to: fallback
  allowed_actions:
    - search
output:
  format: raw
  schema: {}
"#;

    let manifest: SkillManifest =
        serde_yaml::from_str(yaml).expect("failed to deserialize YAML with empty schema");

    assert!(manifest.output.schema.is_empty());
}
