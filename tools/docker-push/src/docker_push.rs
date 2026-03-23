use rmcp::{
    handler::server::{router::tool::ToolRouter, wrapper::Parameters},
    model::{ServerCapabilities, ServerInfo},
    schemars, tool, tool_handler, tool_router, ServerHandler,
};

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct DockerPushRequest {
    /// Full image reference (e.g., ghcr.io/spore/spore-agent:0.1)
    pub image: String,
    /// Override registry URL; falls back to REGISTRY_URL env var
    pub registry_url: Option<String>,
}

#[derive(Debug, Clone)]
pub struct DockerPushTool {
    tool_router: ToolRouter<Self>,
}

impl DockerPushTool {
    pub fn new() -> Self {
        Self {
            tool_router: Self::tool_router(),
        }
    }
}

fn is_valid_ref_char(ch: char) -> bool {
    ch.is_ascii_alphanumeric() || matches!(ch, '.' | '-' | '_' | '/' | ':')
}

fn validate_image_ref(image: &str) -> Result<(), String> {
    if image.is_empty() {
        return Err("image must not be empty".to_string());
    }
    for ch in image.chars() {
        if !is_valid_ref_char(ch) {
            return Err(format!("invalid character '{ch}' in image reference"));
        }
    }
    Ok(())
}

fn validate_registry_url(url: &str) -> Result<(), String> {
    if url.is_empty() {
        return Err("registry URL must not be empty".to_string());
    }
    for ch in url.chars() {
        if !is_valid_ref_char(ch) {
            return Err(format!("invalid character '{ch}' in registry URL"));
        }
    }
    Ok(())
}

fn resolve_image_ref(image: &str, registry_url: Option<&str>) -> Result<String, String> {
    let effective_url = registry_url
        .map(|s| s.to_string())
        .or_else(|| std::env::var("REGISTRY_URL").ok());

    let Some(url) = effective_url else {
        return Ok(image.to_string());
    };

    validate_registry_url(&url)?;
    let url = url.trim_end_matches('/');
    if image.starts_with(url)
        && (image.len() == url.len() || image.as_bytes().get(url.len()) == Some(&b'/'))
    {
        Ok(image.to_string())
    } else {
        Ok(format!("{url}/{image}"))
    }
}

fn extract_digest(output: &str) -> String {
    for line in output.lines() {
        if let Some(pos) = line.find("digest: sha256:") {
            let digest_part = &line[pos + "digest: ".len()..];
            if let Some(digest) = digest_part.split_whitespace().next() {
                if digest.starts_with("sha256:") && digest.len() > "sha256:".len() {
                    return digest.to_string();
                }
            }
        }
    }
    String::new()
}

fn build_error_json(image: &str, reason: &str) -> String {
    serde_json::json!({
        "success": false,
        "image": image,
        "digest": "",
        "push_log": reason,
    })
    .to_string()
}

#[tool_router]
impl DockerPushTool {
    #[tool(description = "Push a tagged Docker image to a container registry")]
    fn docker_push(&self, Parameters(request): Parameters<DockerPushRequest>) -> String {
        if let Err(reason) = validate_image_ref(&request.image) {
            return build_error_json(
                &request.image,
                &format!("Invalid image reference: {reason}"),
            );
        }

        let final_ref = match resolve_image_ref(&request.image, request.registry_url.as_deref()) {
            Ok(r) => r,
            Err(reason) => {
                return build_error_json(
                    &request.image,
                    &format!("Invalid registry URL: {reason}"),
                );
            }
        };

        match std::process::Command::new("docker")
            .args(["push", &final_ref])
            .output()
        {
            Ok(output) => {
                let combined = format!(
                    "{}{}",
                    String::from_utf8_lossy(&output.stdout),
                    String::from_utf8_lossy(&output.stderr),
                );
                let digest = extract_digest(&combined);
                serde_json::json!({
                    "success": output.status.success(),
                    "image": final_ref,
                    "digest": digest,
                    "push_log": combined,
                })
                .to_string()
            }
            Err(e) => build_error_json(&final_ref, &format!("Failed to execute docker: {e}")),
        }
    }
}

#[tool_handler]
impl ServerHandler for DockerPushTool {
    fn get_info(&self) -> ServerInfo {
        ServerInfo::new(ServerCapabilities::builder().enable_tools().build())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn call_docker_push(tool: &DockerPushTool, image: &str, registry_url: Option<&str>) -> String {
        tool.docker_push(Parameters(DockerPushRequest {
            image: image.to_string(),
            registry_url: registry_url.map(|s| s.to_string()),
        }))
    }

    #[test]
    fn rejects_empty_image() {
        let tool = DockerPushTool::new();
        let result = call_docker_push(&tool, "", None);
        let json: serde_json::Value = serde_json::from_str(&result).unwrap();
        assert_eq!(json["success"], false);
        assert!(json["push_log"]
            .as_str()
            .unwrap()
            .contains("Invalid image reference"));
    }

    #[test]
    fn rejects_image_with_shell_metachar() {
        let tool = DockerPushTool::new();
        let result = call_docker_push(&tool, "foo;bar", None);
        let json: serde_json::Value = serde_json::from_str(&result).unwrap();
        assert_eq!(json["success"], false);
        assert!(json["push_log"]
            .as_str()
            .unwrap()
            .contains("Invalid image reference"));
    }

    #[test]
    fn rejects_image_with_pipe() {
        let tool = DockerPushTool::new();
        let result = call_docker_push(&tool, "foo|bar", None);
        let json: serde_json::Value = serde_json::from_str(&result).unwrap();
        assert_eq!(json["success"], false);
        assert!(json["push_log"]
            .as_str()
            .unwrap()
            .contains("Invalid image reference"));
    }

    #[test]
    fn accepts_valid_image_reference() {
        let tool = DockerPushTool::new();
        let result = call_docker_push(&tool, "ghcr.io/spore/spore-agent:0.1", None);
        let json: serde_json::Value = serde_json::from_str(&result).unwrap();
        assert!(json.get("success").is_some());
        assert!(json.get("image").is_some());
        assert!(json.get("digest").is_some());
        assert!(json.get("push_log").is_some());
    }

    #[test]
    fn registry_url_is_prepended() {
        let result = resolve_image_ref("spore-agent:0.1", Some("ghcr.io/spore")).unwrap();
        assert_eq!(result, "ghcr.io/spore/spore-agent:0.1");

        let result =
            resolve_image_ref("ghcr.io/spore/spore-agent:0.1", Some("ghcr.io/spore")).unwrap();
        assert_eq!(result, "ghcr.io/spore/spore-agent:0.1");
    }

    #[test]
    fn registry_url_partial_prefix_does_not_match() {
        let result = resolve_image_ref("myreg-app/image:latest", Some("myreg")).unwrap();
        assert_eq!(result, "myreg/myreg-app/image:latest");
    }

    #[test]
    fn rejects_invalid_registry_url() {
        let tool = DockerPushTool::new();
        let result = call_docker_push(&tool, "my-image:latest", Some("evil;registry"));
        let json: serde_json::Value = serde_json::from_str(&result).unwrap();
        assert_eq!(json["success"], false);
        assert!(json["push_log"]
            .as_str()
            .unwrap()
            .contains("Invalid registry URL"));
    }

    #[test]
    fn digest_extraction_from_output() {
        let output = "latest: digest: sha256:abc123def456 size: 1234";
        assert_eq!(extract_digest(output), "sha256:abc123def456");

        let output = "Pushing image...\nDone.";
        assert_eq!(extract_digest(output), "");
    }
}
