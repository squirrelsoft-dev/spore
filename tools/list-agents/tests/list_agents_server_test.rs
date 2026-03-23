use rmcp::model::CallToolRequestParams;

/// Spawn an MCP client whose child process has explicit env var overrides.
///
/// Uses `env_remove` to unset specific vars and `env` to set key-value pairs,
/// avoiding mutation of the parent process's global environment.
async fn spawn_client_with_env(
    envs: &[(&str, &str)],
    env_removes: &[&str],
) -> mcp_test_utils::RunningService<mcp_test_utils::RoleClient, ()> {
    let mut cmd = tokio::process::Command::new(env!("CARGO_BIN_EXE_list-agents"));
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
async fn tools_list_returns_list_agents_tool() {
    let client = mcp_test_utils::spawn_mcp_client!(env!("CARGO_BIN_EXE_list-agents")).await;
    mcp_test_utils::assert_single_tool(&client, "list_agents", "agent", &["filter"]).await;
    client.cancel().await.expect("failed to cancel client");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn tools_call_returns_empty_when_no_agents() {
    // Remove env vars on the child process only, avoiding global mutation.
    let client =
        spawn_client_with_env(&[], &["AGENT_ENDPOINTS", "AGENT_DESCRIPTIONS"]).await;

    let params = CallToolRequestParams::new("list_agents");
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

    let agents = json["agents"].as_array().expect("agents should be an array");
    assert!(agents.is_empty(), "agents should be empty when no env vars set");

    client.cancel().await.expect("failed to cancel client");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn tools_call_returns_agents_from_env() {
    // Set env vars on the child process only, avoiding global mutation.
    let client = spawn_client_with_env(
        &[
            (
                "AGENT_ENDPOINTS",
                "foo=http://localhost:8080,bar=http://localhost:9090",
            ),
            (
                "AGENT_DESCRIPTIONS",
                "foo=A foo agent,bar=A bar agent",
            ),
        ],
        &[],
    )
    .await;

    let params = CallToolRequestParams::new("list_agents");
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

    let agents = json["agents"].as_array().expect("agents should be an array");
    assert_eq!(agents.len(), 2, "should have 2 agents");

    // Don't assume array order -- check membership
    let has_foo = agents.iter().any(|a| {
        a["name"] == "foo"
            && a["url"] == "http://localhost:8080"
            && a["description"] == "A foo agent"
    });
    let has_bar = agents.iter().any(|a| {
        a["name"] == "bar"
            && a["url"] == "http://localhost:9090"
            && a["description"] == "A bar agent"
    });
    assert!(has_foo, "should contain foo agent with correct fields");
    assert!(has_bar, "should contain bar agent with correct fields");

    client.cancel().await.expect("failed to cancel client");
}
