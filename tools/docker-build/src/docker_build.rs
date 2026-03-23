use rmcp::{
    handler::server::{router::tool::ToolRouter, wrapper::Parameters},
    model::{ServerCapabilities, ServerInfo},
    schemars, tool, tool_handler, tool_router, ServerHandler,
};
use std::collections::HashMap;

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct DockerBuildRequest {
    /// Build context path (required)
    pub context: String,
    /// Image tag (required)
    pub tag: String,
    /// Optional build arguments
    pub build_args: Option<HashMap<String, String>>,
    /// Optional Dockerfile path
    pub dockerfile: Option<String>,
}

#[derive(Debug, Clone)]
pub struct DockerBuildTool {
    tool_router: ToolRouter<Self>,
}

impl DockerBuildTool {
    pub fn new() -> Self {
        Self {
            tool_router: Self::tool_router(),
        }
    }
}

fn validate_path(path: &str, label: &str) -> Result<std::path::PathBuf, String> {
    let path_obj = std::path::Path::new(path);
    for component in path_obj.components() {
        if let std::path::Component::ParentDir = component {
            return Err(format!("Invalid {label}: path traversal not allowed"));
        }
    }

    let canonical = std::fs::canonicalize(path).map_err(|e| format!("Invalid {label}: {e}"))?;

    let cwd = std::env::current_dir()
        .map_err(|e| format!("Invalid {label}: cannot determine working directory: {e}"))?;

    if !canonical.starts_with(&cwd) {
        return Err(format!("Invalid {label}: path escapes working directory"));
    }

    Ok(canonical)
}

fn validate_tag(tag: &str) -> bool {
    !tag.is_empty()
        && tag
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || "._:/-".contains(ch))
}

fn has_shell_metacharacters(s: &str) -> bool {
    s.chars().any(|ch| ";&|$`\n\r(){}<>'\"\\".contains(ch))
}

fn validate_build_args(args: &HashMap<String, String>) -> Result<(), String> {
    for (key, value) in args {
        if key.is_empty() {
            return Err("Invalid build argument: key must not be empty".to_string());
        }
        if has_shell_metacharacters(key) {
            return Err(format!(
                "Invalid build argument: key contains forbidden characters: {key}"
            ));
        }
        if has_shell_metacharacters(value) {
            return Err(format!(
                "Invalid build argument: value contains forbidden characters for key: {key}"
            ));
        }
    }
    Ok(())
}

fn extract_image_id(output: &str) -> String {
    for line in output.lines() {
        // Legacy format: "Successfully built <id>"
        if let Some(id) = line.split("Successfully built ").nth(1) {
            let id = id.trim();
            if !id.is_empty() {
                return id.to_string();
            }
        }

        // BuildKit format: "writing image sha256:<id>"
        if let Some(rest) = line.split("writing image sha256:").nth(1) {
            let id = rest.split_whitespace().next().unwrap_or("").trim();
            if !id.is_empty() {
                return format!("sha256:{id}");
            }
        }
    }

    String::new()
}

fn execute_docker_build(
    context: &str,
    tag: &str,
    dockerfile: Option<&str>,
    build_args: Option<&HashMap<String, String>>,
) -> String {
    let mut cmd = std::process::Command::new("docker");
    cmd.args(["build", "-t", tag]);

    if let Some(df) = dockerfile {
        cmd.args(["-f", df]);
    }

    if let Some(args) = build_args {
        for (key, value) in args {
            cmd.args(["--build-arg", &format!("{key}={value}")]);
        }
    }

    cmd.arg(context);

    match cmd.output() {
        Ok(output) => {
            let stdout = String::from_utf8_lossy(&output.stdout);
            let stderr = String::from_utf8_lossy(&output.stderr);
            let build_log = format!("{stdout}{stderr}");
            let image_id = extract_image_id(&build_log);

            serde_json::json!({
                "success": output.status.success(),
                "image_id": image_id,
                "tag": tag,
                "build_log": build_log,
            })
            .to_string()
        }
        Err(e) => serde_json::json!({
            "success": false,
            "image_id": "",
            "tag": tag,
            "build_log": format!("Failed to execute docker: {e}"),
        })
        .to_string(),
    }
}

