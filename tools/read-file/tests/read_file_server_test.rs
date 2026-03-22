use rmcp::model::CallToolRequestParams;

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn tools_list_returns_read_file_tool_with_correct_schema() {
    let client = mcp_test_utils::spawn_mcp_client!(env!("CARGO_BIN_EXE_read-file")).await;

    mcp_test_utils::assert_single_tool(&client, "read_file", "Read", &["path"]).await;

    client.cancel().await.expect("failed to cancel client");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn tools_call_read_file_returns_content() {
    let client = mcp_test_utils::spawn_mcp_client!(env!("CARGO_BIN_EXE_read-file")).await;

    let temp_path = std::env::temp_dir().join("read_file_integration_test_content.txt");
    let expected_content = "integration test file content";
    std::fs::write(&temp_path, expected_content).expect("failed to write temp file");
    let path_str = temp_path.to_string_lossy().to_string();

    let params = CallToolRequestParams::new("read_file").with_arguments(
        serde_json::json!({ "path": path_str })
            .as_object()
            .unwrap()
            .clone(),
    );
    let result = client
        .peer()
        .call_tool(params)
        .await
        .expect("call_tool");

    assert!(!result.content.is_empty(), "response content should not be empty");
    let text = result.content[0]
        .as_text()
        .expect("first content should be text");
    assert!(
        text.text.contains(expected_content),
        "response should contain file contents, got: {}",
        text.text
    );

    client.cancel().await.expect("failed to cancel client");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn tools_call_read_file_returns_error_for_missing_file() {
    let client = mcp_test_utils::spawn_mcp_client!(env!("CARGO_BIN_EXE_read-file")).await;

    let nonexistent_path = std::env::temp_dir()
        .join("read_file_integration_nonexistent_xyz987.txt")
        .to_string_lossy()
        .to_string();

    let params = CallToolRequestParams::new("read_file").with_arguments(
        serde_json::json!({ "path": nonexistent_path })
            .as_object()
            .unwrap()
            .clone(),
    );
    let result = client
        .peer()
        .call_tool(params)
        .await
        .expect("call_tool");

    assert!(!result.content.is_empty(), "response content should not be empty");
    let text = result.content[0]
        .as_text()
        .expect("first content should be text");
    assert!(
        text.text.contains("Error"),
        "response should contain 'Error' for missing file, got: {}",
        text.text
    );

    client.cancel().await.expect("failed to cancel client");
}
