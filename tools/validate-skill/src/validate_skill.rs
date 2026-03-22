use rmcp::{
    handler::server::{router::tool::ToolRouter, wrapper::Parameters},
    model::{ServerCapabilities, ServerInfo},
    schemars, tool, tool_handler, tool_router, ServerHandler,
};

/// Request payload for the validate_skill tool.
#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct ValidateSkillRequest {
    /// The raw skill file content (markdown with YAML frontmatter) to validate.
    pub content: String,
}

#[derive(Debug, Clone)]
pub struct ValidateSkillTool {
    tool_router: ToolRouter<Self>,
}

impl ValidateSkillTool {
    pub fn new() -> Self {
        Self {
            tool_router: Self::tool_router(),
        }
    }
}

#[tool_router]
impl ValidateSkillTool {
    /// Parses and validates a skill file's content, returning structured
    /// JSON indicating whether the content is valid.
    #[tool(description = "Validates skill file content (markdown with YAML frontmatter) and returns parse/validation errors if any")]
    fn validate_skill(&self, Parameters(request): Parameters<ValidateSkillRequest>) -> String {
        let manifest = match skill_loader::parse_content(&request.content) {
            Ok(m) => m,
            Err(e) => return error_response(vec![e.to_string()]),
        };

        if let Err(e) = skill_loader::validate(&manifest, &skill_loader::AllToolsExist) {
            if let skill_loader::SkillError::ValidationError { reasons, .. } = e {
                return error_response(reasons);
            }
            return error_response(vec![e.to_string()]);
        }

        success_response(&manifest)
    }
}

fn error_response(errors: Vec<String>) -> String {
    serde_json::json!({
        "valid": false,
        "errors": errors,
    })
    .to_string()
}

fn success_response(manifest: &agent_sdk::SkillManifest) -> String {
    serde_json::json!({
        "valid": true,
        "errors": [],
        "manifest": manifest,
    })
    .to_string()
}

#[tool_handler]
impl ServerHandler for ValidateSkillTool {
    fn get_info(&self) -> ServerInfo {
        ServerInfo::new(ServerCapabilities::builder().enable_tools().build())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn valid_content() -> String {
        r#"---
name: test-skill
version: "1.0.0"
description: A test skill
model:
  provider: openai
  name: gpt-4
  temperature: 0.7
tools:
  - read_file
  - write_file
constraints:
  confidence_threshold: 0.8
  max_turns: 5
  allowed_actions:
    - read
    - write
output:
  format: json
  schema:
    result: string
---
This is the preamble body."#
            .to_string()
    }

    fn call_validate(tool: &ValidateSkillTool, content: &str) -> serde_json::Value {
        let result = tool.validate_skill(Parameters(ValidateSkillRequest {
            content: content.to_string(),
        }));
        serde_json::from_str(&result).expect("response should be valid JSON")
    }

    #[tokio::test]
    async fn valid_content_returns_success() {
        let tool = ValidateSkillTool::new();
        let result = call_validate(&tool, &valid_content());

        assert_eq!(result["valid"], true);
        let errors = result["errors"].as_array().unwrap();
        assert!(errors.is_empty());
        assert_eq!(result["manifest"]["name"], "test-skill");
        assert_eq!(result["manifest"]["version"], "1.0.0");
    }

    #[tokio::test]
    async fn missing_frontmatter_returns_error() {
        let tool = ValidateSkillTool::new();
        let result = call_validate(&tool, "no frontmatter here");

        assert_eq!(result["valid"], false);
        let errors = result["errors"].as_array().unwrap();
        assert!(!errors.is_empty());
        let first = errors[0].as_str().unwrap();
        assert!(
            first.contains("opening"),
            "expected mention of 'opening' delimiter in: {first}"
        );
    }

    #[tokio::test]
    async fn invalid_yaml_returns_error() {
        let content = "---\nunknown_only: true\n---\nbody";
        let tool = ValidateSkillTool::new();
        let result = call_validate(&tool, content);

        assert_eq!(result["valid"], false);
        let errors = result["errors"].as_array().unwrap();
        assert!(!errors.is_empty());
    }

    #[tokio::test]
    async fn validation_failure_returns_reasons() {
        // Valid YAML structure but empty name triggers validation error
        let content = r#"---
name: ""
version: "1.0.0"
description: A test skill
model:
  provider: openai
  name: gpt-4
  temperature: 0.7
tools:
  - read_file
constraints:
  confidence_threshold: 0.8
  max_turns: 5
  allowed_actions:
    - read
output:
  format: json
  schema:
    result: string
---
This is the preamble body."#;

        let tool = ValidateSkillTool::new();
        let result = call_validate(&tool, content);

        assert_eq!(result["valid"], false);
        let errors = result["errors"].as_array().unwrap();
        assert!(!errors.is_empty());
        let joined: String = errors
            .iter()
            .map(|e| e.as_str().unwrap().to_string())
            .collect::<Vec<_>>()
            .join(" ");
        assert!(
            joined.contains("name"),
            "expected validation error about 'name' in: {joined}"
        );
    }

    #[tokio::test]
    async fn empty_content_returns_error() {
        let tool = ValidateSkillTool::new();
        let result = call_validate(&tool, "");

        assert_eq!(result["valid"], false);
        let errors = result["errors"].as_array().unwrap();
        assert!(!errors.is_empty());
    }
}
