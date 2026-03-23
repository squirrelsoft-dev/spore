use rmcp::{
    handler::server::{router::tool::ToolRouter, wrapper::Parameters},
    model::{ServerCapabilities, ServerInfo},
    schemars, tool, tool_handler, tool_router, ServerHandler,
};

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct RegisterAgentRequest {
    /// Name of the agent to register
    pub name: String,
    /// URL where the agent can be reached
    pub url: String,
    /// Human-readable description of the agent
    pub description: String,
}

#[derive(Debug, Clone)]
pub struct RegisterAgentTool {
    tool_router: ToolRouter<Self>,
    orchestrator_url: Option<String>,
}

impl RegisterAgentTool {
    pub fn new() -> Self {
        Self {
            tool_router: Self::tool_router(),
            orchestrator_url: None,
        }
    }

    #[cfg(test)]
    fn with_orchestrator_url(url: &str) -> Self {
        Self {
            tool_router: Self::tool_router(),
            orchestrator_url: Some(url.to_string()),
        }
    }
}

fn validate_name(name: &str) -> Result<(), String> {
    if name.is_empty() {
        return Err("name must not be empty".to_string());
    }
    for ch in name.chars() {
        if !ch.is_ascii_alphanumeric() && !matches!(ch, '.' | '_' | '-') {
            return Err(format!("invalid character '{ch}' in agent name"));
        }
    }
    Ok(())
}

fn validate_url(url: &str) -> Result<(), String> {
    if url.is_empty() {
        return Err("url must not be empty".to_string());
    }
    if !url.starts_with("http://") && !url.starts_with("https://") {
        return Err("url must start with http:// or https://".to_string());
    }
    Ok(())
}

fn validate_description(description: &str) -> Result<(), String> {
    if description.is_empty() {
        return Err("description must not be empty".to_string());
    }
    Ok(())
}

fn build_error_json(name: &str, reason: &str) -> String {
    serde_json::json!({
        "success": false,
        "agent_name": name,
        "registered_url": "",
        "error": reason,
    })
    .to_string()
}

fn resolve_orchestrator_url() -> String {
    let url = std::env::var("ORCHESTRATOR_URL")
        .unwrap_or_else(|_| "http://orchestrator:8080".to_string());
    url.trim_end_matches('/').to_string()
}

#[tool_router]
impl RegisterAgentTool {
    #[tool(description = "Register an agent with the orchestrator")]
    async fn register_agent(
        &self,
        Parameters(request): Parameters<RegisterAgentRequest>,
    ) -> String {
        if let Err(reason) = validate_name(&request.name) {
            return build_error_json(&request.name, &format!("Invalid name: {reason}"));
        }
        if let Err(reason) = validate_url(&request.url) {
            return build_error_json(&request.name, &format!("Invalid url: {reason}"));
        }
        if let Err(reason) = validate_description(&request.description) {
            return build_error_json(
                &request.name,
                &format!("Invalid description: {reason}"),
            );
        }

        let base_url = self
            .orchestrator_url
            .clone()
            .unwrap_or_else(resolve_orchestrator_url);
        let register_url = format!("{base_url}/register");

        let payload = serde_json::json!({
            "name": request.name,
            "url": request.url,
            "description": request.description,
        });

        let client = match reqwest::Client::builder()
            .connect_timeout(std::time::Duration::from_secs(5))
            .timeout(std::time::Duration::from_secs(30))
            .build()
        {
            Ok(c) => c,
            Err(e) => {
                return build_error_json(
                    &request.name,
                    &format!("Failed to create HTTP client: {e}"),
                );
            }
        };

        let result = client
            .post(&register_url)
            .json(&payload)
            .send()
            .await;

        match result {
            Ok(response) if response.status().is_success() => {
                serde_json::json!({
                    "success": true,
                    "agent_name": request.name,
                    "registered_url": request.url,
                    "error": "",
                })
                .to_string()
            }
            Ok(response) => {
                let status = response.status();
                let body = response
                    .text()
                    .await
                    .unwrap_or_else(|_| String::new());
                build_error_json(
                    &request.name,
                    &format!("HTTP {status}: {body}"),
                )
            }
            Err(e) => build_error_json(&request.name, &format!("Request failed: {e}")),
        }
    }
}

