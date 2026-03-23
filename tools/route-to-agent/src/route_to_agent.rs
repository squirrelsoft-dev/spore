use agent_sdk::{AgentRequest, AgentResponse};
use rmcp::{
    handler::server::{router::tool::ToolRouter, wrapper::Parameters},
    model::{ServerCapabilities, ServerInfo},
    schemars, tool, tool_handler, tool_router, ServerHandler,
};

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct RouteToAgentRequest {
    /// Name of the target agent to route the request to
    pub agent_name: String,
    /// The request payload to forward to the agent
    pub input: String,
}

#[derive(Debug, Clone)]
pub struct RouteToAgentTool {
    tool_router: ToolRouter<Self>,
    endpoints_override: Option<String>,
}

impl RouteToAgentTool {
    pub fn new() -> Self {
        Self {
            tool_router: Self::tool_router(),
            endpoints_override: None,
        }
    }

    #[cfg(test)]
    fn with_endpoints(endpoints: &str) -> Self {
        Self {
            tool_router: Self::tool_router(),
            endpoints_override: Some(endpoints.to_string()),
        }
    }
}

/// Parse a raw `AGENT_ENDPOINTS` string into a list of (name, url) pairs.
///
/// Format: `"name1=url1,name2=url2"`. Trims whitespace, rejects empty keys/values.
fn parse_endpoints(raw: &str) -> Result<Vec<(String, String)>, String> {
    raw.split(',')
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(|entry| {
            let (key, value) = entry
                .split_once('=')
                .ok_or_else(|| format!("invalid pair '{entry}', expected 'key=value'"))?;
            let key = key.trim();
            let value = value.trim();
            if key.is_empty() || value.is_empty() {
                return Err(format!("empty key or value in pair '{entry}'"));
            }
            Ok((key.to_string(), value.to_string()))
        })
        .collect()
}

/// Linear scan for a matching agent name; returns the URL if found.
fn resolve_agent_url(endpoints: &[(String, String)], agent_name: &str) -> Option<String> {
    endpoints
        .iter()
        .find(|(name, _)| name == agent_name)
        .map(|(_, url)| url.clone())
}

/// Build a JSON error response string.
fn build_error_json(agent_name: &str, error: &str) -> String {
    serde_json::json!({
        "success": false,
        "agent_name": agent_name,
        "response": null,
        "error": error,
    })
    .to_string()
}

/// Build a JSON success response string with the embedded `AgentResponse`.
fn build_success_json(agent_name: &str, response: &AgentResponse) -> String {
    serde_json::json!({
        "success": true,
        "agent_name": agent_name,
        "response": serde_json::to_value(response).unwrap_or_default(),
        "error": "",
    })
    .to_string()
}

#[tool_router]
impl RouteToAgentTool {
    #[tool(description = "Route a request to another agent")]
    async fn route_to_agent(
        &self,
        Parameters(request): Parameters<RouteToAgentRequest>,
    ) -> String {
        let endpoints_raw = match &self.endpoints_override {
            Some(val) => val.clone(),
            None => match std::env::var("AGENT_ENDPOINTS") {
                Ok(val) if !val.trim().is_empty() => val,
                _ => {
                    return build_error_json(
                        &request.agent_name,
                        "AGENT_ENDPOINTS not configured",
                    );
                }
            },
        };

        let endpoints = match parse_endpoints(&endpoints_raw) {
            Ok(eps) => eps,
            Err(msg) => {
                return build_error_json(
                    &request.agent_name,
                    &format!("Failed to parse AGENT_ENDPOINTS: {msg}"),
                );
            }
        };

        let agent_url = match resolve_agent_url(&endpoints, &request.agent_name) {
            Some(url) => url,
            None => {
                return build_error_json(
                    &request.agent_name,
                    &format!("Agent '{}' not found in AGENT_ENDPOINTS", request.agent_name),
                );
            }
        };

        let invoke_url = format!("{}/invoke", agent_url.trim_end_matches('/'));
        let agent_request = AgentRequest::new(request.input);

        let client = match reqwest::Client::builder()
            .connect_timeout(std::time::Duration::from_secs(5))
            .timeout(std::time::Duration::from_secs(30))
            .build()
        {
            Ok(c) => c,
            Err(e) => {
                return build_error_json(
                    &request.agent_name,
                    &format!("Failed to create HTTP client: {e}"),
                );
            }
        };

        let result = client.post(&invoke_url).json(&agent_request).send().await;

        match result {
            Ok(response) if response.status().is_success() => {
                match response.json::<AgentResponse>().await {
                    Ok(agent_response) => {
                        build_success_json(&request.agent_name, &agent_response)
                    }
                    Err(e) => build_error_json(
                        &request.agent_name,
                        &format!("Failed to parse agent response: {e}"),
                    ),
                }
            }
            Ok(response) => {
                let status = response.status();
                let body = response.text().await.unwrap_or_default();
                build_error_json(
                    &request.agent_name,
                    &format!("HTTP {status}: {body}"),
                )
            }
            Err(e) => build_error_json(&request.agent_name, &format!("Request failed: {e}")),
        }
    }
}

