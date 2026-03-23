# Spec: Create `tools/cargo-build/Cargo.toml`

> From: .claude/tasks/issue-47.md

## Objective
Create the Cargo manifest for the `cargo-build` MCP tool crate, establishing it as a new tool package within the workspace. This is the foundational file that enables all subsequent implementation work for the cargo-build tool.

## Current State
The `tools/echo-tool/Cargo.toml` serves as the canonical template for all tool crates. It declares:
- Package metadata: name, version `0.1.0`, edition `2024`
- Runtime dependencies: `mcp-tool-harness` (path dep), `rmcp` (with `transport-io`, `server`, `macros`), `tokio` (with `macros`, `rt`, `io-std`), `serde` (with `derive`), `serde_json`
- Dev dependencies: `mcp-test-utils` (path dep), `tokio` (with `macros`, `rt`, `rt-multi-thread`), `rmcp` (with `client`, `transport-child-process`), `serde_json`

No `tools/cargo-build/` directory exists yet.

## Requirements
- The file must be a valid Cargo.toml that `cargo check -p cargo-build` accepts
- Package name must be `cargo-build`
- Version must be `0.1.0` and edition must be `2024`
- Dependencies must exactly match the echo-tool pattern (same crates, same features, same version constraints)
- Path dependencies must use `../../crates/` relative paths for `mcp-tool-harness` and `mcp-test-utils`
- No additional dependencies beyond what echo-tool declares

## Implementation Details
- **File to create:** `tools/cargo-build/Cargo.toml`
- Copy `tools/echo-tool/Cargo.toml` verbatim and change only `name = "echo-tool"` to `name = "cargo-build"`
- All dependency versions, features, and paths remain identical

### Expected file content

```toml
[package]
name = "cargo-build"
version = "0.1.0"
edition = "2024"

[dependencies]
mcp-tool-harness = { path = "../../crates/mcp-tool-harness" }
rmcp = { version = "1", features = ["transport-io", "server", "macros"] }
tokio = { version = "1", features = ["macros", "rt", "io-std"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"

[dev-dependencies]
mcp-test-utils = { path = "../../crates/mcp-test-utils" }
tokio = { version = "1", features = ["macros", "rt", "rt-multi-thread"] }
rmcp = { version = "1", features = ["client", "transport-child-process"] }
serde_json = "1"
```

## Dependencies
- Blocked by: none (this is the first task for the cargo-build tool)
- Blocking: "Implement `CargoBuildTool` struct and handler", "Write `main.rs`", "Write integration test"

## Risks & Edge Cases
- The workspace root `Cargo.toml` may need to include `tools/cargo-build` in its `[workspace] members` list for the crate to be recognized. Verify whether this is manual or auto-discovered via glob.
- The crate will not compile until `src/main.rs` (or `src/lib.rs`) exists, so `cargo check -p cargo-build` will fail until the next task is complete. Validation should confirm the manifest parses correctly rather than requiring a full build.

## Verification
- Run `cargo verify-project --manifest-path tools/cargo-build/Cargo.toml` to confirm the manifest is syntactically valid (this does not require source files to exist)
- Confirm the file is byte-identical to `tools/echo-tool/Cargo.toml` except for the package name
- Confirm the workspace root includes the new crate (check `[workspace] members` in the root `Cargo.toml`)
