# Spec: Migrate write-file main.rs to use `serve_stdio_tool`

> From: .claude/tasks/issue-56.md

## Objective
Replace the hand-written boilerplate in `tools/write-file/src/main.rs` with a single call to `mcp_tool_harness::serve_stdio_tool`, matching the same migration pattern applied to echo-tool. This eliminates duplicated tracing setup and stdio-serving logic across tool entrypoints.

## Current State
- `tools/write-file/src/main.rs` contains 24 lines of boilerplate: tracing subscriber initialization, log statement, `.serve(rmcp::transport::stdio())`, and `.waiting().await`.
- This is identical in structure to `tools/echo-tool/src/main.rs`, `tools/read-file/src/main.rs`, and `tools/validate-skill/src/main.rs`.
- The `crates/mcp-tool-harness` crate (created in Group 1 of issue-56) will provide `serve_stdio_tool<T: ServerHandler>(tool: T, tool_name: &str)` that encapsulates all of this boilerplate.
- `tools/write-file/src/write_file.rs` does not use `tracing` or `tracing-subscriber` directly, so those dependencies can be removed from `Cargo.toml`.

## Requirements
- Replace the body of `tools/write-file/src/main.rs` to delegate to `mcp_tool_harness::serve_stdio_tool`.
- Add `mcp-tool-harness` as a dependency in `tools/write-file/Cargo.toml`.
- Remove direct dependencies on `tracing`, `tracing-subscriber`, and `rmcp`'s `transport-io` feature from `tools/write-file/Cargo.toml` since they are no longer used directly. Keep `rmcp` with features `server` and `macros` (used by `write_file.rs`).
- Remove the `tokio` `io-std` feature if it is no longer needed.
- The resulting `main.rs` must be under 10 lines (excluding blank lines).

## Implementation Details

### File: `tools/write-file/src/main.rs`

```rust
mod write_file;
use write_file::WriteFileTool;

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    mcp_tool_harness::serve_stdio_tool(WriteFileTool::new(), "write-file").await
}
```

### File: `tools/write-file/Cargo.toml`

Changes to `[dependencies]`:
- **Add:** `mcp-tool-harness = { path = "../../crates/mcp-tool-harness" }`
- **Remove:** `tracing = "0.1"`
- **Remove:** `tracing-subscriber = { version = "0.3", features = ["env-filter"] }`
- **Remove** `transport-io` from `rmcp` features (now handled by the harness)
- **Keep:** `rmcp = { version = "1", features = ["server", "macros"] }` (used by `write_file.rs`)
- **Keep:** `tokio = { version = "1", features = ["macros", "rt"] }` (needed for `#[tokio::main]`)
- **Keep:** `serde`, `serde_json` (used by `write_file.rs`)

The `[dev-dependencies]` section is unchanged.

### Key decisions
- **`flavor = "current_thread"` stays in main.rs:** The tokio runtime annotation must remain in the binary entrypoint. The harness function is `async` but not `#[tokio::main]` itself.
- **Return type stays `Box<dyn std::error::Error>`:** Matches the harness function signature and avoids adding `anyhow`.
- **Module declaration stays in main.rs:** `mod write_file;` must remain here since it declares the sibling module for the binary crate.

## Dependencies
- **Blocked by:** Create `crates/mcp-tool-harness` crate with `serve_stdio_tool` function (Group 1)
- **Blocking:** Run verification suite (Group 5)

## Risks & Edge Cases

1. **Harness crate not yet available:** This task cannot be implemented until `crates/mcp-tool-harness` lands. The spec is written against the expected `serve_stdio_tool` signature from the task breakdown.

2. **Removing `transport-io` from rmcp features:** The `transport-io` feature is only needed for `rmcp::transport::stdio()`, which moves into the harness. If `write_file.rs` uses any other rmcp transport APIs, this removal would break compilation. Confirmed: `write_file.rs` does not use transport APIs.

3. **Tracing still works:** Even though `tracing` and `tracing-subscriber` are removed from this crate's direct dependencies, the harness initializes tracing before calling the tool. Any `tracing::info!()` calls in `write_file.rs` would still work via the transitive dependency -- but currently `write_file.rs` has zero tracing calls, so this is not a concern.

4. **Dev-dependency on rmcp with `transport-child-process`:** The `[dev-dependencies]` section still pulls in rmcp with `client` and `transport-child-process` features for integration tests. This is unaffected by the main.rs migration.

## Verification

1. **File length:** `wc -l tools/write-file/src/main.rs` is under 10 lines.
2. **Pattern match:** The file structurally mirrors the migrated echo-tool `main.rs`.
3. **Compilation:** `cargo check -p write-file` succeeds.
4. **Lint:** `cargo clippy -p write-file` produces no warnings.
5. **Tests pass:** `cargo test -p write-file` passes (all existing unit and integration tests).
6. **Server starts:** `cargo run -p write-file` logs "Starting write-file MCP server" to stderr and waits for input.
7. **Workspace clean:** `cargo test` across the full workspace still passes.
8. **No leftover imports:** `main.rs` does not import `rmcp::ServiceExt`, `tracing_subscriber`, or `tracing` directly.
