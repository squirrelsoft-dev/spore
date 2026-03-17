# Spec: Write integration test for MCP server round-trip

> From: .claude/tasks/issue-10.md

## Objective

Create an integration test that exercises the echo-tool as a real MCP server over stdio transport. The test spawns the `echo-tool` binary as a child process, connects to it as an MCP client using rmcp's client-side API, and performs full round-trip protocol exchanges: listing tools to verify schema advertisement, and calling the `echo` tool to verify it returns the input message unchanged. This validates that the MCP server implementation works end-to-end, covering serialization, transport, tool routing, and response formatting -- areas that unit tests (which call the tool method directly) cannot cover.

## Current State

- **Echo tool does not exist yet.** The `tools/echo-tool/` crate has not been created. The implementation spec (`.claude/specs/issue-10/implement-echo-tool-server.md`) defines:
  - An `EchoTool` struct with a `#[tool_router]` impl block containing an `echo` method.
  - The `echo` tool accepts a `message: String` parameter and returns `CallToolResult::success(vec![Content::text(message)])`.
  - A `ServerHandler` impl that advertises the tool via `ServerCapabilities::builder().enable_tools().build()`.
  - The binary starts an MCP server over stdio transport using `rmcp::transport::stdio()`.

- **rmcp client-side API (from docs.rs and official examples):**
  - `TokioChildProcess::new(command)` spawns a child process and wraps it as an MCP transport. Requires the `transport-child-process` feature on `rmcp`.
  - `().serve(transport).await` establishes a client connection (the `()` implements `ClientHandler` as a no-op handler). Returns a `RunningService<RoleClient, ()>`.
  - The `RunningService` derefs to `Peer<RoleClient>` which provides:
    - `list_tools(params)` -- sends `tools/list` request, returns `ListToolsResult { tools: Vec<Tool>, ... }`.
    - `list_all_tools()` -- convenience that handles pagination, returns `Vec<Tool>`.
    - `call_tool(params)` -- sends `tools/call` request, returns `CallToolResult { content: Vec<Content>, is_error: Option<bool>, ... }`.
    - `peer_info()` -- returns `Option<&ServerInfo>` with the server's advertised capabilities.
    - `cancel()` -- cancels the service and waits for cleanup.
  - `CallToolRequestParams::new("tool_name").with_arguments(json_object)` constructs a tool call request.
  - `ConfigureCommandExt` provides `.configure(|cmd| { ... })` for setting up `tokio::process::Command` arguments.
  - The `object!` macro from `rmcp` creates a `JsonObject` inline (similar to `serde_json::json!` but returns a `Map<String, Value>` directly).

- **Workspace test conventions:**
  - Integration tests live in `tests/` directories (e.g., `crates/skill-loader/tests/skill_loader_test.rs`).
  - Each test file is a separate compilation unit with its own imports.
  - Async tests use `#[tokio::test]` with `tokio` as a dev-dependency.
  - Assertions use `assert_eq!`, `assert!`, and pattern matching.
  - No mocking frameworks are used.
  - Test functions have descriptive names that state the expected behavior (e.g., `load_valid_skill_with_full_frontmatter`, `invoke_returns_ok`).

- **Cargo.toml (from scaffolding spec):** The current `[dev-dependencies]` only has `tokio = { version = "1", features = ["macros", "rt"] }`. This task must add `rmcp` with client and transport-child-process features as a dev-dependency.

## Requirements

1. **File location:** Create `tools/echo-tool/tests/echo_server_test.rs` as an integration test file.

2. **Dev-dependency additions:** Add `rmcp` with `client` and `transport-child-process` features to `[dev-dependencies]` in `tools/echo-tool/Cargo.toml`. Also add `serde_json = "1"` to dev-dependencies for constructing tool arguments.

3. **Binary spawning:** Tests must spawn the `echo-tool` binary as a child process using `TokioChildProcess` from rmcp's `transport-child-process` feature. The binary is located via `cargo build -p echo-tool` output or by using `env!("CARGO_BIN_EXE_echo-tool")` (which Cargo sets automatically for integration tests of binary crates). The child process uses stdio for the MCP transport.

