use rmcp::model::CallToolRequestParams;

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn tools_list_returns_register_agent_tool() {
    let client =
        mcp_test_utils::spawn_mcp_client!(env!("CARGO_BIN_EXE_register-agent")).await;
    mcp_test_utils::assert_single_tool(
        &client,
        "register_agent",
        "Register an agent",
        &["name", "url", "description"],
    )
    .await;
    client.cancel().await.expect("failed to cancel client");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn tools_call_with_empty_name_returns_error() {
    let client =
        mcp_test_utils::spawn_mcp_client!(env!("CARGO_BIN_EXE_register-agent")).await;

    let params = CallToolRequestParams::new("register_agent").with_arguments(
        serde_json::json!({
            "name": "",
            "url": "http://example.com",
            "description": "A test agent"
        })
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
    assert_eq!(json["success"], false, "should fail for empty name");

    client.cancel().await.expect("failed to cancel client");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn tools_call_with_valid_inputs_returns_structured_json() {
    let client =
        mcp_test_utils::spawn_mcp_client!(env!("CARGO_BIN_EXE_register-agent")).await;

    let params = CallToolRequestParams::new("register_agent").with_arguments(
        serde_json::json!({
            "name": "test-agent",
            "url": "http://localhost:9999",
            "description": "A test agent"
        })
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
    assert!(
        json.get("success").is_some(),
        "response should have 'success' field"
    );
    assert!(
        json.get("agent_name").is_some(),
        "response should have 'agent_name' field"
    );
    assert!(
        json.get("registered_url").is_some(),
        "response should have 'registered_url' field"
    );

    client.cancel().await.expect("failed to cancel client");
}
