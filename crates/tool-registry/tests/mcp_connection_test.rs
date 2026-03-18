use std::borrow::Cow;
use std::net::SocketAddr;
use std::sync::Arc;

use rmcp::model::{
    CallToolRequestParams, CallToolResult, Content, ListToolsResult, ServerCapabilities, Tool,
};
use rmcp::service::{RequestContext, RoleServer};
use rmcp::{ErrorData as McpError, ServerHandler, ServiceExt};
use serde_json::json;
use tokio::net::TcpListener;
use tokio::task::JoinHandle;
use tool_registry::{RegistryError, ToolEntry, ToolRegistry};

// ---------------------------------------------------------------------------
// Mock MCP server
// ---------------------------------------------------------------------------

/// A minimal MCP server that advertises a configurable set of tools and
/// responds to `call_tool` for the "echo" tool by reflecting its arguments.
struct MockServer {
    tools: Vec<Tool>,
}

impl MockServer {
    fn with_echo_tool() -> Self {
        let schema = json_object_schema(json!({
            "type": "object",
            "properties": {
                "message": { "type": "string" }
            }
        }));

        Self {
            tools: vec![Tool::new("echo", "Echoes the input", Arc::new(schema))],
        }
    }

    fn with_tools(tools: Vec<Tool>) -> Self {
        Self { tools }
    }
}

impl ServerHandler for MockServer {
    fn get_info(&self) -> rmcp::model::InitializeResult {
        rmcp::model::InitializeResult {
            capabilities: ServerCapabilities::builder().enable_tools().build(),
            ..Default::default()
        }
    }

    fn list_tools(
        &self,
        _request: Option<rmcp::model::PaginatedRequestParams>,
        _context: RequestContext<RoleServer>,
    ) -> impl std::future::Future<Output = Result<ListToolsResult, McpError>> + Send + '_ {
        std::future::ready(Ok(ListToolsResult {
            tools: self.tools.clone(),
            next_cursor: None,
            meta: None,
        }))
    }

    fn call_tool(
        &self,
        request: CallToolRequestParams,
        _context: RequestContext<RoleServer>,
    ) -> impl std::future::Future<Output = Result<CallToolResult, McpError>> + Send + '_ {
        let result = if request.name == Cow::Borrowed("echo") {
            let msg = request
                .arguments
                .as_ref()
                .and_then(|args| args.get("message"))
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            Ok(CallToolResult::success(vec![Content::text(msg)]))
        } else {
            Err(McpError::invalid_request(
                format!("unknown tool: {}", request.name),
                None,
            ))
        };
        std::future::ready(result)
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Parse a JSON value into a `serde_json::Map` for use as a tool schema.
fn json_object_schema(
    value: serde_json::Value,
) -> serde_json::Map<String, serde_json::Value> {
    serde_json::from_value(value).expect("valid JSON object schema")
}

/// Start a TCP-based MCP server, returning the bound address and a handle
/// to the background task that accepts exactly one connection.
async fn start_tcp_server(server: MockServer) -> (SocketAddr, JoinHandle<()>) {
    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind TCP listener");
    let addr = listener.local_addr().expect("local addr");

    let handle = tokio::spawn(async move {
        let (stream, _) = listener.accept().await.expect("accept connection");
        let service = server.serve(stream).await.expect("serve connection");
        let _ = service.waiting().await;
    });

    (addr, handle)
}

/// Start a Unix-socket-based MCP server, returning a background task handle.
#[cfg(unix)]
async fn start_unix_server(
    server: MockServer,
    socket_path: std::path::PathBuf,
) -> JoinHandle<()> {
    let listener =
        tokio::net::UnixListener::bind(&socket_path).expect("bind Unix listener");

    tokio::spawn(async move {
        let (stream, _) = listener.accept().await.expect("accept unix connection");
        let service = server.serve(stream).await.expect("serve unix connection");
        let _ = service.waiting().await;
    })
}

