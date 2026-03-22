use rmcp::model::CallToolRequestParams;

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn tools_list_returns_docker_build_tool() {
    let client = mcp_test_utils::spawn_mcp_client!(env!("CARGO_BIN_EXE_docker-build")).await;
    mcp_test_utils::assert_single_tool(
        &client,
        "docker_build",
        "docker build",
        &["context", "tag", "build_args", "dockerfile"],
    )
    .await;
    client.cancel().await.expect("failed to cancel client");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn tools_call_rejects_path_traversal() {
    let client = mcp_test_utils::spawn_mcp_client!(env!("CARGO_BIN_EXE_docker-build")).await;

    let params = CallToolRequestParams::new("docker_build").with_arguments(
        serde_json::json!({ "context": "../../etc", "tag": "test:latest" })
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
    let json: serde_json::Value = serde_json::from_str(&text.text).expect("should parse as JSON");
    assert_eq!(json["success"], false, "path traversal should be rejected");

    client.cancel().await.expect("failed to cancel client");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn tools_call_rejects_invalid_tag() {
    let client = mcp_test_utils::spawn_mcp_client!(env!("CARGO_BIN_EXE_docker-build")).await;

    let params = CallToolRequestParams::new("docker_build").with_arguments(
        serde_json::json!({ "context": ".", "tag": "test;evil" })
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
    let json: serde_json::Value = serde_json::from_str(&text.text).expect("should parse as JSON");
    assert_eq!(json["success"], false, "invalid tag should be rejected");

    client.cancel().await.expect("failed to cancel client");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn tools_call_returns_error_when_docker_unavailable() {
    let client = mcp_test_utils::spawn_mcp_client!(env!("CARGO_BIN_EXE_docker-build")).await;

    let params = CallToolRequestParams::new("docker_build").with_arguments(
        serde_json::json!({ "context": ".", "tag": "test:latest" })
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
    let json: serde_json::Value = serde_json::from_str(&text.text).expect("should parse as JSON");
    assert!(
        json.get("success").is_some(),
        "response should contain a success field"
    );
    if json["success"] == false {
        let build_log = json["build_log"].as_str().unwrap_or("");
        assert!(
            !build_log.is_empty(),
            "build_log should be non-empty when success is false"
        );
    }

    client.cancel().await.expect("failed to cancel client");
}
