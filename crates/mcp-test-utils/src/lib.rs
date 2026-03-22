pub use rmcp::service::RunningService;
pub use rmcp::RoleClient;
pub use rmcp::ServiceExt;

/// Spawn an MCP server binary as a child process and connect an MCP client.
///
/// Accepts a binary path expression (typically `env!("CARGO_BIN_EXE_...")`).
/// Returns a `RunningService<RoleClient, ()>` inside an async block.
///
/// # Example
///
/// ```ignore
/// let client = spawn_mcp_client!(env!("CARGO_BIN_EXE_echo-tool")).await;
/// ```
#[macro_export]
macro_rules! spawn_mcp_client {
    ($bin_path:expr) => {
        async {
            let transport = ::rmcp::transport::TokioChildProcess::new(
                ::tokio::process::Command::new($bin_path),
            )
            .expect("failed to spawn MCP server binary");

            <() as ::rmcp::ServiceExt<::rmcp::RoleClient>>::serve((), transport)
                .await
                .expect("failed to connect to MCP server")
        }
    };
}

/// Assert that a connected MCP client exposes exactly one tool with the
/// expected name, a description containing the given substring, and
/// input-schema properties matching every entry in `expected_params`.
pub async fn assert_single_tool(
    client: &RunningService<RoleClient, ()>,
    expected_name: &str,
    description_contains: &str,
    expected_params: &[&str],
) {
    let tools_result = client
        .peer()
        .list_tools(None)
        .await
        .expect("list_tools");
    let tools = &tools_result.tools;

    assert_eq!(tools.len(), 1, "expected exactly 1 tool, found {}", tools.len());
    assert_eq!(tools[0].name, expected_name);

    let description = tools[0]
        .description
        .as_deref()
        .expect("tool should have a description");
    assert!(
        description.contains(description_contains),
        "expected description to contain \"{description_contains}\", got: \"{description}\""
    );

    let properties = tools[0]
        .input_schema
        .get("properties")
        .expect("input_schema should have properties");

    for param in expected_params {
        assert!(
            properties.get(*param).is_some(),
            "input_schema properties should contain '{param}'"
        );
    }
}

/// Create a unique temporary directory for a test.
///
/// Creates `<temp_dir>/spore_tests/<test_name>/<pid>`, removing any prior
/// contents. Returns the path to the created directory.
pub fn unique_temp_dir(test_name: &str) -> std::path::PathBuf {
    let dir = std::env::temp_dir()
        .join("spore_tests")
        .join(test_name)
        .join(format!("{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("failed to create temp dir");
    dir
}

/// Returns canonical valid skill YAML frontmatter for use in tests.
///
/// The returned string contains a complete skill definition with `---`
/// delimiters and a trailing preamble body line, suitable for passing to
/// `skill_loader::parse_content` and related functions.
pub fn valid_skill_content() -> String {
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