#[tool_router]
impl DockerBuildTool {
    #[tool(description = "Run docker build and return the result with image ID")]
    fn docker_build(&self, Parameters(request): Parameters<DockerBuildRequest>) -> String {
        let error_response = |message: String| -> String {
            serde_json::json!({
                "success": false,
                "image_id": "",
                "tag": &request.tag,
                "build_log": message,
            })
            .to_string()
        };

        if let Err(msg) = validate_path(&request.context, "context path") {
            return error_response(msg);
        }

        if !validate_tag(&request.tag) {
            return error_response(format!("Invalid tag: {}", request.tag));
        }

        if let Some(ref df) = request.dockerfile
            && let Err(msg) = validate_path(df, "dockerfile path")
        {
            return error_response(msg);
        }

        if let Some(ref args) = request.build_args
            && let Err(msg) = validate_build_args(args)
        {
            return error_response(msg);
        }

        execute_docker_build(
            &request.context,
            &request.tag,
            request.dockerfile.as_deref(),
            request.build_args.as_ref(),
        )
    }
}

#[tool_handler]
impl ServerHandler for DockerBuildTool {
    fn get_info(&self) -> ServerInfo {
        ServerInfo::new(ServerCapabilities::builder().enable_tools().build())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn call_docker_build(
        tool: &DockerBuildTool,
        context: &str,
        tag: &str,
        build_args: Option<HashMap<String, String>>,
        dockerfile: Option<&str>,
    ) -> String {
        tool.docker_build(Parameters(DockerBuildRequest {
            context: context.to_string(),
            tag: tag.to_string(),
            build_args,
            dockerfile: dockerfile.map(|s| s.to_string()),
        }))
    }

    #[test]
    fn rejects_context_with_path_traversal() {
        let tool = DockerBuildTool::new();
        let result = call_docker_build(&tool, "../../etc", "test:latest", None, None);
        let json: serde_json::Value = serde_json::from_str(&result).unwrap();
        assert_eq!(json["success"], false);
    }

    #[test]
    fn rejects_tag_with_shell_metacharacters() {
        let tool = DockerBuildTool::new();
        let result = call_docker_build(&tool, ".", "foo;rm -rf /", None, None);
        let json: serde_json::Value = serde_json::from_str(&result).unwrap();
        assert_eq!(json["success"], false);
    }

    #[test]
    fn rejects_invalid_build_arg_keys() {
        let tool = DockerBuildTool::new();
        let mut args = HashMap::new();
        args.insert(";bad".to_string(), "value".to_string());
        let result = call_docker_build(&tool, ".", "test:latest", Some(args), None);
        let json: serde_json::Value = serde_json::from_str(&result).unwrap();
        assert_eq!(json["success"], false);
    }

    #[test]
    fn validates_clean_inputs() {
        let tool = DockerBuildTool::new();
        let result = call_docker_build(&tool, ".", "test:latest", None, None);
        let json: serde_json::Value = serde_json::from_str(&result).unwrap();
        assert!(json.get("success").is_some());
        assert!(json.get("image_id").is_some());
        assert!(json.get("tag").is_some());
        assert!(json.get("build_log").is_some());
    }

    #[test]
    fn extract_image_id_legacy_format() {
        let output = "Step 1/3 : FROM alpine\nSuccessfully built abc123def456\n";
        assert_eq!(extract_image_id(output), "abc123def456");
    }

    #[test]
    fn extract_image_id_buildkit_format() {
        let output = "writing image sha256:abc123def456 done\n";
        assert_eq!(extract_image_id(output), "sha256:abc123def456");
    }

    #[test]
    fn extract_image_id_no_match() {
        let output = "some random docker output\n";
        assert_eq!(extract_image_id(output), "");
    }

    #[test]
    fn validate_tag_accepts_valid() {
        assert!(validate_tag("my-app:v1.0"));
        assert!(validate_tag("registry/my-app:latest"));
    }

    #[test]
    fn validate_tag_rejects_empty() {
        assert!(!validate_tag(""));
    }

    #[test]
    fn validate_tag_rejects_metacharacters() {
        assert!(!validate_tag("foo;bar"));
        assert!(!validate_tag("foo bar"));
    }

    #[test]
    fn has_shell_metacharacters_detects_them() {
        assert!(has_shell_metacharacters("hello;world"));
        assert!(has_shell_metacharacters("$(cmd)"));
        assert!(has_shell_metacharacters("foo\nbar"));
        assert!(!has_shell_metacharacters("clean-value_123"));
    }
}
