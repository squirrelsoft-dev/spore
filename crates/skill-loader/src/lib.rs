mod error;
mod frontmatter;

pub use error::SkillError;

use std::path::PathBuf;
use std::sync::Arc;

use agent_sdk::SkillManifest;
use tool_registry::ToolRegistry;

pub struct SkillLoader {
    skill_dir: PathBuf,
    #[allow(dead_code)]
    tool_registry: Arc<ToolRegistry>,
}

impl SkillLoader {
    pub fn new(skill_dir: PathBuf, tool_registry: Arc<ToolRegistry>) -> Self {
        Self {
            skill_dir,
            tool_registry,
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

        let (yaml, body) = frontmatter::extract_frontmatter(&content).map_err(|err| match err {
            SkillError::ParseError { source, .. } => SkillError::ParseError {
                path: path.clone(),
                source,
            },
            other => other,
        })?;

        let fm: frontmatter::SkillFrontmatter =
            serde_yaml::from_str(yaml).map_err(|err| SkillError::ParseError {
                path: path.clone(),
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
}