4. **Client connection:** Tests must connect to the spawned binary as an MCP client using rmcp's `ServiceExt::serve()` on the `TokioChildProcess` transport. The client handler can be `()` (the no-op default implementation).

5. **Five test cases**, each as a separate `#[tokio::test] async fn`:

   | # | Test name | Action | Assertion |
   |---|-----------|--------|-----------|
   | 1 | `tools_list_returns_echo_tool` | Call `client.list_all_tools()` | Returns exactly 1 tool with `name == "echo"` |
   | 2 | `tools_list_echo_has_correct_description` | Call `client.list_all_tools()` | The `echo` tool has `description` containing `"Returns the input message unchanged"` |
   | 3 | `tools_list_echo_has_message_parameter` | Call `client.list_all_tools()` | The `echo` tool's `input_schema` has a `properties` key containing `"message"` |
   | 4 | `tools_call_echo_returns_message` | Call `client.call_tool()` with `{ "message": "hello" }` | Response `content` has 1 element containing the text `"hello"`, `is_error` is `None` or `Some(false)` |
   | 5 | `tools_call_echo_preserves_unicode` | Call `client.call_tool()` with `{ "message": "Hello 42" }` | Response `content` has 1 element containing the text `"Hello 42"` |

6. **Cleanup:** Each test must call `client.cancel().await` (or equivalent) to shut down the child process after assertions. Use a helper function or consistent cleanup pattern.

7. **Timeout safety:** Tests should not hang indefinitely. The tokio test runtime has a default timeout behavior, but consider using `tokio::time::timeout` around the entire test body if hangs are a concern during development.

8. **No test ordering dependencies:** Each test spawns its own independent child process and client. Tests are fully parallelizable.

## Implementation Details

### File to create: `tools/echo-tool/tests/echo_server_test.rs`

```rust
use rmcp::{
    ServiceExt,
    model::CallToolRequestParams,
    transport::TokioChildProcess,
};
use tokio::process::Command;

/// Helper: spawn the echo-tool binary and connect as an MCP client.
/// Returns the running client service.
async fn spawn_echo_client() -> rmcp::service::RunningService<rmcp::RoleClient, ()> {
    let transport = TokioChildProcess::new(
        Command::new(env!("CARGO_BIN_EXE_echo-tool"))
    ).expect("failed to spawn echo-tool");

    ().serve(transport)
        .await
        .expect("failed to connect to echo-tool server")
}

#[tokio::test]
async fn tools_list_returns_echo_tool() {
    let client = spawn_echo_client().await;

    let tools = client.list_all_tools().await.expect("list_all_tools failed");
    assert_eq!(tools.len(), 1, "expected exactly 1 tool, got {}", tools.len());
    assert_eq!(tools[0].name, "echo");

    client.cancel().await.expect("shutdown failed");
}

#[tokio::test]
async fn tools_list_echo_has_correct_description() {
    let client = spawn_echo_client().await;

    let tools = client.list_all_tools().await.expect("list_all_tools failed");
    let echo_tool = &tools[0];
    let description = echo_tool.description.as_deref().expect("echo tool should have a description");
    assert!(
        description.contains("Returns the input message unchanged"),
        "unexpected description: {description}"
    );

    client.cancel().await.expect("shutdown failed");
}

#[tokio::test]
async fn tools_list_echo_has_message_parameter() {
    let client = spawn_echo_client().await;

    let tools = client.list_all_tools().await.expect("list_all_tools failed");
    let echo_tool = &tools[0];
    let schema = &echo_tool.input_schema;
    // The input_schema is a JsonObject (Map<String, Value>).
    // It should have a "properties" key containing "message".
    let properties = schema
        .get("properties")
        .expect("input_schema should have 'properties'");
    assert!(
        properties.get("message").is_some(),
        "input_schema properties should include 'message', got: {properties:?}"
    );

    client.cancel().await.expect("shutdown failed");
}

#[tokio::test]
async fn tools_call_echo_returns_message() {
    let client = spawn_echo_client().await;

    let result = client
        .call_tool(CallToolRequestParams::new("echo").with_arguments(
            serde_json::json!({ "message": "hello" })
                .as_object()
                .unwrap()
                .clone(),
        ))
        .await
        .expect("call_tool failed");

    assert!(
        result.is_error.is_none() || !result.is_error.unwrap(),
        "expected success, got error"
    );
    assert_eq!(result.content.len(), 1, "expected 1 content item");
    // Extract text from the Content variant and assert it matches "hello"
    // Exact extraction depends on Content enum API (see Risks section)

    client.cancel().await.expect("shutdown failed");
}

#[tokio::test]
async fn tools_call_echo_preserves_unicode() {
    let client = spawn_echo_client().await;

    let input = "Hello 42";
    let result = client
        .call_tool(CallToolRequestParams::new("echo").with_arguments(
            serde_json::json!({ "message": input })
                .as_object()
                .unwrap()
                .clone(),
        ))
        .await
        .expect("call_tool failed");

    assert!(
        result.is_error.is_none() || !result.is_error.unwrap(),
        "expected success, got error"
    );
    assert_eq!(result.content.len(), 1, "expected 1 content item");
    // Extract text from Content and assert it equals input

    client.cancel().await.expect("shutdown failed");
}
```

