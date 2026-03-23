# Spec: Create `tools/docker-build/Cargo.toml`

> From: .claude/tasks/issue-48.md

## Objective
Create the Cargo manifest for the `docker-build` MCP tool crate. This is the foundational file that defines the crate metadata, dependencies, and dev-dependencies needed before any Rust source files can be written. Without it, the crate cannot compile or be included in the workspace.

## Current State
- `tools/cargo-build/Cargo.toml` exists and serves as the template. It declares `mcp-tool-harness` (path), `rmcp`, `tokio`, `serde`, and `serde_json` as dependencies, plus `mcp-test-utils` and test-oriented features in dev-dependencies.
- The workspace `Cargo.toml` at the repo root lists tool crates under `[workspace] members`. Currently `tools/cargo-build` is listed but `tools/docker-build` is not.
- The directory `tools/docker-build/` does not yet exist.

## Requirements
- The file `tools/docker-build/Cargo.toml` must be a valid Cargo manifest with `name = "docker-build"`.
- `edition` must be `"2024"`, matching the existing tool crates.
- `version` must be `"0.1.0"`.
- `[dependencies]` must include exactly:
  - `mcp-tool-harness = { path = "../../crates/mcp-tool-harness" }`
  - `rmcp = { version = "1", features = ["transport-io", "server", "macros"] }`
  - `tokio = { version = "1", features = ["macros", "rt", "io-std"] }`
  - `serde = { version = "1", features = ["derive"] }`
  - `serde_json = "1"`
- `[dev-dependencies]` must include exactly:
  - `mcp-test-utils = { path = "../../crates/mcp-test-utils" }`
  - `tokio = { version = "1", features = ["macros", "rt", "rt-multi-thread"] }`
  - `rmcp = { version = "1", features = ["client", "transport-child-process"] }`
  - `serde_json = "1"`
- The workspace root `Cargo.toml` must add `"tools/docker-build"` to the `members` list so Cargo discovers the crate.
- No new external dependencies beyond what `cargo-build` already uses.

## Implementation Details
- **Files to create:**
  - `tools/docker-build/Cargo.toml` -- copy of `tools/cargo-build/Cargo.toml` with `name` changed to `"docker-build"`. All other fields remain identical.
- **Files to modify:**
  - `Cargo.toml` (workspace root) -- add `"tools/docker-build"` to the `[workspace] members` array, placed alphabetically near `"tools/cargo-build"`.
- No Rust source files are created in this task. The crate will not compile until `src/main.rs` is added by the blocking task.

## Dependencies
- Blocked by: None
- Blocking: "Implement `DockerBuildTool` struct and handler", "Write `main.rs`", "Write integration tests"

## Risks & Edge Cases
- **Workspace resolution failure**: If the workspace root `Cargo.toml` is not updated, `cargo build` will not find the new crate. Mitigation: always update the members list as part of this task.
- **Path dependency correctness**: The relative path `../../crates/mcp-tool-harness` assumes the crate lives at `tools/docker-build/`. If the directory is placed elsewhere, the path will break. Mitigation: verify the directory structure matches `tools/cargo-build/`.
- **Incomplete task**: Because no `src/main.rs` exists yet, `cargo check -p docker-build` will fail after this task alone. This is expected and resolved by the blocking tasks.

## Verification
- `cat tools/docker-build/Cargo.toml` shows the correct manifest with `name = "docker-build"` and all required dependencies.
- `grep "tools/docker-build" Cargo.toml` confirms the workspace root includes the new member.
- `cargo metadata --no-deps` lists `docker-build` as a workspace member (will only fully succeed once `src/main.rs` exists from the blocking task).
