# Spec: Migrate validate-skill main.rs to use `serve_stdio_tool`

> From: .claude/tasks/issue-56.md

## Objective
Replace the boilerplate in `tools/validate-skill/src/main.rs` with a single call to `mcp_tool_harness::serve_stdio_tool`, eliminating duplicated tracing setup and stdio-serving logic that is identical across all four MCP tool binaries.

## Current State
- `tools/validate-skill/src/main.rs` contains 24 lines of boilerplate: tracing subscriber initialization, info log, `tool.serve(rmcp::transport::stdio())`, and `service.waiting().await`.
- This is identical in structure to `tools/echo-tool/src/main.rs`, `tools/read-file/src/main.rs`, and `tools/write-file/src/main.rs`.
- `tools/validate-skill/Cargo.toml` directly depends on `rmcp`, `tokio`, `tracing`, and `tracing-subscriber` to support this boilerplate.
- The `crates/mcp-tool-harness` crate does not yet exist. This task is blocked until it lands.

## Requirements
- Replace the body of `main.rs` with a call to `mcp_tool_harness::serve_stdio_tool(ValidateSkillTool::new(), "validate-skill").await`.
- Keep the `mod validate_skill;` declaration and `use validate_skill::ValidateSkillTool;` import.
- Add `mcp-tool-harness = { path = "../../crates/mcp-tool-harness" }` to `[dependencies]` in `tools/validate-skill/Cargo.toml`.
- Remove direct dependencies on `tracing`, `tracing-subscriber` from `Cargo.toml` if they are no longer used in `validate_skill.rs` or any other source file in the crate.
- Retain `rmcp` in `[dependencies]` because `validate_skill.rs` uses rmcp macros and types directly.
- Retain `tokio` in `[dependencies]` because `#[tokio::main]` is used in `main.rs`.
- File must stay under 50 lines.

## Implementation Details

### File: `tools/validate-skill/src/main.rs`

```rust
mod validate_skill;
use validate_skill::ValidateSkillTool;

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    mcp_tool_harness::serve_stdio_tool(ValidateSkillTool::new(), "validate-skill").await
}
```

### File: `tools/validate-skill/Cargo.toml`

Add to `[dependencies]`:
```toml
mcp-tool-harness = { path = "../../crates/mcp-tool-harness" }
```

Remove from `[dependencies]` (only if no other source file in the crate uses them):
- `tracing`
- `tracing-subscriber`

Keep unchanged:
- `rmcp` (used in `validate_skill.rs` for `#[tool]`, `ServerHandler`, etc.)
- `tokio` (used for `#[tokio::main]`)
- `serde`, `serde_json` (used in `validate_skill.rs`)
- `skill-loader`, `agent-sdk` (used in `validate_skill.rs`)

Before removing `tracing`, verify whether `validate_skill.rs` contains any `tracing::` calls. If it does, keep `tracing` as a dependency. `tracing-subscriber` is only used in `main.rs` boilerplate, so it can be removed.

### Key decisions
- **`flavor = "current_thread"`**: Retained from the current implementation. A single-threaded tokio runtime is sufficient for a stdio-based MCP server.
- **`Box<dyn std::error::Error>`**: Kept as the return type, consistent with echo-tool pattern and avoiding extra dependencies.
- **No `use` imports for rmcp or tracing in main.rs**: The harness handles all of that internally.

### Line budget
- Module declaration + use: 2 lines
- `main` function: 4 lines
- Blank line: 1 line
- **Total: ~7 lines** (well under the 50-line limit)

## Dependencies
- Blocked by: "Create `crates/mcp-tool-harness` crate with `serve_stdio_tool` function"
- Blocking: "Run verification suite"

## Risks & Edge Cases

1. **Harness crate not yet available:** This task cannot be implemented until `crates/mcp-tool-harness` is created and added to the workspace. Attempting to build before that will fail.

2. **`tracing` usage in `validate_skill.rs`:** Before removing the `tracing` dependency, check whether `validate_skill.rs` uses `tracing::info!`, `tracing::error!`, or similar macros. If it does, `tracing` must remain in `Cargo.toml`. Only `tracing-subscriber` is guaranteed removable since it is only used in the current `main.rs` boilerplate.

3. **Behavioral equivalence:** The harness function must produce identical runtime behavior: tracing to stderr with ANSI disabled, `EnvFilter` defaulting to `DEBUG`, and the same `"Starting validate-skill MCP server"` log message format. Any deviation in the harness would affect all migrated tools.

4. **Dev-dependencies unchanged:** The `[dev-dependencies]` section (used for integration tests) is not modified by this task. Test migration is handled by a separate task.

## Verification

1. **File exists and is minimal:** `tools/validate-skill/src/main.rs` is under 10 lines.
2. **Compilation:** `cargo check -p validate-skill` succeeds (once harness crate is available).
3. **Lint:** `cargo clippy -p validate-skill` produces no warnings.
4. **Server starts:** `cargo run -p validate-skill` logs "Starting validate-skill MCP server" to stderr and waits for MCP client input.
5. **Integration tests pass:** `cargo test -p validate-skill` passes with no regressions.
6. **Workspace tests:** `cargo test` across the full workspace still passes.
7. **No removed dep regressions:** Grep the crate source for any `tracing_subscriber::` or `tracing::` usage to confirm dependency removal is safe.