**Note on Content text extraction:** The `Content` type in rmcp is an enum with variants like `Text`, `Image`, `Resource`, etc. To extract the text string from a `Content::Text` variant, the implementer should use pattern matching or the type's accessor methods. The exact code depends on rmcp's API. For example:
- If `Content` has a method like `.as_text() -> Option<&str>`, use that.
- If it is an enum variant like `Content::Text { text }`, use `match` or `if let`.
- If `Content` has an `Annotated<TextContent>` wrapper, destructure accordingly.

The implementer should inspect `rmcp::model::Content` (via `cargo doc -p rmcp --open` or IDE autocompletion) and adapt the assertion. The illustrative code above deliberately leaves a comment placeholder for this because the exact API must be verified at implementation time.

**Note on `with_arguments`:** The `with_arguments` method expects a `JsonObject` (i.e., `serde_json::Map<String, Value>`). The pattern `serde_json::json!({...}).as_object().unwrap().clone()` converts from `Value` to `Map`. Alternatively, the rmcp `object!` macro can be used if it is in scope: `object!({ "message": "hello" })`.

### File to modify: `tools/echo-tool/Cargo.toml`

Add the following to the existing `[dev-dependencies]` section:

```toml
[dev-dependencies]
rmcp = { version = "1", features = ["client", "transport-child-process"] }
serde_json = "1"
tokio = { version = "1", features = ["macros", "rt"] }
```

The `tokio` entry already exists in dev-dependencies from the scaffolding task. The new additions are `rmcp` (with client + transport features for the test client) and `serde_json` (for constructing tool call arguments).

**Feature rationale:**
- `client` -- enables `RoleClient`, `Peer<RoleClient>` methods (`list_tools`, `call_tool`, etc.), and `ClientHandler` trait.
- `transport-child-process` -- enables `TokioChildProcess` for spawning the binary as a child process with stdio transport.
- `serde_json` -- already a regular dependency, but explicitly listing it under dev-dependencies is harmless and makes the test file's dependency on it clear. (Cargo merges features from both sections.)

### Helper function: `spawn_echo_client`

A shared helper function (`spawn_echo_client`) encapsulates the boilerplate of spawning the binary and connecting as a client. This keeps each test function focused on its specific assertion. The helper:

1. Uses `env!("CARGO_BIN_EXE_echo-tool")` to locate the compiled binary. This is a Cargo-provided environment variable that resolves to the absolute path of the binary built for integration tests. It works reliably across platforms and build configurations.
2. Creates a `TokioChildProcess` transport wrapping the spawned binary.
3. Calls `().serve(transport).await` to establish the MCP client connection, where `()` is the no-op `ClientHandler`.
4. Returns the `RunningService` (which derefs to `Peer<RoleClient>` for tool operations).

### No other files created or modified

This task only creates the integration test file and modifies `Cargo.toml` dev-dependencies. It does not modify source code, the workspace root `Cargo.toml`, or any other files.

## Dependencies

