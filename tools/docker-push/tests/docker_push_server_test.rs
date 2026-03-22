use rmcp::model::CallToolRequestParams;

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn tools_list_returns_docker_push_tool() {
    let client = mcp_test_utils::spawn_mcp_client!(env!("CARGO_BIN_EXE_docker-push")).await;
    mcp_test_utils::assert_single_tool(
        &client,
        "docker_push",
        "Push",
        &["image", "registry_url"],
    )
    .await;
    client.cancel().await.expect("failed to cancel client");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn tools_call_with_invalid_image_returns_error() {
    let client = mcp_test_utils::spawn_mcp_client!(env!("CARGO_BIN_EXE_docker-push")).await;

    let params = CallToolRequestParams::new("docker_push").with_arguments(
        serde_json::json!({ "image": "foo;bar" })
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
    assert_eq!(json["success"], false, "should fail for invalid image");
    assert!(
        json["push_log"]
            .as_str()
            .unwrap_or("")
            .contains("Invalid image reference"),
        "push_log should contain 'Invalid image reference', got: {}",
        json["push_log"]
    );

    client.cancel().await.expect("failed to cancel client");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn tools_call_with_valid_image_returns_structured_json() {
    let client = mcp_test_utils::spawn_mcp_client!(env!("CARGO_BIN_EXE_docker-push")).await;

    let params = CallToolRequestParams::new("docker_push").with_arguments(
        serde_json::json!({ "image": "nonexistent-image:latest" })
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
    assert!(json.get("success").is_some(), "response should have 'success' field");
    assert!(json.get("image").is_some(), "response should have 'image' field");
    assert!(json.get("digest").is_some(), "response should have 'digest' field");
    assert!(json.get("push_log").is_some(), "response should have 'push_log' field");

    client.cancel().await.expect("failed to cancel client");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn tools_call_with_empty_image_returns_error() {
    let client = mcp_test_utils::spawn_mcp_client!(env!("CARGO_BIN_EXE_docker-push")).await;

    let params = CallToolRequestParams::new("docker_push").with_arguments(
        serde_json::json!({ "image": "" })
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
    assert_eq!(json["success"], false, "should fail for empty image");

    client.cancel().await.expect("failed to cancel client");
}
