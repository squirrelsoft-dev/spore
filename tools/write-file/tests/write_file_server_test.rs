use rmcp::{
    model::CallToolRequestParams,
    service::RunningService,
    transport::TokioChildProcess,
    RoleClient, ServiceExt,
};
use tokio::process::Command;

/// Spawn the write-file binary as a child process and connect an MCP client.
async fn spawn_write_file_client() -> RunningService<RoleClient, ()> {
    let transport = TokioChildProcess::new(Command::new(env!("CARGO_BIN_EXE_write-file")))
        .expect("failed to spawn write-file");

    ().serve(transport)
        .await
        .expect("failed to connect to write-file server")
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn tools_list_returns_write_file_tool() {
    let client = spawn_write_file_client().await;

    let tools_result = client.peer().list_tools(None).await.expect("list_tools");
    assert_eq!(tools_result.tools.len(), 1, "expected exactly 1 tool");
    assert_eq!(tools_result.tools[0].name, "write_file");

    client.cancel().await.expect("failed to cancel client");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn tools_list_write_file_has_correct_description() {
    let client = spawn_write_file_client().await;

    let tools_result = client.peer().list_tools(None).await.expect("list_tools");
    let tool = &tools_result.tools[0];
    let description = tool
        .description
        .as_deref()
        .expect("write_file tool should have a description");
    assert!(
        description.contains("Write content to a file"),
        "unexpected description: {description}"
    );

    client.cancel().await.expect("failed to cancel client");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn tools_list_write_file_has_path_and_content_parameters() {
    let client = spawn_write_file_client().await;

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
    assert!(
        properties.get("content").is_some(),
        "input_schema properties should contain 'content'"
    );

    client.cancel().await.expect("failed to cancel client");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn tools_call_write_file_creates_file() {
    let client = spawn_write_file_client().await;

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
