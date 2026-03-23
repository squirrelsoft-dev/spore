# Spec: Create `tools/list-agents/Cargo.toml`
> From: .claude/tasks/issue-51.md

## Objective
Create the Cargo.toml manifest for the `list-agents` MCP tool crate, following the same structure as `tools/register-agent/Cargo.toml` but without the `reqwest` dependency (this tool only reads environment variables, it does not make HTTP requests).

## Current State
The `tools/list-agents/` directory does not yet exist. The `tools/register-agent/Cargo.toml` serves as the reference template. It declares `mcp-tool-harness` (path dep), `rmcp` with transport/server/macros features, `tokio`, `serde`, `serde_json`, and `reqwest`. Dev-dependencies include `mcp-test-utils`, `tokio` with `rt-multi-thread`, `rmcp` with client/transport features, and `serde_json`.

## Requirements
1. Create `tools/list-agents/Cargo.toml` with `name = "list-agents"`, `version = "0.1.0"`, `edition = "2024"`.
2. Dependencies (identical to `register-agent` minus `reqwest`):
   - `mcp-tool-harness = { path = "../../crates/mcp-tool-harness" }`
   - `rmcp = { version = "1", features = ["transport-io", "server", "macros"] }`
   - `tokio = { version = "1", features = ["macros", "rt", "io-std"] }`
   - `serde = { version = "1", features = ["derive"] }`
   - `serde_json = "1"`
3. Dev-dependencies (same as `register-agent`):
   - `mcp-test-utils = { path = "../../crates/mcp-test-utils" }`
   - `tokio = { version = "1", features = ["macros", "rt", "rt-multi-thread", "net"] }`
   - `rmcp = { version = "1", features = ["client", "transport-child-process"] }`
   - `serde_json = "1"`
4. No `reqwest` dependency in either `[dependencies]` or `[dev-dependencies]`.

## Implementation Details
- Copy `tools/register-agent/Cargo.toml` as the starting point.
- Change `name` from `"register-agent"` to `"list-agents"`.
- Remove the `reqwest` line from `[dependencies]`.
- Keep everything else unchanged.
- Ensure the workspace `Cargo.toml` at the repo root includes `"tools/list-agents"` in its `members` list (this may be handled by a separate spec/task).

## Dependencies
- Blocked by: none (Group 1)
- Blocking: "Implement `ListAgentsTool` struct and handler", "Write `main.rs`", "Write integration tests"

## Risks & Edge Cases
- The `../../crates/mcp-tool-harness` and `../../crates/mcp-test-utils` relative paths assume the standard two-level nesting under `tools/list-agents/`. If the directory is placed elsewhere, paths will break.
- The crate will not compile until `src/main.rs` (or `src/lib.rs`) exists; this is expected since the Cargo.toml is created first as a Group 1 task.
- The workspace root `Cargo.toml` must list this crate as a member for `cargo build` to pick it up.

## Verification
1. Run `cargo metadata --manifest-path tools/list-agents/Cargo.toml --no-deps` and confirm the package name is `list-agents` with the correct dependency set.
2. Confirm `reqwest` does not appear anywhere in the file.
3. Confirm the dependency versions and feature flags match the spec above exactly.
