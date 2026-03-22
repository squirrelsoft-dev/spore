use rmcp::model::CallToolRequestParams;

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn tools_list_validates_echo_tool() {
    let client = mcp_test_utils::spawn_mcp_client!(env!("CARGO_BIN_EXE_echo-tool")).await;
    mcp_test_utils::assert_single_tool(&client, "echo", "Returns the input message unchanged", &["message"]).await;
    client.cancel().await.expect("failed to cancel client");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn tools_call_echo_returns_message() {
    let client = mcp_test_utils::spawn_mcp_client!(env!("CARGO_BIN_EXE_echo-tool")).await;

    let params = CallToolRequestParams::new("echo").with_arguments(
        serde_json::json!({ "message": "hello" })
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
        text.text.contains("hello"),
        "response should contain 'hello', got: {}",
        text.text
    );

    client.cancel().await.expect("failed to cancel client");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn tools_call_echo_preserves_unicode() {
    let client = mcp_test_utils::spawn_mcp_client!(env!("CARGO_BIN_EXE_echo-tool")).await;

    let unicode_message = "Hej verden! \u{1F30D} \u{00E9}\u{00E8}\u{00EA} \u{4F60}\u{597D}";
    let params = CallToolRequestParams::new("echo").with_arguments(
        serde_json::json!({ "message": unicode_message })
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
    assert_eq!(
        text.text, unicode_message,
        "unicode message should be preserved exactly"
    );

    client.cancel().await.expect("failed to cancel client");
}
