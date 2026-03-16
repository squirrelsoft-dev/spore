use std::sync::Arc;

use skill_loader::{AllToolsExist, SkillLoader};
use tool_registry::ToolRegistry;

fn skills_dir() -> std::path::PathBuf {
    let manifest_dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    manifest_dir
        .join("../../skills")
        .canonicalize()
        .expect("skills/ directory must exist")
}

fn make_loader(dir: &std::path::Path) -> SkillLoader {
    let registry = Arc::new(ToolRegistry::new());
    SkillLoader::new(dir.to_path_buf(), registry, Box::new(AllToolsExist))
}

#[tokio::test]
async fn load_cogs_analyst_skill() {
    let dir = skills_dir();
    let loader = make_loader(&dir);
    let manifest = loader.load("cogs-analyst").await.unwrap();

    assert_eq!(manifest.name, "cogs-analyst");
    assert_eq!(manifest.version, "1.0.0");
    assert_eq!(manifest.description, "Handles COGS-related finance queries");

    assert_eq!(manifest.model.provider, "anthropic");
    assert_eq!(manifest.model.name, "claude-sonnet-4-6");
    assert!((manifest.model.temperature - 0.1).abs() < f64::EPSILON);

    assert_eq!(
        manifest.tools,
        vec!["get_account_groups", "execute_sql", "query_store_lookup"]
    );

    assert_eq!(manifest.constraints.max_turns, 5);
    assert!((manifest.constraints.confidence_threshold - 0.75).abs() < f64::EPSILON);
    assert_eq!(
        manifest.constraints.escalate_to,
        Some("general-finance-agent".to_string())
    );
    assert_eq!(manifest.constraints.allowed_actions, vec!["read", "query"]);

    assert_eq!(manifest.output.format, "structured_json");
    assert_eq!(manifest.output.schema.len(), 4);
    assert_eq!(manifest.output.schema.get("sql").unwrap(), "string");
    assert_eq!(manifest.output.schema.get("explanation").unwrap(), "string");
    assert_eq!(manifest.output.schema.get("confidence").unwrap(), "float");
    assert_eq!(manifest.output.schema.get("source").unwrap(), "string");

    assert!(!manifest.preamble.is_empty());
    assert!(manifest.preamble.contains("COGS"));
}

#[tokio::test]
async fn load_echo_skill() {
    let dir = skills_dir();
    let loader = make_loader(&dir);
    let manifest = loader.load("echo").await.unwrap();

    assert_eq!(manifest.name, "echo");
    assert_eq!(manifest.version, "1.0");
    assert_eq!(manifest.description, "Echoes input back for testing");

    assert_eq!(manifest.model.provider, "anthropic");
    assert_eq!(manifest.model.name, "claude-haiku-4-5-20251001");
    assert!((manifest.model.temperature - 0.0).abs() < f64::EPSILON);

    assert!(manifest.tools.is_empty());

    assert_eq!(manifest.constraints.max_turns, 1);
    assert!((manifest.constraints.confidence_threshold - 1.0).abs() < f64::EPSILON);
    assert_eq!(manifest.constraints.escalate_to, None);
    assert!(manifest.constraints.allowed_actions.is_empty());

    assert_eq!(manifest.output.format, "text");
    assert!(manifest.output.schema.is_empty());

    assert_eq!(
        manifest.preamble,
        "Echo back the input exactly as received. Do not modify, summarize, or interpret."
    );
}

#[tokio::test]
async fn load_skill_writer_skill() {
    let dir = skills_dir();
    let loader = make_loader(&dir);
    let manifest = loader.load("skill-writer").await.unwrap();

    assert_eq!(manifest.name, "skill-writer");
    assert_eq!(manifest.version, "0.1");
    assert_eq!(
        manifest.description,
        "Produces validated skill files from plain-language descriptions"
    );

    assert_eq!(manifest.model.provider, "anthropic");
    assert_eq!(manifest.model.name, "claude-sonnet-4-6");
    assert!((manifest.model.temperature - 0.2).abs() < f64::EPSILON);

    assert_eq!(manifest.tools, vec!["write_file", "validate_skill"]);

    assert_eq!(manifest.constraints.max_turns, 10);
    assert!((manifest.constraints.confidence_threshold - 0.9).abs() < f64::EPSILON);
    assert_eq!(manifest.constraints.escalate_to, None);
    assert_eq!(manifest.constraints.allowed_actions, vec!["read", "write"]);

    assert_eq!(manifest.output.format, "structured_json");
    assert_eq!(manifest.output.schema.len(), 2);
    assert_eq!(
        manifest.output.schema.get("skill_yaml").unwrap(),
        "string"
    );
    assert_eq!(
        manifest.output.schema.get("validation_result").unwrap(),
        "string"
    );

    assert!(!manifest.preamble.is_empty());
    assert!(manifest.preamble.contains('\n'));
}
