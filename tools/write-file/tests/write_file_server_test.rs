use rmcp::model::CallToolRequestParams;

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn tools_list_returns_single_write_file_tool() {
    let client = mcp_test_utils::spawn_mcp_client!(env!("CARGO_BIN_EXE_write-file")).await;

    mcp_test_utils::assert_single_tool(
        &client,
        "write_file",
        "Write content to a file",
        &["path", "content"],
    )
    .await;

    client.cancel().await.expect("failed to cancel client");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn tools_call_write_file_creates_file() {
    let client = mcp_test_utils::spawn_mcp_client!(env!("CARGO_BIN_EXE_write-file")).await;

    let temp_dir = std::env::temp_dir();
    let file_path = temp_dir.join(format!("spore_write_file_test_{}.txt", std::process::id()));
    let expected_content = "Hello from write_file integration test!";

    let params = CallToolRequestParams::new("write_file").with_arguments(
        serde_json::json!({
            "path": file_path.to_str().unwrap(),
            "content": expected_content,
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

    assert!(!result.content.is_empty(), "response content should not be empty");
    let text = result.content[0]
        .as_text()
        .expect("first content should be text");
    assert!(
        text.text.contains("Wrote"),
        "response should confirm write, got: {}",
        text.text
    );

    let actual_content = std::fs::read_to_string(&file_path)
        .expect("should be able to read back the written file");
    assert_eq!(
        actual_content, expected_content,
        "file content should match what was written"
    );

    let _ = std::fs::remove_file(&file_path);

    client.cancel().await.expect("failed to cancel client");
}
