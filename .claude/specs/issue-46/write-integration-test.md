# Spec: Write integration test for validate-skill tool

> From: .claude/tasks/issue-46.md

## Objective

Create integration tests for the `validate-skill` MCP tool in `tools/validate-skill/tests/validate_skill_server_test.rs`, following the established pattern in `tools/echo-tool/tests/echo_server_test.rs`. The tests exercise the tool's MCP interface end-to-end by spawning the binary as a child process and communicating over the MCP protocol. Six tests cover tool listing metadata and tool invocation with valid, missing-frontmatter, and invalid-YAML inputs.

## Current State

- The `validate-skill` crate does not exist yet. Per the task breakdown (`.claude/tasks/issue-46.md`), this task is blocked by "Write `main.rs`" and "Implement `ValidateSkillTool` struct and handler".
- The `echo-tool` integration tests (`tools/echo-tool/tests/echo_server_test.rs`) provide the reference pattern: spawn the tool binary via `TokioChildProcess`, connect an MCP client with `ServiceExt::serve`, and exercise `list_tools` / `call_tool` endpoints.
- The `ValidateSkillTool` exposes a single MCP tool named `validate_skill` that accepts a `content: String` parameter (the full skill file content with YAML frontmatter) and returns structured JSON: `{ "valid": bool, "errors": [...], "manifest": {...} }`.
- A well-formed skill file requires YAML frontmatter with fields: `name`, `version`, `description`, `model` (with `provider` and `name`), `tools`, `constraints` (with `confidence_threshold`, `max_turns`), and `output` (with `format`), plus a non-empty body (preamble).

## Requirements

1. **File location:** `tools/validate-skill/tests/validate_skill_server_test.rs`

2. **Helper function:** Define `spawn_validate_skill_client` that mirrors `spawn_echo_client`:
   - Use `TokioChildProcess::new(Command::new(env!("CARGO_BIN_EXE_validate-skill")))` to spawn the binary.
   - Connect and return `RunningService<RoleClient, ()>`.

3. **All test functions** use `#[tokio::test(flavor = "multi_thread", worker_threads = 2)]`.

4. **Every test** must call `client.cancel().await.expect("failed to cancel client")` at the end, matching the echo-tool teardown pattern.

5. **Six test cases:**

   | # | Test name | Action | Assertion |
   |---|-----------|--------|-----------|
   | 1 | `tools_list_returns_validate_skill_tool` | Call `list_tools` | `tools.len() == 1` and `tools[0].name == "validate_skill"` |
   | 2 | `tools_list_has_correct_description` | Call `list_tools`, read description | Description contains the substring `"Validate"` |
   | 3 | `tools_list_has_content_parameter` | Call `list_tools`, inspect `input_schema` | `input_schema.properties` contains the key `"content"` |
   | 4 | `tools_call_with_valid_skill_returns_valid_true` | Call `call_tool` with a well-formed skill file string | Parse response text as JSON; assert `valid == true`, `errors` is empty, and `manifest` object is present with expected fields (`name`, `version`, `description`) |
   | 5 | `tools_call_with_missing_frontmatter_returns_valid_false` | Call `call_tool` with content lacking `---` delimiters (e.g., `"no frontmatter here"`) | Parse response text as JSON; assert `valid == false` and `errors` array is non-empty |
   | 6 | `tools_call_with_invalid_yaml_returns_valid_false` | Call `call_tool` with content that has `---` delimiters but malformed YAML between them (e.g., `"---\n: invalid: yaml: [broken\n---\nbody"`) | Parse response text as JSON; assert `valid == false` and `errors` array is non-empty |

6. **Imports:** Use the same crates as the echo-tool tests: `rmcp::{model::CallToolRequestParams, service::RunningService, transport::TokioChildProcess, RoleClient, ServiceExt}`, `tokio::process::Command`, and `serde_json` for parsing response JSON.

## Implementation Details

### Helper function

```rust
async fn spawn_validate_skill_client() -> RunningService<RoleClient, ()> {
    let transport = TokioChildProcess::new(
        Command::new(env!("CARGO_BIN_EXE_validate-skill"))
    ).expect("failed to spawn validate-skill");

    ().serve(transport)
        .await
        .expect("failed to connect to validate-skill server")
}
```

### Valid skill file fixture (for test 4)

