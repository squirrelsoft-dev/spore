# Spec: Add `uuid` and `serde_json` dependencies to `agent-sdk/Cargo.toml`

> From: .claude/tasks/issue-4.md

## Objective
Add the `uuid` and `serde_json` crates as direct dependencies of the `agent-sdk` crate so that downstream types (`AgentRequest`, `AgentResponse`, `ToolCallRecord`) can use `uuid::Uuid` and `serde_json::Value` in their public API surfaces.

## Current State
`crates/agent-sdk/Cargo.toml` currently declares:

```toml
[package]
name = "agent-sdk"
version = "0.1.0"
edition = "2024"

[dependencies]
serde = { version = "1", features = ["derive"] }
schemars = { version = "0.8", features = ["derive"] }

[dev-dependencies]
serde_yaml = "0.9"
```

`serde_json` is already a transitive dependency (pulled in by `schemars`), but it is not listed as a direct dependency. `uuid` is not present at all. Both `serde` and `schemars` were added in issue #2.

## Requirements
- Add `uuid = { version = "1", features = ["v4", "serde"] }` to `[dependencies]`.
  - The `v4` feature is required for `Uuid::new_v4()` used in `AgentRequest::new()`.
  - The `serde` feature is required so `Uuid` implements `Serialize`/`Deserialize`.
- Add `serde_json = "1"` to `[dependencies]`.
  - Required because `serde_json::Value` appears in public struct fields (`AgentRequest::context`, `AgentResponse::output`, `ToolCallRecord::input`, `ToolCallRecord::output`).
  - A direct dependency is necessary even though `schemars` already pulls in `serde_json` transitively; relying on transitive dependencies for public API types is fragile and violates Rust best practices.
- Existing dependencies (`serde`, `schemars`) and dev-dependencies (`serde_yaml`) must remain unchanged.
- The crate must continue to compile cleanly after the change (`cargo check -p agent-sdk`).

## Implementation Details
- **File to modify**: `crates/agent-sdk/Cargo.toml`
- **Change**: Append two lines to the `[dependencies]` section:
  ```toml
  uuid = { version = "1", features = ["v4", "serde"] }
  serde_json = "1"
  ```
- No Rust source files are created or modified in this task.

## Dependencies
- Blocked by: None (this is in Group 1, the first group)
- Blocking: All tasks in Group 2 (`ToolCallRecord`, `AgentError`, `HealthStatus`) and Group 3 (`AgentRequest`, `AgentResponse`), since those types use `uuid::Uuid` and `serde_json::Value`

## Risks & Edge Cases
- **Version conflict with transitive `serde_json`**: Minimal risk. Both this crate and `schemars` specify `serde_json = "1"`, so Cargo will unify them to a single version. No semver conflict is possible within the `1.x` range.
- **Feature flag completeness for `uuid`**: The `v4` feature pulls in the `getrandom` crate for random UUID generation. This works on all standard platforms (Linux, macOS, Windows). If the crate ever targets `no_std` or WASM without `wasm-bindgen`, `getrandom` would need a different configuration, but that is out of scope.
- **Edition 2024 compatibility**: Both `uuid 1.x` and `serde_json 1.x` are compatible with Rust edition 2024. No issues expected.

## Verification
- `cargo check -p agent-sdk` succeeds with no errors.
- `cargo clippy -p agent-sdk` produces no warnings.
- `cargo test -p agent-sdk` passes (existing tests from issue #2 remain green).
- Inspect `crates/agent-sdk/Cargo.toml` and confirm it contains exactly the four dependencies: `serde`, `schemars`, `uuid`, and `serde_json` with the specified versions and features.
