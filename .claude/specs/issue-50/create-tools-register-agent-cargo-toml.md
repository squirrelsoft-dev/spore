# Spec: Create `tools/register-agent/Cargo.toml`

> From: .claude/tasks/issue-50.md

## Objective
Create the Cargo.toml manifest for the `register-agent` MCP tool crate. This is the foundational build configuration that all other register-agent tasks depend on (struct/handler implementation, main.rs, and integration tests).

## Current State
- `tools/docker-push/Cargo.toml` serves as the reference template for new tool crates.
- The workspace root `Cargo.toml` defines all workspace members; `tools/register-agent` is not yet listed.
- All existing tool crates follow an identical dependency pattern: `mcp-tool-harness`, `rmcp`, `tokio`, `serde`, `serde_json` for runtime; `mcp-test-utils`, `tokio` (with `rt-multi-thread`), `rmcp` (with `client`), and `serde_json` for dev/test.

## Requirements
- Create `tools/register-agent/Cargo.toml` with `name = "register-agent"`, `version = "0.1.0"`, `edition = "2024"`.
- Include the following `[dependencies]`:
  - `mcp-tool-harness = { path = "../../crates/mcp-tool-harness" }`
  - `rmcp = { version = "1", features = ["transport-io", "server", "macros"] }`
  - `tokio = { version = "1", features = ["macros", "rt", "io-std"] }`
  - `serde = { version = "1", features = ["derive"] }`
  - `serde_json = "1"`
  - `reqwest = { version = "0.13", features = ["json"] }`
- Include the following `[dev-dependencies]` (identical to `docker-push`):
  - `mcp-test-utils = { path = "../../crates/mcp-test-utils" }`
  - `tokio = { version = "1", features = ["macros", "rt", "rt-multi-thread"] }`
  - `rmcp = { version = "1", features = ["client", "transport-child-process"] }`
  - `serde_json = "1"`
- Add `"tools/register-agent"` to the `[workspace] members` list in the root `Cargo.toml`.

## Implementation Details
- **File to create:** `tools/register-agent/Cargo.toml`
  - Copy the structure from `tools/docker-push/Cargo.toml`.
  - Change `name` to `"register-agent"`.
  - Add `reqwest = { version = "0.13", features = ["json"] }` to `[dependencies]`.
  - Keep all other fields and versions unchanged.
- **File to modify:** `Cargo.toml` (workspace root)
  - Append `"tools/register-agent"` to the `members` array.

## Dependencies
- Blocked by: nothing
- Blocking: "Implement RegisterAgentTool struct and handler", "Write main.rs", "Write integration tests"

## Risks & Edge Cases
- `reqwest 0.13` must be compatible with the project's existing `tokio 1.x` and `rustc` edition 2024. Verify that `cargo check -p register-agent` succeeds after creation.
- The workspace root `Cargo.toml` members list must include the new crate or `cargo` commands will not discover it.
- A `src/` directory with at least a `lib.rs` or `main.rs` stub is required for `cargo check` to pass, but that file is the responsibility of downstream tasks ("Write main.rs"). The verification step for this task should confirm the manifest parses correctly rather than requiring a full build.

## Verification
- `cargo metadata -p register-agent --no-deps` exits successfully and shows the correct package name, version, and dependency list.
- The root `Cargo.toml` `[workspace] members` array includes `"tools/register-agent"`.
- The `[dependencies]` section contains exactly the six crates listed above with correct versions and features.
- The `[dev-dependencies]` section matches the `docker-push` template exactly.
