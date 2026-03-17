# Spec: Add `rig-core` and `rmcp` dependencies to agent-runtime Cargo.toml

> From: .claude/tasks/issue-9.md

## Objective
Add the external crates `rig-core` and `rmcp`, plus workspace-internal path dependencies (`tool-registry`, `agent-sdk`, `skill-loader`), and `tokio` to the `agent-runtime` crate so that subsequent tasks can implement the MCP-to-rig-core bridge and skeleton startup flow.

## Current State
`crates/agent-runtime/Cargo.toml` is a bare scaffold with no dependencies:

```toml
[package]
name = "agent-runtime"
version = "0.1.0"
edition = "2024"

[dependencies]
```

The workspace root `Cargo.toml` already lists `agent-runtime` as a member alongside `agent-sdk`, `skill-loader`, `tool-registry`, and `orchestrator`. Sibling crates use path dependencies (e.g., `agent-sdk = { path = "../agent-sdk" }`) and pin versions for external crates (e.g., `serde = { version = "1", features = ["derive"] }`).

## Requirements
- Add `rig-core = { version = "0.32", features = ["rmcp"] }` to `[dependencies]`. The `rmcp` feature gate enables `McpTool` and `AgentBuilder::rmcp_tools()`.
- Add `rmcp = { version = "0.16", features = ["client", "transport-async-rw"] }` to `[dependencies]`. Must be pinned to `0.16` because `rig-core 0.32` depends on `rmcp ^0.16`; using rmcp 1.x would cause a Cargo version conflict.
- Add `tool-registry = { path = "../tool-registry" }` to `[dependencies]`.
- Add `agent-sdk = { path = "../agent-sdk" }` to `[dependencies]`.
- Add `skill-loader = { path = "../skill-loader" }` to `[dependencies]`.
- Add `tokio = { version = "1", features = ["full"] }` to `[dependencies]`.
- After the change, `cargo check -p agent-runtime` must succeed with no errors.
- `cargo clippy -p agent-runtime` must produce no warnings.
- All existing workspace tests (`cargo test`) must continue to pass.

## Implementation Details
- **File to modify:** `crates/agent-runtime/Cargo.toml`
- **Changes:** Append six dependency lines under the existing `[dependencies]` section. No new sections, no dev-dependencies, no build-dependencies.
- **No code changes** are required in this task -- only the manifest file is touched.

### Expected final state of `crates/agent-runtime/Cargo.toml`
```toml
[package]
name = "agent-runtime"
version = "0.1.0"
edition = "2024"

[dependencies]
rig-core = { version = "0.32", features = ["rmcp"] }
rmcp = { version = "0.16", features = ["client", "transport-async-rw"] }
tool-registry = { path = "../tool-registry" }
agent-sdk = { path = "../agent-sdk" }
skill-loader = { path = "../skill-loader" }
tokio = { version = "1", features = ["full"] }
```

## Dependencies
- Blocked by: None (this is a manifest-only change; the crates it depends on already exist in the workspace)
- Blocking: "Implement MCP-to-rig-core bridge in agent-runtime"

## Risks & Edge Cases
- **Version conflict between `rig-core` and `rmcp`:** `rig-core 0.32` depends on `rmcp ^0.16`. If `rmcp` is specified as any other major version, Cargo will fail to resolve. Mitigation: pin `rmcp` to `0.16` as specified.
- **`rig-core 0.32` or `rmcp 0.16` not published:** If these exact versions are not available on crates.io at implementation time, the build will fail. Mitigation: verify with `cargo search rig-core` and `cargo search rmcp` before committing, and adjust patch version if needed while staying within the same semver range.
- **`edition = "2024"` compatibility:** The crate uses Rust edition 2024. Ensure the added dependencies compile under this edition (they should, as they target stable Rust).
- **Feature flag typos:** A misspelled feature name will cause a hard build error. Double-check feature names against each crate's published `Cargo.toml`.

## Verification
- `cargo check -p agent-runtime` exits with status 0 (no compile errors).
- `cargo clippy -p agent-runtime` exits with status 0 and no warnings.
- `cargo test` across the full workspace passes (no regressions).
- `cargo metadata --format-version=1 | jq '.packages[] | select(.name == "agent-runtime") | .dependencies[].name'` lists `rig-core`, `rmcp`, `tool-registry`, `agent-sdk`, `skill-loader`, and `tokio`.
