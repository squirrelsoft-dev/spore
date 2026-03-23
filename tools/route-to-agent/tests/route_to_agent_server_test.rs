use rmcp::model::CallToolRequestParams;

/// Spawn an MCP client whose child process has explicit env var overrides.
///
/// Uses `env_remove` to unset specific vars and `env` to set key-value pairs,
/// avoiding mutation of the parent process's global environment.
async fn spawn_client_with_env(
    envs: &[(&str, &str)],
    env_removes: &[&str],
) -> mcp_test_utils::RunningService<mcp_test_utils::RoleClient, ()> {
    let mut cmd = tokio::process::Command::new(env!("CARGO_BIN_EXE_route-to-agent"));
    for key in env_removes {
        cmd.env_remove(key);
    }
    for (key, value) in envs {
        cmd.env(key, value);
    }
    let transport =
        rmcp::transport::TokioChildProcess::new(cmd).expect("failed to spawn MCP server binary");
    <() as mcp_test_utils::ServiceExt<mcp_test_utils::RoleClient>>::serve((), transport)
        .await
        .expect("failed to connect to MCP server")
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn tools_list_returns_route_to_agent_tool() {
    let client = mcp_test_utils::spawn_mcp_client!(env!("CARGO_BIN_EXE_route-to-agent")).await;
    mcp_test_utils::assert_single_tool(
        &client,
        "route_to_agent",
        "Route",
        &["agent_name", "input"],
    )
    .await;
    client.cancel().await.expect("failed to cancel client");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn tools_call_returns_error_when_agent_not_found() {
    let client = spawn_client_with_env(
        &[("AGENT_ENDPOINTS", "foo=http://localhost:9999")],
        &[],
    )
    .await;

    let params = CallToolRequestParams::new("route_to_agent").with_arguments(
        serde_json::json!({ "agent_name": "nonexistent", "input": "hello" })
            .as_object()
            .unwrap()
            .clone(),
    );
    let result = client
        .peer()
        .call_tool(params)
        .await
        .expect("call_tool");

    let text = result
        .content
        .first()
        .expect("should have content")
        .as_text()
        .expect("first content should be text");
    let json: serde_json::Value =
        serde_json::from_str(&text.text).expect("should parse as JSON");

    assert_eq!(json["success"], false, "success should be false");
    let error = json["error"]
        .as_str()
        .expect("error should be a string");
    assert!(
        error.contains("not found"),
        "error should contain 'not found', got: \"{error}\""
    );
    assert_eq!(json["agent_name"], "nonexistent");

    client.cancel().await.expect("failed to cancel client");
}