- **Blocked by:**
  - "Implement echo tool server" (Group 2, issue #10) -- the `echo-tool` binary must exist and function as an MCP server for the integration test to connect to it.
  - "Create `tools/echo-tool/` crate with Cargo.toml" (Group 1, issue #10) -- the crate must exist with its `Cargo.toml`.
  - "Add `tools/echo-tool` to workspace members" (Group 1, issue #10) -- the crate must be in the workspace for `cargo test -p echo-tool` to build integration tests.

- **Blocking:**
  - "Run verification suite" (Group 4, issue #10) -- the verification task runs `cargo test` across the workspace and depends on these tests existing and passing.

## Risks & Edge Cases

1. **rmcp `Content` enum API:** The exact structure of `Content` and how to extract the text string from it is not fully documented. The implementer must inspect the actual rmcp type (via `cargo doc`, IDE, or source code) and adapt the text extraction assertions accordingly. The spec deliberately leaves this as a comment placeholder in the illustrative code. If `Content` does not implement `PartialEq`, pattern matching or accessor methods will be needed.

2. **`CARGO_BIN_EXE_echo-tool` availability:** This environment variable is set by Cargo for integration tests of binary crates. It contains the path to the compiled binary. If the binary name in `Cargo.toml` differs from `echo-tool` (e.g., uses underscores), the variable name changes accordingly. The implementer must verify the exact binary name matches `echo-tool` (with hyphen, as specified in `Cargo.toml`'s `name = "echo-tool"`).

3. **Child process startup time:** The echo-tool binary may take a brief moment to initialize (tracing subscriber setup, MCP server initialization). The rmcp client's `serve()` call handles the MCP initialization handshake (`initialize` request/response), so by the time `spawn_echo_client` returns, the server should be ready. However, if flaky timing issues occur, consider adding a small delay or retry logic. This is unlikely for a simple echo server.

4. **Child process cleanup:** If a test panics before calling `client.cancel()`, the child process may be orphaned. Rust's `Drop` implementation on `TokioChildProcess` should kill the child process, and the test harness will clean up, but this is worth monitoring. If cleanup is a concern, wrap assertions and cleanup in a function that handles both success and failure paths.

5. **`with_arguments` type conversion:** The `CallToolRequestParams::with_arguments` method expects a `JsonObject` (`serde_json::Map<String, Value>`), not a `serde_json::Value`. The pattern `serde_json::json!({...}).as_object().unwrap().clone()` handles this conversion. Alternatively, rmcp's `object!` macro can be used directly. If the implementer finds `object!` available and more ergonomic, it is acceptable to use either approach.

6. **Port/transport conflicts:** Each test spawns an independent child process communicating over stdio pipes. There are no shared ports or resources, so tests are fully parallelizable without risk of conflicts.

7. **Test compilation time:** Adding `rmcp` with client features as a dev-dependency increases compilation time for tests. This is unavoidable for integration testing against the real MCP protocol but should not affect production build times since dev-dependencies are only built for tests.

8. **Stderr output from child process:** The echo-tool binary logs to stderr via tracing. In integration tests, this output may appear in the test runner's output. This is informational and should not cause test failures. If the output is noisy, it can be suppressed by not setting `RUST_LOG` or by configuring the child process's stderr to be piped/null, but this is a cosmetic concern and not a requirement.

## Verification

1. `cargo test -p echo-tool --test echo_server_test` compiles and all 5 test functions pass.
2. `cargo clippy -p echo-tool --tests` reports no warnings on the integration test file.
3. `tools_list_returns_echo_tool` confirms the server advertises exactly one tool named `echo`.
4. `tools_list_echo_has_correct_description` confirms the tool description matches the expected string.
5. `tools_list_echo_has_message_parameter` confirms the tool's input schema includes a `message` property.
6. `tools_call_echo_returns_message` confirms the echo tool returns `"hello"` unchanged when called with `{ "message": "hello" }`.
7. `tools_call_echo_preserves_unicode` confirms the echo tool preserves unicode characters in the round-trip.
8. Each test spawns and shuts down its own child process independently, with no shared state.
9. `cargo test` across the full workspace still passes (no regressions).
