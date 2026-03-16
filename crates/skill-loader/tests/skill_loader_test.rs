use std::sync::Arc;

use skill_loader::{SkillError, SkillLoader};
use tempfile::tempdir;
use tokio::fs;
use tool_registry::ToolRegistry;

fn make_loader(dir: &std::path::Path) -> SkillLoader {
    let registry = Arc::new(ToolRegistry);
    SkillLoader::new(dir.to_path_buf(), registry)
}

#[tokio::test]
async fn load_valid_skill_with_full_frontmatter() {
    let dir = tempdir().unwrap();
    let content = "\
---
name: summarize
version: \"1.0\"
description: Summarize input text
model:
  provider: anthropic
  name: claude-3-haiku
  temperature: 0.3
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
---
You are a summarization assistant.";

    fs::write(dir.path().join("summarize.md"), content)
        .await
        .unwrap();

    let loader = make_loader(dir.path());
    let manifest = loader.load("summarize").await.unwrap();

    assert_eq!(manifest.name, "summarize");
    assert_eq!(manifest.version, "1.0");
    assert_eq!(manifest.description, "Summarize input text");

    assert_eq!(manifest.model.provider, "anthropic");
    assert_eq!(manifest.model.name, "claude-3-haiku");
    assert!((manifest.model.temperature - 0.3).abs() < f64::EPSILON);

    assert_eq!(manifest.tools, vec!["web_search", "file_read"]);

    assert_eq!(manifest.constraints.max_turns, 5);
    assert!(
        (manifest.constraints.confidence_threshold - 0.85).abs() < f64::EPSILON
    );
    assert_eq!(manifest.constraints.escalate_to, "human_reviewer");
    assert_eq!(
        manifest.constraints.allowed_actions,
        vec!["summarize", "clarify"]
    );

    assert_eq!(manifest.output.format, "json");
    assert_eq!(
        manifest.output.schema.get("summary").unwrap(),
        "string"
    );
    assert_eq!(
        manifest.output.schema.get("confidence").unwrap(),
        "number"
    );

    assert_eq!(manifest.preamble, "You are a summarization assistant.");
}

#[tokio::test]
async fn load_missing_file_returns_io_error() {
    let dir = tempdir().unwrap();
    let loader = make_loader(dir.path());

    let result = loader.load("nonexistent").await;
    assert!(result.is_err());

    let err = result.unwrap_err();
    match err {
        SkillError::IoError { path, .. } => {
            assert!(path.to_string_lossy().contains("nonexistent.md"));
        }
        other => panic!("expected IoError, got: {:?}", other),
    }
}

#[tokio::test]
async fn load_malformed_yaml_returns_parse_error() {
    let dir = tempdir().unwrap();
    let content = "\
---
name: [unterminated
---
body content";

    fs::write(dir.path().join("bad-yaml.md"), content)
        .await
        .unwrap();

    let loader = make_loader(dir.path());
    let result = loader.load("bad-yaml").await;
    assert!(result.is_err());

    let err = result.unwrap_err();
    match err {
        SkillError::ParseError { path, .. } => {
            assert!(path.to_string_lossy().contains("bad-yaml.md"));
        }
        other => panic!("expected ParseError, got: {:?}", other),
    }
}

#[tokio::test]
async fn load_missing_delimiters_returns_parse_error() {
    let dir = tempdir().unwrap();
    let content = "This is plain markdown with no frontmatter delimiters.";

    fs::write(dir.path().join("no-frontmatter.md"), content)
        .await
        .unwrap();

    let loader = make_loader(dir.path());
    let result = loader.load("no-frontmatter").await;
    assert!(result.is_err());

    let err = result.unwrap_err();
    match err {
        SkillError::ParseError { .. } => {}
        other => panic!("expected ParseError, got: {:?}", other),
    }
}

#[tokio::test]
async fn load_skill_with_empty_body() {
    let dir = tempdir().unwrap();
    let content = "\
---
name: minimal
version: \"0.1\"
description: A minimal skill
model:
  provider: openai
  name: gpt-4
  temperature: 0.5
tools:
  - shell
constraints:
  max_turns: 3
  confidence_threshold: 0.9
  escalate_to: admin
  allowed_actions:
    - execute
output:
  format: text
  schema:
    result: string
---";

    fs::write(dir.path().join("minimal.md"), content)
        .await
        .unwrap();

    let loader = make_loader(dir.path());
    let manifest = loader.load("minimal").await.unwrap();

    assert_eq!(manifest.preamble, "");
}

#[tokio::test]
async fn load_skill_with_rich_markdown_body() {
    let dir = tempdir().unwrap();
    let content = "\
---
name: rich
version: \"2.0\"
description: A skill with rich markdown body
model:
  provider: anthropic
  name: claude-3-opus
  temperature: 0.7
tools:
  - web_search
constraints:
  max_turns: 10
  confidence_threshold: 0.75
  escalate_to: senior_dev
  allowed_actions:
    - search
    - analyze
output:
  format: markdown
  schema:
    report: string
---
# Instructions

Follow these steps carefully.

```python
def hello():
    print(\"hello world\")
```

---

This paragraph comes after a horizontal rule.";

    fs::write(dir.path().join("rich.md"), content)
        .await
        .unwrap();

    let loader = make_loader(dir.path());
    let manifest = loader.load("rich").await.unwrap();

    assert!(manifest.preamble.contains("# Instructions"));
    assert!(manifest.preamble.contains("```python"));
    assert!(manifest.preamble.contains("def hello():"));
    assert!(manifest.preamble.contains("---"));
    assert!(
        manifest
            .preamble
            .contains("This paragraph comes after a horizontal rule.")
    );
}
