use rmcp::model::CallToolRequestParams;

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn tools_list_returns_cargo_build_tool() {
    let client = mcp_test_utils::spawn_mcp_client!(env!("CARGO_BIN_EXE_cargo-build")).await;
    mcp_test_utils::assert_single_tool(&client, "cargo_build", "cargo build", &["package", "release"]).await;
    client.cancel().await.expect("failed to cancel client");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn tools_call_builds_echo_tool_successfully() {
    let client = mcp_test_utils::spawn_mcp_client!(env!("CARGO_BIN_EXE_cargo-build")).await;

    let params = CallToolRequestParams::new("cargo_build").with_arguments(
        serde_json::json!({ "package": "echo-tool" })
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
    assert_eq!(json["success"], true, "build should succeed");
    assert_eq!(json["exit_code"], 0, "exit code should be 0");

    client.cancel().await.expect("failed to cancel client");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn tools_call_returns_error_for_nonexistent_package() {
    let client = mcp_test_utils::spawn_mcp_client!(env!("CARGO_BIN_EXE_cargo-build")).await;

    let params = CallToolRequestParams::new("cargo_build").with_arguments(
        serde_json::json!({ "package": "nonexistent-package-xyz" })
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
    assert_eq!(json["success"], false, "build should fail");
    assert!(
        !json["stderr"].as_str().unwrap_or("").is_empty(),
        "stderr should be non-empty"
    );

    client.cancel().await.expect("failed to cancel client");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn tools_call_rejects_invalid_package_name() {
    let client = mcp_test_utils::spawn_mcp_client!(env!("CARGO_BIN_EXE_cargo-build")).await;

    let params = CallToolRequestParams::new("cargo_build").with_arguments(
        serde_json::json!({ "package": "foo;bar" })
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
    assert_eq!(json["success"], false, "build should fail for invalid name");
    assert!(
        json["stderr"]
            .as_str()
            .unwrap_or("")
            .contains("Invalid package name"),
        "stderr should contain 'Invalid package name', got: {}",
        json["stderr"]
    );

    client.cancel().await.expect("failed to cancel client");
}
