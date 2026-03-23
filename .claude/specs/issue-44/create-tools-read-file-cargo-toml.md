# Spec: Create `tools/read-file/Cargo.toml`

> From: .claude/tasks/issue-44.md

## Objective

Create the `Cargo.toml` manifest for the `read-file` MCP tool crate. This file defines the package name, edition, runtime dependencies, and dev-dependencies required to build the tool binary and run its integration tests. It is the first prerequisite for all other `read-file` implementation tasks.

## Current State

`tools/echo-tool/Cargo.toml` is the reference implementation. It uses:

- `[package]` with `name = "echo-tool"`, `version = "0.1.0"`, `edition = "2024"`
- `[dependencies]`: `rmcp` (features: `transport-io`, `server`, `macros`), `tokio` (features: `macros`, `rt`, `io-std`), `serde` (feature: `derive`), `serde_json`, `tracing`, `tracing-subscriber` (feature: `env-filter`)
- `[dev-dependencies]`: `tokio` (features: `macros`, `rt`, `rt-multi-thread`), `rmcp` (features: `client`, `transport-child-process`), `serde_json`

The root `Cargo.toml` workspace at `/Users/sbeardsley/Developer/squirrelsoft-dev/spore/Cargo.toml` currently lists `tools/echo-tool` in `members` but does not yet include `tools/read-file`. Adding `tools/read-file` to the workspace `members` is a separate task ("Add `tools/read-file` to workspace `Cargo.toml`") that runs in parallel with this one.

All dependencies used by `echo-tool` are already present in the workspace; no new third-party crates are needed.

## Requirements

- The file must be created at `tools/read-file/Cargo.toml`.
- `[package] name` must be `"read-file"` (the binary produced will be `read-file`; the env macro in integration tests will be `CARGO_BIN_EXE_read-file`).
- `[package] version` must be `"0.1.0"` and `edition` must be `"2024"`, matching the workspace convention.
- `[dependencies]` must include exactly:
  - `rmcp = { version = "1", features = ["transport-io", "server", "macros"] }`
  - `tokio = { version = "1", features = ["macros", "rt", "io-std"] }`
  - `serde = { version = "1", features = ["derive"] }`
  - `serde_json = "1"`
  - `tracing = "0.1"`
  - `tracing-subscriber = { version = "0.3", features = ["env-filter"] }`
- `[dev-dependencies]` must include exactly:
  - `tokio = { version = "1", features = ["macros", "rt", "rt-multi-thread"] }`
  - `rmcp = { version = "1", features = ["client", "transport-child-process"] }`
  - `serde_json = "1"`
- No additional dependencies may be added beyond those listed above.

## Implementation Details

- File to create: `tools/read-file/Cargo.toml`
- The file is a direct copy of `tools/echo-tool/Cargo.toml` with only `name = "echo-tool"` changed to `name = "read-file"`. No other lines change.
- The `rt-multi-thread` feature in `[dev-dependencies]` for `tokio` is required because the integration tests use `#[tokio::test(flavor = "multi_thread", worker_threads = 2)]`.
- The `client` and `transport-child-process` features in `[dev-dependencies]` for `rmcp` are required because the integration tests spawn the `read-file` binary as a child process and communicate with it over stdio via an MCP client.
- No `[[bin]]` section is needed; Cargo infers the binary from `src/main.rs` and uses the package name as the binary name.

## Dependencies

- Blocked by: None
- Blocking: "Implement `ReadFileTool` struct and handler in `src/read_file.rs`", "Write `src/main.rs`", "Write integration test in `tests/read_file_server_test.rs`"

## Risks & Edge Cases

- If `tools/read-file` is not added to the workspace `members` in the root `Cargo.toml` (a parallel task), `cargo build -p read-file` and `cargo test -p read-file` will fail with an unknown package error. This task and the workspace membership task must both complete before any build or test commands are run.
- Version constraints (`rmcp = "1"`, `tokio = "1"`, etc.) must stay in sync with what `echo-tool` uses. Drifting versions could cause duplicate dependency resolution in the workspace.
- The `edition = "2024"` field requires a Rust toolchain that supports the 2024 edition. This is already established by `echo-tool` using the same edition, so no toolchain change is needed.

## Verification

- `tools/read-file/Cargo.toml` exists and its content matches `tools/echo-tool/Cargo.toml` with only the `name` field changed to `"read-file"`.
- After `tools/read-file` is added to the workspace `members`, running `cargo check -p read-file` succeeds (once `src/main.rs` exists).
- Running `cargo metadata --manifest-path tools/read-file/Cargo.toml` resolves without errors and lists the expected dependencies.
