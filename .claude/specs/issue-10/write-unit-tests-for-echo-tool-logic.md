# Spec: Write unit tests for echo tool logic

> From: .claude/tasks/issue-10.md

## Objective

Add inline unit tests for the echo tool's core handler method to verify that it returns the input message unchanged as `Content::text(...)` inside a successful `CallToolResult`. These tests exercise the tool logic directly (without starting an MCP server), ensuring the echo behavior is correct before integration testing covers the full server round-trip.

## Current State

- **Echo tool does not exist yet.** The `tools/echo-tool/` crate is not created. The task breakdown (`.claude/tasks/issue-10.md`) specifies that `main.rs` (and optionally `echo.rs`) will define an `EchoTool` struct with a `#[tool(description = "...")]` method `echo` that accepts a `message: String` and returns `Result<CallToolResult, McpError>` with `CallToolResult::success(vec![Content::text(message)])`.

- **rmcp API surface (from task breakdown and rmcp docs):**
  - `#[tool_router]` on the impl block generates a router; each `#[tool(...)]` method becomes a callable tool.
  - `CallToolResult` has a `content` field (`Vec<Content>`) and can be constructed via `CallToolResult::success(vec![Content::text(...)])`.
  - `Content::text(msg)` creates a `Content` variant holding a text string.
  - `McpError` is the error type for tool methods.
  - The `echo` method is `async` and takes `&self` plus a `message: String` parameter.

- **Workspace test conventions:**
  - Inline tests use `#[cfg(test)] mod tests { use super::*; ... }` (see `crates/skill-loader/src/frontmatter.rs`, `crates/skill-loader/src/validation.rs`).
  - Integration tests live in `tests/` directories (see `crates/agent-sdk/tests/`, `crates/skill-loader/tests/`).
  - Async tests use `#[tokio::test]` with `tokio` as a dev-dependency (see `crates/agent-sdk/tests/micro_agent_test.rs`).
  - Tests construct structs directly and assert on return values. No mocking frameworks are used.
  - Assertions use `assert_eq!`, `assert!`, and pattern matching. Substring checks via `.contains()` are preferred over exact string equality for error messages.

- **Dev-dependencies (from task breakdown):** `Cargo.toml` will include `tokio = { version = "1", features = ["macros", "rt"] }` under `[dev-dependencies]`, which provides `#[tokio::test]`.

## Requirements

1. **File location:** The `#[cfg(test)] mod tests` block must be placed in the file that defines `EchoTool` and its `echo` method. This will be either `tools/echo-tool/src/main.rs` or `tools/echo-tool/src/echo.rs`, depending on whether the implementer extracts the tool into a separate module (as suggested by the task breakdown's 50-line limit guideline).

2. **Direct construction:** Tests must construct `EchoTool` directly (e.g., `let tool = EchoTool;` for a unit struct, or `EchoTool {}` / `EchoTool::default()` for an empty struct). No MCP server startup, no transport layer, no client connection.

3. **Direct method call:** Tests must call the `echo` method directly on the `EchoTool` instance, passing a test message string. The method is async, so tests use `.await`.

4. **Five test cases**, each as a separate `#[tokio::test] async fn`:

   | # | Test name | Input | Assertion |
   |---|-----------|-------|-----------|
   | 1 | `echo_returns_input_unchanged` | `"hello world"` | Result is `Ok`, `content` vec has exactly 1 element, that element is `Content::text("hello world")` |
   | 2 | `echo_returns_empty_string` | `""` | Result is `Ok`, content contains `Content::text("")` |
   | 3 | `echo_preserves_whitespace_and_special_chars` | `"  line1\nline2\ttab  "` | Result is `Ok`, content text matches the input exactly (whitespace, newlines, tabs preserved) |
   | 4 | `echo_preserves_unicode` | `"Hello 42"` (or another unicode string) | Result is `Ok`, content text matches the input exactly |
   | 5 | `echo_result_is_success` | `"test"` | Result is `Ok`, the `CallToolResult` indicates success (not an error result) |

5. **Assertion strategy for Content:** The `Content` type in rmcp is an enum. To verify the text content, tests should either:
   - Compare directly with `Content::text(expected)` if `Content` derives `PartialEq`.
   - Or extract the text from the content variant and compare the string value.
   The implementer should use whichever approach compiles against the actual rmcp API. The spec prescribes the logical assertion; the exact extraction mechanism depends on rmcp's `Content` type definition.

6. **No server, no transport:** Tests must not start an MCP server, spawn a process, or use any transport. They test the pure function logic only.

7. **Imports:** The test module imports from `super::*` (to access `EchoTool`) and from `rmcp::model::{Content, CallToolResult}` (or wherever these types are re-exported in rmcp).

## Implementation Details

### File to modify: `tools/echo-tool/src/echo.rs` (preferred) or `tools/echo-tool/src/main.rs`

Append a `#[cfg(test)]` module at the bottom of whichever file defines `EchoTool`:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn echo_returns_input_unchanged() {
        let tool = EchoTool;
        let result = tool.echo("hello world".to_string()).await;
        let call_result = result.expect("echo should not fail");
        assert_eq!(call_result.content.len(), 1);
        // Assert the single content element is Content::text("hello world")
    }

    #[tokio::test]
    async fn echo_returns_empty_string() {
        let tool = EchoTool;
        let result = tool.echo(String::new()).await;
        let call_result = result.expect("echo should not fail");
        assert_eq!(call_result.content.len(), 1);
        // Assert content is Content::text("")
    }

    #[tokio::test]
    async fn echo_preserves_whitespace_and_special_chars() {
        let tool = EchoTool;
        let input = "  line1\nline2\ttab  ".to_string();
        let result = tool.echo(input.clone()).await;
        let call_result = result.expect("echo should not fail");
        assert_eq!(call_result.content.len(), 1);
        // Assert content text matches input exactly
    }

    #[tokio::test]
    async fn echo_preserves_unicode() {
        let tool = EchoTool;
        let input = "Hello 42".to_string();
        let result = tool.echo(input.clone()).await;
        let call_result = result.expect("echo should not fail");
        assert_eq!(call_result.content.len(), 1);
        // Assert content text matches input exactly
    }

    #[tokio::test]
    async fn echo_result_is_success() {
        let tool = EchoTool;
        let result = tool.echo("test".to_string()).await;
        let call_result = result.expect("echo should not fail");
        assert!(call_result.is_error.is_none() || !call_result.is_error.unwrap());
    }
}
```

**Note on Content assertion:** The exact field access or method to extract text from `Content` depends on rmcp's API. The `Content` enum in rmcp typically has a variant like `Content::Text { text: String }` or similar. The implementer must inspect the rmcp API (or the generated docs) to determine the correct pattern match or accessor. The code above is illustrative; the implementer should adapt the assertion to compile against the actual types. If `Content` implements `PartialEq`, a direct `assert_eq!(content, Content::text(expected))` is cleanest.

**Note on `is_error` field:** `CallToolResult` in rmcp has an `is_error` field of type `Option<bool>`. A successful result has `is_error` as `None` or `Some(false)`. The implementer should verify this against the actual rmcp type definition.

### No other files created or modified

This task only adds the `#[cfg(test)] mod tests` block. It does not modify `Cargo.toml` (dev-dependencies are handled by the scaffolding task), `lib.rs`, or any other files.