Construct a well-formed skill file string inline as a constant or local variable. It must include all required frontmatter fields and a non-empty body. Example:

```text
---
name: test-skill
version: "1.0"
description: A test skill
model:
  provider: anthropic
  name: claude-3
tools:
  - echo
constraints:
  confidence_threshold: 0.8
  max_turns: 5
output:
  format: text
---
You are a test skill that echoes messages.
```

The exact field values do not matter as long as they pass the `SkillFrontmatter` deserialization and `validate()` checks (non-empty strings, `confidence_threshold` between 0.0 and 1.0, `max_turns > 0`, `format` in `ALLOWED_OUTPUT_FORMATS`).

### Calling the tool (tests 4-6)

Follow the echo-tool `call_tool` pattern:

```rust
let params = CallToolRequestParams::new("validate_skill").with_arguments(
    serde_json::json!({ "content": SKILL_CONTENT })
        .as_object()
        .unwrap()
        .clone(),
);
let result = client.peer().call_tool(params).await.expect("call_tool");
```

Then extract the text response and parse it as JSON:

```rust
let text = result.content[0].as_text().expect("first content should be text");
let json: serde_json::Value = serde_json::from_str(&text.text).expect("response should be valid JSON");
```

### Assertions for valid response (test 4)

```rust
assert_eq!(json["valid"], true);
assert!(json["errors"].as_array().unwrap().is_empty());
assert!(json["manifest"].is_object());
assert_eq!(json["manifest"]["name"], "test-skill");
```

### Assertions for invalid responses (tests 5, 6)

```rust
assert_eq!(json["valid"], false);
assert!(!json["errors"].as_array().unwrap().is_empty());
```

### No other files created or modified

This task only creates the single test file. It does not modify `Cargo.toml` or any other files (the test file is auto-discovered from the `tests/` directory).

## Dependencies

- **Blocked by:**
  - "Write `main.rs`" -- the binary must exist to be spawned by `env!("CARGO_BIN_EXE_validate-skill")`.
  - "Implement `ValidateSkillTool` struct and handler" -- the tool logic must be implemented for tests to pass.
  - "Create `tools/validate-skill/Cargo.toml`" -- dev-dependencies for integration tests must be declared.
- **Blocking:**
  - "Run verification suite" -- the verification step depends on all tests existing and passing.

## Risks & Edge Cases

1. **Binary name mismatch:** The `env!("CARGO_BIN_EXE_validate-skill")` macro resolves at compile time based on the package name in `Cargo.toml`. If the binary target name differs from `validate-skill`, compilation will fail. Confirm the binary name matches.

2. **Tool name underscore vs hyphen:** The MCP tool name is `validate_skill` (underscore), while the binary/package name is `validate-skill` (hyphen). Tests must assert the tool name with an underscore.

3. **Valid skill fixture drift:** If the `SkillFrontmatter` struct gains new required fields, the inline fixture in test 4 will fail to deserialize. The fixture should include all currently required fields. This is acceptable -- the test will correctly catch schema changes.

4. **ALLOWED_OUTPUT_FORMATS:** The `output.format` value in the fixture must be one of the allowed formats. Use `"text"` which is the most basic format. If `"text"` is not in `ALLOWED_OUTPUT_FORMATS`, check the `agent-sdk` crate for the valid values and update the fixture accordingly.

5. **Response structure assumption:** Tests assume the tool returns a JSON string in the text content of the MCP response. If the tool implementation changes the response format, tests will need updating.

6. **AllToolsExist stub:** The validate-skill tool uses `AllToolsExist`, so tool names in the fixture do not need to correspond to real registered tools. Any non-empty tool name will pass validation.

## Verification

1. `cargo test -p validate-skill` compiles and all 6 test functions pass.
2. `cargo clippy -p validate-skill --tests` reports no warnings on the test file.
3. Test 1 confirms the tool is listed with the correct name `validate_skill`.
4. Test 2 confirms the tool description contains "Validate".
5. Test 3 confirms the `content` input parameter is exposed in the schema.
6. Test 4 confirms a well-formed skill file produces `valid: true` with a populated manifest.
7. Test 5 confirms content without frontmatter delimiters produces `valid: false` with errors.
8. Test 6 confirms malformed YAML produces `valid: false` with errors.
