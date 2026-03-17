use rmcp::{
    model::CallToolRequestParams,
    service::RunningService,
    transport::TokioChildProcess,
    RoleClient, ServiceExt,
};
use tokio::process::Command;

/// Spawn the echo-tool binary as a child process and connect an MCP client.
async fn spawn_echo_client() -> RunningService<RoleClient, ()> {
    let transport = TokioChildProcess::new(Command::new(env!("CARGO_BIN_EXE_echo-tool")))
        .expect("failed to spawn echo-tool");

    ().serve(transport)
        .await
        .expect("failed to connect to echo-tool server")
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn tools_list_returns_echo_tool() {
    let client = spawn_echo_client().await;

    let tools_result = client.peer().list_tools(None).await.expect("list_tools");
    assert_eq!(tools_result.tools.len(), 1, "expected exactly 1 tool");
    assert_eq!(tools_result.tools[0].name, "echo");

    client.cancel().await.expect("failed to cancel client");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn tools_list_echo_has_correct_description() {
    let client = spawn_echo_client().await;

    let tools_result = client.peer().list_tools(None).await.expect("list_tools");
    let tool = &tools_result.tools[0];
    let description = tool
        .description
        .as_deref()
        .expect("echo tool should have a description");
    assert!(
        description.contains("Returns the input message unchanged"),
        "unexpected description: {description}"
    );

    client.cancel().await.expect("failed to cancel client");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn tools_list_echo_has_message_parameter() {
    let client = spawn_echo_client().await;

    let tools_result = client.peer().list_tools(None).await.expect("list_tools");
    let tool = &tools_result.tools[0];
    let properties = tool
        .input_schema
        .get("properties")
        .expect("input_schema should have properties");
    assert!(
        properties.get("message").is_some(),
        "input_schema properties should contain 'message'"
    );

    client.cancel().await.expect("failed to cancel client");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn tools_call_echo_returns_message() {
    let client = spawn_echo_client().await;

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
    let client = spawn_echo_client().await;

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