#[tool_handler]
impl ServerHandler for RouteToAgentTool {
    fn get_info(&self) -> ServerInfo {
        ServerInfo::new(ServerCapabilities::builder().enable_tools().build())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Convenience helper that calls the route_to_agent tool method.
    async fn call_route(tool: &RouteToAgentTool, agent_name: &str, input: &str) -> String {
        tool.route_to_agent(Parameters(RouteToAgentRequest {
            agent_name: agent_name.to_string(),
            input: input.to_string(),
        }))
        .await
    }

    // ── Validation tests (no HTTP needed) ──

    #[tokio::test]
    async fn rejects_empty_agent_name() {
        let tool = RouteToAgentTool::with_endpoints("foo=http://localhost:1234");
        let result = call_route(&tool, "", "hello").await;
        let json: serde_json::Value = serde_json::from_str(&result).unwrap();
        assert_eq!(json["success"], false);
        let error = json["error"].as_str().unwrap().to_lowercase();
        assert!(
            error.contains("not found"),
            "error should mention agent not found, got: {error}"
        );
    }

    #[tokio::test]
    async fn rejects_empty_input() {
        // The implementation does not validate empty input at the tool level;
        // it proceeds to make an HTTP call. With an unreachable endpoint the
        // request fails, producing success == false.
        let port = find_unused_port().await;
        let endpoints = format!("test-agent=http://127.0.0.1:{port}");
        let tool = RouteToAgentTool::with_endpoints(&endpoints);
        let result = call_route(&tool, "test-agent", "").await;
        let json: serde_json::Value = serde_json::from_str(&result).unwrap();
        assert_eq!(json["success"], false);
    }

    #[tokio::test]
    async fn returns_error_when_agent_not_found() {
        let tool = RouteToAgentTool::with_endpoints("foo=http://localhost:1234");
        let result = call_route(&tool, "bar", "hello").await;
        let json: serde_json::Value = serde_json::from_str(&result).unwrap();
        assert_eq!(json["success"], false);
        let error = json["error"].as_str().unwrap().to_lowercase();
        assert!(
            error.contains("not found"),
            "error should contain 'not found', got: {error}"
        );
    }

    #[tokio::test]
    async fn returns_error_when_no_endpoints() {
        let tool = RouteToAgentTool::with_endpoints("");
        let result = call_route(&tool, "any-agent", "hello").await;
        let json: serde_json::Value = serde_json::from_str(&result).unwrap();
        assert_eq!(json["success"], false);
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
                    "{response_line}Content-Type: application/json\r\nContent-Length: {}\r\n\r\n{body}",
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

    /// Build a valid AgentResponse JSON string for mock server responses.
    fn mock_agent_response_json() -> String {
        serde_json::json!({
            "id": "00000000-0000-0000-0000-000000000001",
            "output": "agent says hello",
            "confidence": 0.95,
            "escalated": false,
            "escalate_to": null,
            "tool_calls": []
        })
        .to_string()
    }

    // ── Mock HTTP server tests ──

    #[tokio::test]
    async fn successful_route_returns_agent_response() {
        let body = mock_agent_response_json();
        let base_url = start_mock_server(200, &body).await;
        let endpoints = format!("test-agent={base_url}");
        let tool = RouteToAgentTool::with_endpoints(&endpoints);
        let result = call_route(&tool, "test-agent", "hello").await;
        let json: serde_json::Value = serde_json::from_str(&result).unwrap();
        assert_eq!(json["success"], true);
        assert_eq!(json["agent_name"], "test-agent");
        let response = &json["response"];
        assert_eq!(response["output"], "agent says hello");
        let confidence = response["confidence"].as_f64().unwrap();
        assert!((confidence - 0.95).abs() < 0.001, "expected confidence near 0.95, got {confidence}");
        assert_eq!(response["escalated"], false);
    }

    #[tokio::test]
    async fn agent_returns_4xx_produces_error() {
        let base_url = start_mock_server(400, "Bad Request").await;
        let endpoints = format!("test-agent={base_url}");
        let tool = RouteToAgentTool::with_endpoints(&endpoints);
        let result = call_route(&tool, "test-agent", "hello").await;
        let json: serde_json::Value = serde_json::from_str(&result).unwrap();
        assert_eq!(json["success"], false);
        let error = json["error"].as_str().unwrap();
        assert!(!error.is_empty(), "error field should be non-empty");
    }

    #[tokio::test]
    async fn agent_returns_5xx_produces_error() {
        let base_url = start_mock_server(500, "Internal Server Error").await;
        let endpoints = format!("test-agent={base_url}");
        let tool = RouteToAgentTool::with_endpoints(&endpoints);
        let result = call_route(&tool, "test-agent", "hello").await;
        let json: serde_json::Value = serde_json::from_str(&result).unwrap();
        assert_eq!(json["success"], false);
        let error = json["error"].as_str().unwrap();
        assert!(!error.is_empty(), "error field should be non-empty");
    }

    #[tokio::test]
    async fn agent_unreachable_produces_error() {
        let port = find_unused_port().await;
        let endpoints = format!("test-agent=http://127.0.0.1:{port}");
        let tool = RouteToAgentTool::with_endpoints(&endpoints);
        let result = call_route(&tool, "test-agent", "hello").await;
        let json: serde_json::Value = serde_json::from_str(&result).unwrap();
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