/// Create a `ToolRegistry` containing a single entry with the given name
/// and endpoint.
fn make_registry_with_entry(name: &str, endpoint: &str) -> ToolRegistry {
    let registry = ToolRegistry::new();
    registry
        .register(ToolEntry {
            name: name.to_string(),
            version: "1.0.0".to_string(),
            endpoint: endpoint.to_string(),
            action_type: None,
            handle: None,
        })
        .expect("register entry");
    registry
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn connect_tcp_succeeds() {
    let (addr, server_handle) = start_tcp_server(MockServer::with_echo_tool()).await;
    let endpoint = format!("mcp://127.0.0.1:{}", addr.port());
    let registry = make_registry_with_entry("echo", &endpoint);

    let result = registry.connect("echo").await;
    assert!(result.is_ok(), "connect should succeed: {:?}", result.err());
    assert!(
        registry.get_handle("echo").is_some(),
        "handle should exist after connect"
    );

    drop(registry);
    let _ = server_handle.await;
}

#[cfg(unix)]
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn connect_unix_succeeds() {
    let tmp_dir = tempfile::tempdir().expect("create temp dir");
    let socket_path = tmp_dir.path().join("test.sock");

    let server_handle =
        start_unix_server(MockServer::with_echo_tool(), socket_path.clone()).await;

    let endpoint = format!("mcp+unix://{}", socket_path.display());
    let registry = make_registry_with_entry("echo", &endpoint);

    let result = registry.connect("echo").await;
    assert!(result.is_ok(), "connect should succeed: {:?}", result.err());
    assert!(
        registry.get_handle("echo").is_some(),
        "handle should exist after connect"
    );

    drop(registry);
    let _ = server_handle.await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn call_tool_through_handle() {
    let (addr, server_handle) = start_tcp_server(MockServer::with_echo_tool()).await;
    let endpoint = format!("mcp://127.0.0.1:{}", addr.port());
    let registry = make_registry_with_entry("echo", &endpoint);

    registry.connect("echo").await.expect("connect");

    let handle = registry.get_handle("echo").expect("handle exists");
    let args = json_object_schema(json!({
        "message": "hello world"
    }));

    let result = handle
        .peer()
        .call_tool(CallToolRequestParams {
            meta: None,
            name: Cow::Borrowed("echo"),
            arguments: Some(args),
            task: None,
        })
        .await
        .expect("call_tool");

    assert_eq!(result.is_error, Some(false));
    assert!(!result.content.is_empty(), "content should not be empty");
    let text = result.content[0]
        .as_text()
        .expect("first content should be text");
    assert_eq!(text.text, "hello world");

    // Drop handle before registry so all Arc refs to the client
    // RunningService are released and the server sees the disconnect.
    drop(handle);
    drop(registry);
    let _ = server_handle.await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn connect_to_invalid_endpoint_returns_error() {
    let registry = make_registry_with_entry("broken", "mcp://127.0.0.1:1");

    let result = registry.connect("broken").await;
    assert!(result.is_err(), "connect to invalid endpoint should fail");
    match result.unwrap_err() {
        RegistryError::ConnectionFailed { endpoint, .. } => {
            assert_eq!(endpoint, "mcp://127.0.0.1:1");
        }
        other => panic!("expected ConnectionFailed, got: {:?}", other),
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn list_tools_through_handle() {
    let empty_schema = json_object_schema(json!({
        "type": "object"
    }));

    let tools = vec![
        Tool::new("tool_a", "First tool", Arc::new(empty_schema.clone())),
        Tool::new("tool_b", "Second tool", Arc::new(empty_schema)),
    ];

    let (addr, server_handle) = start_tcp_server(MockServer::with_tools(tools)).await;
    let endpoint = format!("mcp://127.0.0.1:{}", addr.port());
    let registry = make_registry_with_entry("tool_a", &endpoint);

    registry.connect("tool_a").await.expect("connect");

    let handle = registry.get_handle("tool_a").expect("handle exists");
    let tools_result = handle.peer().list_tools(None).await.expect("list_tools");

    assert_eq!(
        tools_result.tools.len(),
        2,
        "server should advertise 2 tools"
    );

    let names: Vec<&str> = tools_result
        .tools
        .iter()
        .map(|t| t.name.as_ref())
        .collect();
    assert!(names.contains(&"tool_a"), "should contain tool_a");
    assert!(names.contains(&"tool_b"), "should contain tool_b");

    // Drop handle before registry so all Arc refs to the client
    // RunningService are released and the server sees the disconnect.
    drop(handle);
    drop(registry);
    let _ = server_handle.await;
}
