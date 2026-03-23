# Spec: Create register-agent Cargo.toml

> From: .claude/tasks/issue-50.md

## Objective
Create the Cargo manifest for the `register-agent` MCP tool crate and add it to the workspace. This tool will make HTTP POST requests to the orchestrator to register agents, so it needs `reqwest` as an additional runtime dependency and a lightweight HTTP server for testing. This is the foundational file that must exist before any Rust source files can be written.

## Current State
- `tools/docker-push/Cargo.toml` is the closest existing template. It declares `mcp-tool-harness` (path), `rmcp`, `tokio`, `serde`, and `serde_json` as dependencies, plus `mcp-test-utils` and test-oriented features in dev-dependencies.
- The workspace root `Cargo.toml` lists tool crates under `[workspace] members`. Currently `tools/docker-push` and `tools/docker-build` are the most recently added members. `tools/register-agent` is not yet listed.
- The directory `tools/register-agent/` does not yet exist.
- No existing tool crate uses `reqwest` or `axum`, so these will be new workspace-level dependencies.

## Requirements
- The file `tools/register-agent/Cargo.toml` must be a valid Cargo manifest with `name = "register-agent"`.
- `edition` must be `"2024"`, matching all existing tool crates.
- `version` must be `"0.1.0"`.
- `[dependencies]` must include exactly:
  - `mcp-tool-harness = { path = "../../crates/mcp-tool-harness" }`
  - `rmcp = { version = "1", features = ["transport-io", "server", "macros"] }`
  - `tokio = { version = "1", features = ["macros", "rt", "io-std"] }`
  - `serde = { version = "1", features = ["derive"] }`
  - `serde_json = "1"`
  - `reqwest = { version = "0.12", features = ["json"] }` -- needed for HTTP POST to the orchestrator
- `[dev-dependencies]` must include exactly:
  - `mcp-test-utils = { path = "../../crates/mcp-test-utils" }`
  - `tokio = { version = "1", features = ["macros", "rt", "rt-multi-thread"] }`
  - `rmcp = { version = "1", features = ["client", "transport-child-process"] }`
  - `serde_json = "1"`
  - `axum = "0.8"` -- lightweight mock HTTP server for integration tests (preferred over adding `mockito` or `wiremock` per task instructions)
- The workspace root `Cargo.toml` must add `"tools/register-agent"` to the `[workspace] members` array.
- No other external dependencies beyond those listed above.

## Implementation Details
- **Files to create:**
  - `tools/register-agent/Cargo.toml` -- based on the `tools/docker-push/Cargo.toml` pattern with the following differences:
    - `name` changed to `"register-agent"`
    - `reqwest = { version = "0.12", features = ["json"] }` added to `[dependencies]`
    - `axum = "0.8"` added to `[dev-dependencies]`
- **Files to modify:**
  - `Cargo.toml` (workspace root) -- add `"tools/register-agent"` to the `[workspace] members` array, placed alphabetically after `"tools/read-file"`.
- No Rust source files are created in this task. The crate will not compile until `src/main.rs` is added by a blocking task.

## Dependencies
- Blocked by: none (Group 1)
- Blocking: "Implement register_agent tool logic", "Create main.rs entry point", "Write integration tests"

## Risks & Edge Cases
- **New external dependencies**: `reqwest` and `axum` are new to the workspace. `reqwest` pulls in a significant dependency tree (hyper, http, tower, rustls/native-tls). Ensure the `reqwest` version chosen is compatible with the existing `rmcp` and `tokio` versions already in the lockfile. Using `0.12` aligns with the current tokio 1.x ecosystem.
- **`axum` version alignment**: `axum` 0.8 depends on tokio 1.x and hyper 1.x, which should be compatible with `reqwest` 0.12. If version conflicts arise, pinning to a specific patch version may be needed.
- **Workspace resolution failure**: If the workspace root `Cargo.toml` is not updated, `cargo build` will not find the new crate. Mitigation: always update the members list as part of this task.
- **Path dependency correctness**: The relative path `../../crates/mcp-tool-harness` assumes the crate lives at `tools/register-agent/`. If the directory is placed elsewhere, the path will break. Mitigation: verify the directory structure matches `tools/docker-push/`.
- **Incomplete task**: Because no `src/main.rs` exists yet, `cargo check -p register-agent` will fail after this task alone. This is expected and resolved by the blocking tasks.

## Verification
- `cat tools/register-agent/Cargo.toml` shows the correct manifest with `name = "register-agent"`, edition 2024, and all required dependencies including `reqwest` and `axum`.
- `grep "tools/register-agent" Cargo.toml` confirms the workspace root includes the new member.
- `cargo metadata --no-deps` lists `register-agent` as a workspace member (will only fully succeed once `src/main.rs` exists from the blocking task).
- Verify `reqwest` features include `json` and `axum` appears only in `[dev-dependencies]`.