## Dependencies

- **Blocked by:**
  - "Implement echo tool server" (Group 2, issue #10) -- the `EchoTool` struct and its `echo` method must exist before tests can be written against them. The `#[tool_router]` macro must be applied so the method signature is finalized.
  - "Create `tools/echo-tool/` crate with Cargo.toml" (Group 1, issue #10) -- `tokio` must be listed as a dev-dependency for `#[tokio::test]` to compile.
  - "Add `tools/echo-tool` to workspace members" (Group 1, issue #10) -- the crate must be in the workspace for `cargo test -p echo-tool` to work.
- **Blocking:**
  - "Run verification suite" (Group 4, issue #10) -- the verification task runs `cargo test` across the workspace and depends on these tests existing and passing.

## Risks & Edge Cases

1. **rmcp `Content` type API:** The exact structure of `Content` and how to extract the text string may differ from what is shown in the illustrative code. The implementer must adapt assertions to the actual rmcp API. If `Content` does not implement `PartialEq`, pattern matching or accessor methods will be needed instead of `assert_eq!`.

2. **`#[tool_router]` macro interaction:** The `#[tool_router]` macro may transform the `echo` method signature or generate wrapper methods. If the macro makes the `echo` method private or changes its signature, the tests may need to call a different method name or use the generated router interface. The implementer should verify that `tool.echo(message).await` compiles; if not, they should call whatever public method the macro exposes.

3. **`CallToolResult` fields:** The `content` field name and `is_error` field on `CallToolResult` are based on the MCP specification and rmcp's implementation. If rmcp uses different field names (e.g., `result` instead of `content`), the tests must be adapted. The implementer should check `CallToolResult`'s definition in rmcp.

4. **Empty string behavior:** The echo tool is specified to return the input unchanged. An empty string is a valid input and should produce `Content::text("")`. This is a degenerate case worth testing to ensure no accidental filtering or trimming occurs.

5. **Test isolation:** Each test constructs its own `EchoTool` instance and does not share state, so tests are fully parallelizable by the Rust test harness.

6. **Async runtime:** `#[tokio::test]` uses a single-threaded runtime by default. The echo tool method is trivially async (no real I/O), so this is sufficient. No `#[tokio::test(flavor = "multi_thread")]` is needed.

## Verification

1. `cargo test -p echo-tool` compiles and all 5 test functions pass.
2. `cargo clippy -p echo-tool --tests` reports no warnings on the test module.
3. Each test asserts that the echo method returns `Ok(CallToolResult)` with a single `Content::text(...)` element matching the input.
4. Test `echo_returns_empty_string` confirms the tool handles empty input without panicking or returning an error.
5. Test `echo_preserves_whitespace_and_special_chars` confirms no trimming or escaping is applied.
6. Test `echo_result_is_success` confirms the result is not marked as an error.
