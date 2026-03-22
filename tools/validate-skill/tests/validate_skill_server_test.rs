use rmcp::{
    model::CallToolRequestParams,
    service::RunningService,
    transport::TokioChildProcess,
    RoleClient, ServiceExt,
};
use tokio::process::Command;

/// Spawn the validate-skill binary as a child process and connect an MCP client.
async fn spawn_validate_skill_client() -> RunningService<RoleClient, ()> {
    let transport =
        TokioChildProcess::new(Command::new(env!("CARGO_BIN_EXE_validate-skill")))
            .expect("failed to spawn validate-skill");

    ().serve(transport)
        .await
        .expect("failed to connect to validate-skill server")
}

fn valid_skill_content() -> String {
    r#"---
name: test-skill
version: "1.0.0"
description: A test skill
model:
  provider: openai
  name: gpt-4
  temperature: 0.7
tools:
  - read_file
  - write_file
constraints:
  confidence_threshold: 0.8
  max_turns: 5
  allowed_actions:
    - read
    - write
output:
  format: json
  schema:
    result: string
---
This is the preamble body."#
        .to_string()
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn tools_list_returns_validate_skill_tool() {
    let client = spawn_validate_skill_client().await;

    let tools_result = client.peer().list_tools(None).await.expect("list_tools");
    assert_eq!(tools_result.tools.len(), 1, "expected exactly 1 tool");
    assert_eq!(tools_result.tools[0].name, "validate_skill");

    client.cancel().await.expect("failed to cancel client");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn tools_list_has_correct_description() {
    let client = spawn_validate_skill_client().await;

    let tools_result = client.peer().list_tools(None).await.expect("list_tools");
    let tool = &tools_result.tools[0];
    let description = tool
        .description
        .as_deref()
        .expect("validate_skill tool should have a description");
    assert!(
        description.contains("Validate"),
        "unexpected description: {description}"
    );

    client.cancel().await.expect("failed to cancel client");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn tools_list_has_content_parameter() {
    let client = spawn_validate_skill_client().await;

    let tools_result = client.peer().list_tools(None).await.expect("list_tools");
    let tool = &tools_result.tools[0];
    let properties = tool
        .input_schema
        .get("properties")
        .expect("input_schema should have properties");
    assert!(
        properties.get("content").is_some(),
        "input_schema properties should contain 'content'"
    );

    client.cancel().await.expect("failed to cancel client");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn tools_call_with_valid_skill_returns_valid_true() {
    let client = spawn_validate_skill_client().await;

    let params = CallToolRequestParams::new("validate_skill").with_arguments(
        serde_json::json!({ "content": valid_skill_content() })
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
    let parsed: serde_json::Value =
        serde_json::from_str(&text.text).expect("response should be valid JSON");

    assert_eq!(parsed["valid"], true);
    assert!(
        parsed["errors"].as_array().unwrap().is_empty(),
        "errors should be empty for valid input"
    );
    assert!(
        parsed.get("manifest").is_some(),
        "response should contain 'manifest'"
    );
    assert_eq!(parsed["manifest"]["name"], "test-skill");
    assert_eq!(parsed["manifest"]["version"], "1.0.0");
    assert_eq!(parsed["manifest"]["description"], "A test skill");

    client.cancel().await.expect("failed to cancel client");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn tools_call_with_missing_frontmatter_returns_valid_false() {
    let client = spawn_validate_skill_client().await;

    let params = CallToolRequestParams::new("validate_skill").with_arguments(
        serde_json::json!({ "content": "no frontmatter here" })
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
    let parsed: serde_json::Value =
        serde_json::from_str(&text.text).expect("response should be valid JSON");

    assert_eq!(parsed["valid"], false);
    let errors = parsed["errors"]
        .as_array()
        .expect("errors should be an array");
    assert!(!errors.is_empty(), "errors should not be empty");

    client.cancel().await.expect("failed to cancel client");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn tools_call_with_invalid_yaml_returns_valid_false() {
    let client = spawn_validate_skill_client().await;

    let params = CallToolRequestParams::new("validate_skill").with_arguments(
        serde_json::json!({ "content": "---\nunknown_only: true\n---\nbody" })
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
    let parsed: serde_json::Value =
        serde_json::from_str(&text.text).expect("response should be valid JSON");

    assert_eq!(parsed["valid"], false);
    let errors = parsed["errors"]
        .as_array()
        .expect("errors should be an array");
    assert!(!errors.is_empty(), "errors should not be empty");

    client.cancel().await.expect("failed to cancel client");
}
