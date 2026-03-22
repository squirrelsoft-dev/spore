use rmcp::{
    model::CallToolRequestParams,
    service::RunningService,
    transport::TokioChildProcess,
    RoleClient, ServiceExt,
};
use tokio::process::Command;

/// Spawn the read-file binary as a child process and connect an MCP client.
async fn spawn_read_file_client() -> RunningService<RoleClient, ()> {
    let transport = TokioChildProcess::new(Command::new(env!("CARGO_BIN_EXE_read-file")))
        .expect("failed to spawn read-file");

    ().serve(transport)
        .await
        .expect("failed to connect to read-file server")
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn tools_list_returns_read_file_tool() {
    let client = spawn_read_file_client().await;

    let tools_result = client.peer().list_tools(None).await.expect("list_tools");
    assert_eq!(tools_result.tools.len(), 1, "expected exactly 1 tool");
    assert_eq!(
        tools_result.tools[0].name, "read_file",
        "expected tool named 'read_file'"
    );

    client.cancel().await.expect("failed to cancel client");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn tools_list_read_file_has_correct_description() {
    let client = spawn_read_file_client().await;

    let tools_result = client.peer().list_tools(None).await.expect("list_tools");
    let tool = &tools_result.tools[0];
    let description = tool
        .description
        .as_deref()
        .expect("read_file tool should have a description");
    assert!(
        description.contains("Read the contents of a file"),
        "unexpected description: {description}"
    );

    client.cancel().await.expect("failed to cancel client");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn tools_list_read_file_has_path_parameter() {
    let client = spawn_read_file_client().await;

    let tools_result = client.peer().list_tools(None).await.expect("list_tools");
    let tool = &tools_result.tools[0];
    let properties = tool
        .input_schema
        .get("properties")
        .expect("input_schema should have properties");
    assert!(
        properties.get("path").is_some(),
        "input_schema properties should contain 'path'"
    );

    client.cancel().await.expect("failed to cancel client");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn tools_call_read_file_returns_content() {
    let client = spawn_read_file_client().await;

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
    let client = spawn_read_file_client().await;

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
