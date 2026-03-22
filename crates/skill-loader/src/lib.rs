mod error;
mod frontmatter;
pub mod validation;

pub use error::SkillError;
pub use tool_registry::ToolExists;
pub use validation::{AllToolsExist, validate};

use std::path::PathBuf;
use std::sync::Arc;

use agent_sdk::SkillManifest;
use tool_registry::ToolRegistry;

/// Parses a skill file's raw string content into a `SkillManifest`
/// without performing any filesystem I/O or validation.
///
/// Callers are responsible for running [`validate`] separately if needed.
pub fn parse_content(content: &str) -> Result<SkillManifest, SkillError> {
    let placeholder = PathBuf::from("<content>");

    let (yaml, body) =
        frontmatter::extract_frontmatter(content).map_err(|err| match err {
            SkillError::ParseError { source, .. } => SkillError::ParseError {
                path: placeholder.clone(),
                source,
            },
            other => other,
        })?;

    let fm: frontmatter::SkillFrontmatter =
        serde_yaml::from_str(yaml).map_err(|err| SkillError::ParseError {
            path: placeholder,
            source: err.to_string(),
        })?;

    Ok(SkillManifest {
        name: fm.name,
        version: fm.version,
        description: fm.description,
        model: fm.model,
        preamble: body.trim().to_string(),
        tools: fm.tools,
        constraints: fm.constraints,
        output: fm.output,
    })
}

pub struct SkillLoader {
    skill_dir: PathBuf,
    #[allow(dead_code)]
    tool_registry: Arc<ToolRegistry>,
    tool_checker: Box<dyn ToolExists + Send + Sync>,
}

impl SkillLoader {
    pub fn new(
        skill_dir: PathBuf,
        tool_registry: Arc<ToolRegistry>,
        tool_checker: Box<dyn ToolExists + Send + Sync>,
    ) -> Self {
        Self {
            skill_dir,
            tool_registry,
            tool_checker,
        }
    }

    pub async fn load(&self, skill_name: &str) -> Result<SkillManifest, SkillError> {
        let path = self.skill_dir.join(format!("{skill_name}.md"));

        let content = tokio::fs::read_to_string(&path)
            .await
            .map_err(|err| SkillError::IoError {
                path: path.clone(),
                source: err.to_string(),
            })?;

        let manifest = parse_content(&content).map_err(|err| match err {
            SkillError::ParseError { source, .. } => SkillError::ParseError {
                path: path.clone(),
                source,
            },
            other => other,
        })?;

        validate(&manifest, &*self.tool_checker)?;
        Ok(manifest)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_content_valid_returns_expected_manifest() {
        let content = mcp_test_utils::valid_skill_content();
        let manifest = parse_content(&content).unwrap();

        assert_eq!(manifest.name, "test-skill");
        assert_eq!(manifest.version, "1.0.0");
        assert_eq!(manifest.description, "A test skill");
        assert_eq!(manifest.model.provider, "openai");
        assert_eq!(manifest.model.name, "gpt-4");
        assert_eq!(manifest.tools, vec!["read_file", "write_file"]);
        assert_eq!(manifest.preamble, "This is the preamble body.");
        assert_eq!(manifest.output.format, "json");
    }

    #[test]
    fn parse_content_missing_opening_delimiter_returns_parse_error() {
        let content = "name: test\nversion: 1.0\n---\nbody";
        let err = parse_content(content).unwrap_err();
        assert!(matches!(err, SkillError::ParseError { .. }));
        if let SkillError::ParseError { path, source } = &err {
            assert_eq!(path, &PathBuf::from("<content>"));
            assert!(source.contains("opening"));
        }
    }

    #[test]
    fn parse_content_missing_closing_delimiter_returns_parse_error() {
        let content = "---\nname: test\nversion: 1.0\nno closing";
        let err = parse_content(content).unwrap_err();
        assert!(matches!(err, SkillError::ParseError { .. }));
        if let SkillError::ParseError { path, source } = &err {
            assert_eq!(path, &PathBuf::from("<content>"));
            assert!(source.contains("closing"));
        }
    }

    #[test]
    fn parse_content_invalid_yaml_fields_returns_parse_error() {
        let content = "---\nunknown_only: true\n---\nbody";
        let err = parse_content(content).unwrap_err();
        assert!(matches!(err, SkillError::ParseError { .. }));
        if let SkillError::ParseError { path, .. } = &err {
            assert_eq!(path, &PathBuf::from("<content>"));
        }
    }

    #[test]
    fn parse_content_accepts_markdown_output_format() {
        let content = mcp_test_utils::valid_skill_content()
            .replace("format: json", "format: markdown");
        let manifest = parse_content(&content).unwrap();
        assert_eq!(manifest.output.format, "markdown");
    }

    #[test]
    fn parse_content_body_is_trimmed() {
        let content = mcp_test_utils::valid_skill_content().replace(
            "This is the preamble body.",
            "  \n  Trimmed body text.  \n  ",
        );
        let manifest = parse_content(&content).unwrap();
        assert_eq!(manifest.preamble, "Trimmed body text.");
    }
}
