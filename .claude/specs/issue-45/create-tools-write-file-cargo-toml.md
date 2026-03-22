# Spec: Create `tools/write-file/Cargo.toml`

> From: .claude/tasks/issue-45.md

## Objective
Create the Cargo manifest for the `write-file` MCP tool crate. This establishes the package metadata and dependency set so that subsequent tasks can add source code and integrate the crate into the workspace.

## Current State
`tools/echo-tool/Cargo.toml` serves as the reference template. It declares:
- Package: `echo-tool`, version `0.1.0`, edition `2024`
- Dependencies: `rmcp` (with `transport-io`, `server`, `macros` features), `tokio`, `serde` (with `derive`), `serde_json`, `tracing`, `tracing-subscriber` (with `env-filter`)
- Dev-dependencies: `tokio` (with `macros`, `rt`, `rt-multi-thread`), `rmcp` (with `client`, `transport-child-process`), `serde_json`

The `tools/write-file/` directory does not yet exist.

## Requirements
- The file must be located at `tools/write-file/Cargo.toml`.
- The package name must be `write-file`.
- Version must be `0.1.0` and edition must be `2024`, matching the existing tool convention.
- The `[dependencies]` section must list exactly the same crates and feature sets as `tools/echo-tool/Cargo.toml`: `rmcp`, `tokio`, `serde`, `serde_json`, `tracing`, `tracing-subscriber`.
- The `[dev-dependencies]` section must list exactly the same crates and feature sets as `tools/echo-tool/Cargo.toml`: `tokio`, `rmcp`, `serde_json`.
- No additional dependencies may be added (file I/O uses `std::fs`).
- The file must be valid TOML parseable by Cargo.

## Implementation Details
- **File to create:** `tools/write-file/Cargo.toml`
  - Copy `tools/echo-tool/Cargo.toml` verbatim.
  - Change `name = "echo-tool"` to `name = "write-file"`.
  - Leave all other fields unchanged.

## Dependencies
- Blocked by: none
- Blocking: "Add `tools/write-file` to workspace members"

## Risks & Edge Cases
- If the edition or dependency versions in `echo-tool` change before this task runs, the copy will be stale. Mitigate by reading the reference file at implementation time rather than hard-coding values.
- The crate will not compile until source files (`src/main.rs` or `src/lib.rs`) are added by a follow-up task.

## Verification
- `cat tools/write-file/Cargo.toml` shows the correct package name (`write-file`) and matching dependencies.
- `cargo metadata --manifest-path tools/write-file/Cargo.toml` parses without errors (note: workspace membership is not yet required at this step, so `--no-deps` may be needed).
- Diff against `tools/echo-tool/Cargo.toml` shows only the package name changed.