#[tool_handler]
impl ServerHandler for RegisterAgentTool {
    fn get_info(&self) -> ServerInfo {
        ServerInfo::new(ServerCapabilities::builder().enable_tools().build())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Convenience helper that calls the register_agent tool method.
    async fn call_register_agent(
        tool: &RegisterAgentTool,
        name: &str,
        url: &str,
        description: &str,
    ) -> String {
        tool.register_agent(Parameters(RegisterAgentRequest {
            name: name.to_string(),
            url: url.to_string(),
            description: description.to_string(),
        }))
        .await
    }

    // ── Validation tests (spec: 9 required test cases) ──

    #[tokio::test]
    async fn rejects_empty_name() {
        let tool = RegisterAgentTool::new();
        let result = call_register_agent(&tool, "", "http://localhost:8080", "A test agent").await;
        let json: serde_json::Value = serde_json::from_str(&result).unwrap();
        assert_eq!(json["success"], false);
        assert!(json["error"].as_str().unwrap().to_lowercase().contains("name"));
    }

    #[tokio::test]
    async fn rejects_empty_url() {
        let tool = RegisterAgentTool::new();
        let result = call_register_agent(&tool, "my-agent", "", "A test agent").await;
        let json: serde_json::Value = serde_json::from_str(&result).unwrap();
        assert_eq!(json["success"], false);
        assert!(json["error"].as_str().unwrap().to_lowercase().contains("url"));
    }

    #[tokio::test]
    async fn rejects_empty_description() {
        let tool = RegisterAgentTool::new();
        let result = call_register_agent(&tool, "my-agent", "http://localhost:8080", "").await;
        let json: serde_json::Value = serde_json::from_str(&result).unwrap();
        assert_eq!(json["success"], false);
        assert!(json["error"]
            .as_str()
            .unwrap()
            .to_lowercase()
            .contains("description"));
    }

    #[tokio::test]
    async fn rejects_name_with_shell_metachar() {
        let tool = RegisterAgentTool::new();
        for bad_name in &["foo;bar", "foo|bar", "$(cmd)"] {
            let result =
                call_register_agent(&tool, bad_name, "http://localhost:8080", "A test agent")
                    .await;
            let json: serde_json::Value = serde_json::from_str(&result).unwrap();
            assert_eq!(
                json["success"], false,
                "expected rejection for name '{bad_name}'"
            );
        }
    }

    // ── Error JSON structure test ──

    #[tokio::test]
    async fn error_json_has_expected_structure() {
        let tool = RegisterAgentTool::new();
        let result = call_register_agent(&tool, "", "http://localhost:8080", "A test agent").await;
        let json: serde_json::Value = serde_json::from_str(&result).unwrap();
        assert_eq!(json["success"], false);
        assert!(json.get("agent_name").is_some(), "missing 'agent_name' field");
        assert!(
            json.get("registered_url").is_some(),
            "missing 'registered_url' field"
        );
        assert!(json.get("error").is_some(), "missing 'error' field");
        assert!(
            !json["error"].as_str().unwrap().is_empty(),
            "error should be non-empty"
        );
        assert_eq!(json["registered_url"], "");
    }

    // ── Mock HTTP server helpers ──

    /// Start a mock TCP server that responds with the given status and body.
    async fn start_mock_server(status: u16, body: &str) -> String {
        use tokio::io::AsyncWriteExt;
        use tokio::net::TcpListener;

        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let reason = match status {
            200 => "OK",
            400 => "Bad Request",
            500 => "Internal Server Error",
            _ => "Unknown",
        };
        let response_line = format!("HTTP/1.1 {status} {reason}\r\n");
        let body = body.to_string();

        tokio::spawn(async move {
            if let Ok((mut stream, _)) = listener.accept().await {
                use tokio::io::AsyncReadExt;
                let mut buf = vec![0u8; 4096];
                let _ = stream.read(&mut buf).await;
                let response = format!(
                    "{response_line}Content-Type: text/plain\r\nContent-Length: {}\r\n\r\n{body}",
                    body.len()
                );
                let _ = stream.write_all(response.as_bytes()).await;
                let _ = stream.shutdown().await;
            }
        });

        format!("http://127.0.0.1:{}", addr.port())
    }

    /// Find a free port with no listener (for the unreachable test).
    async fn find_unused_port() -> u16 {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = listener.local_addr().unwrap().port();
        drop(listener);
        port
    }

    /// Call register_agent against a specific orchestrator URL using the
    /// `with_orchestrator_url` constructor, avoiding env var mutation.
    async fn call_register_with_orchestrator(orchestrator_url: &str) -> serde_json::Value {
        let tool = RegisterAgentTool::with_orchestrator_url(orchestrator_url);
        let result =
            call_register_agent(&tool, "test-agent", "http://localhost:9999", "A test agent")
                .await;
        serde_json::from_str(&result).unwrap()
    }

    // ── Mock HTTP server tests ──

    #[tokio::test]
    async fn successful_registration_returns_correct_json() {
        let base_url = start_mock_server(200, "OK").await;
        let json = call_register_with_orchestrator(&base_url).await;
        assert_eq!(json["success"], true);
        assert_eq!(json["agent_name"], "test-agent");
        assert_eq!(json["registered_url"], "http://localhost:9999");
    }

    #[tokio::test]
    async fn orchestrator_4xx_produces_error_json() {
        let base_url = start_mock_server(400, "Bad Request").await;
        let json = call_register_with_orchestrator(&base_url).await;
        assert_eq!(json["success"], false);
        let error = json["error"].as_str().unwrap();
        assert!(!error.is_empty(), "error field should be non-empty");
    }

    #[tokio::test]
    async fn orchestrator_5xx_produces_error_json() {
        let base_url = start_mock_server(500, "Internal Server Error").await;
        let json = call_register_with_orchestrator(&base_url).await;
        assert_eq!(json["success"], false);
        let error = json["error"].as_str().unwrap();
        assert!(!error.is_empty(), "error field should be non-empty");
    }

    #[tokio::test]
    async fn orchestrator_unreachable_produces_error_json() {
        let port = find_unused_port().await;
        let base_url = format!("http://127.0.0.1:{port}");
        let json = call_register_with_orchestrator(&base_url).await;
        assert_eq!(json["success"], false);
        let error = json["error"].as_str().unwrap().to_lowercase();
        assert!(
            error.contains("connect")
                || error.contains("connection")
                || error.contains("refused")
                || error.contains("error sending request"),
            "error should mention connection failure, got: {error}"
        );
    }
}
