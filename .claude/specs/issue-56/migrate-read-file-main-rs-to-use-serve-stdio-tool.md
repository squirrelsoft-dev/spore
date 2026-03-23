# Spec: Migrate read-file main.rs to use `serve_stdio_tool`

> From: .claude/tasks/issue-56.md

## Objective
Replace the boilerplate in `tools/read-file/src/main.rs` with a call to `mcp_tool_harness::serve_stdio_tool`, and update `tools/read-file/Cargo.toml` to depend on the harness crate while removing now-unnecessary direct dependencies.

## Current State
- `tools/read-file/src/main.rs` contains 24 lines of boilerplate: tracing initialisation, stdio transport setup, and service waiting. This is identical to every other tool entrypoint (`echo-tool`, `write-file`, `validate-skill`).
- The `crates/mcp-tool-harness` crate (created by a separate task) exposes `serve_stdio_tool<T: ServerHandler>(tool: T, tool_name: &str)` that encapsulates all of this boilerplate.
- `tools/read-file/src/read_file.rs` does not import `tracing` or `tracing-subscriber` directly, so those dependencies are only used by `main.rs` today.

## Requirements
1. Replace the body of `tools/read-file/src/main.rs` to call `mcp_tool_harness::serve_stdio_tool(ReadFileTool::new(), "read-file").await`.
2. Add `mcp-tool-harness = { path = "../../crates/mcp-tool-harness" }` to `[dependencies]` in `tools/read-file/Cargo.toml`.
3. Remove the following direct dependencies from `tools/read-file/Cargo.toml` since they are no longer used by the crate itself (they are pulled transitively through the harness):
   - `tracing`
   - `tracing-subscriber`
   - `rmcp` from `[dependencies]` (the harness re-exports the serve functionality; `rmcp` remains in `[dev-dependencies]` for integration tests).
4. Keep `tokio` in `[dependencies]` (needed for the `#[tokio::main]` attribute).
5. Keep `serde` and `serde_json` in `[dependencies]` (used by `read_file.rs`).
6. The resulting `main.rs` must be under 10 lines (excluding blank lines and comments), matching the post-migration echo-tool pattern.
7. No new external dependencies are introduced.

## Implementation Details

### File: `tools/read-file/src/main.rs`

```rust
mod read_file;
use read_file::ReadFileTool;

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    mcp_tool_harness::serve_stdio_tool(ReadFileTool::new(), "read-file").await
}
```

### File: `tools/read-file/Cargo.toml`

Changes to `[dependencies]`:
- **Add:** `mcp-tool-harness = { path = "../../crates/mcp-tool-harness" }`
- **Remove:** `tracing`, `tracing-subscriber`, `rmcp` (from `[dependencies]` only)
- **Keep:** `tokio`, `serde`, `serde_json`

The `[dev-dependencies]` section remains unchanged (`tokio` with `rt-multi-thread`, `rmcp` with client features, `serde_json`).

### Key decisions
- **`rmcp` removed from `[dependencies]`:** The `ReadFileTool` struct in `read_file.rs` imports `rmcp` types (`ServerHandler`, `tool`, `schemars`, etc.) via `use rmcp::...`. However, `rmcp` is a dependency of `mcp-tool-harness`, and Cargo makes transitive dependencies available when they are re-exported. If `mcp-tool-harness` does not re-export `rmcp` publicly, then `rmcp` must be retained in `[dependencies]`. Verify at implementation time; if `read_file.rs` fails to compile without a direct `rmcp` dependency, keep it.
- **`flavor = "current_thread"`:** Retained to match the existing behaviour and the harness function's expectations.
- **No `use` import for harness:** The function is called with its fully-qualified path `mcp_tool_harness::serve_stdio_tool` to keep the file minimal and make the dependency obvious.

## Dependencies
- **Blocked by:** "Create `crates/mcp-tool-harness` crate with `serve_stdio_tool` function"
- **Blocking:** "Run verification suite"

## Risks & Edge Cases

1. **Transitive `rmcp` availability:** If `mcp-tool-harness` does not re-export `rmcp` types publicly, the `read_file.rs` module will fail to compile after removing `rmcp` from direct dependencies. In that case, `rmcp` must remain in `[dependencies]`. Check with `cargo check -p read-file` after the change.

2. **Tracing still works:** The harness initialises tracing internally. After migration, `tools/read-file` no longer controls tracing configuration directly. This is the intended design -- all tools share identical tracing setup through the harness.

3. **Behavioural equivalence:** The harness function must produce identical runtime behaviour: tracing to stderr with ANSI disabled, `EnvFilter` from environment, info-level startup log `"Starting read-file MCP server"`, and `.waiting().await` for graceful shutdown. Any divergence is a bug in the harness, not in this migration.

4. **Integration tests unaffected:** The integration tests in `tools/read-file/tests/` spawn the binary as a child process. They do not depend on the internal structure of `main.rs`, so they should continue to pass without modification.

## Verification

1. **Compilation:** `cargo check -p read-file` succeeds.
2. **Lint:** `cargo clippy -p read-file` produces no warnings.
3. **Line count:** `wc -l tools/read-file/src/main.rs` is under 10 lines.
4. **Unit tests:** `cargo test -p read-file` passes.
5. **Workspace tests:** `cargo test` across the full workspace still passes.
6. **Server starts:** `cargo run -p read-file` logs "Starting read-file MCP server" to stderr and waits for input.
7. **No direct tracing imports:** `grep -r 'tracing' tools/read-file/src/main.rs` returns no matches.
8. **Dependency removed:** `tracing` and `tracing-subscriber` do not appear in `tools/read-file/Cargo.toml` `[dependencies]`.
