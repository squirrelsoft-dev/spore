use mcp_test_utils::{assert_single_tool, spawn_mcp_client, valid_skill_content};
use rmcp::model::CallToolRequestParams;

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn tools_list_validates_single_tool() {
    let client = spawn_mcp_client!(env!("CARGO_BIN_EXE_validate-skill")).await;

    assert_single_tool(&client, "validate_skill", "Validate", &["content"]).await;

    client.cancel().await.expect("failed to cancel client");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn tools_call_with_valid_skill_returns_valid_true() {
    let client = spawn_mcp_client!(env!("CARGO_BIN_EXE_validate-skill")).await;

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
    let client = spawn_mcp_client!(env!("CARGO_BIN_EXE_validate-skill")).await;

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
    let client = spawn_mcp_client!(env!("CARGO_BIN_EXE_validate-skill")).await;

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
