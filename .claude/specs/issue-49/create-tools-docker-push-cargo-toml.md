# Spec: Create `tools/docker-push/Cargo.toml`

> From: .claude/tasks/issue-49.md

## Objective

Scaffold the `tools/docker-push/` crate by creating its `Cargo.toml` manifest. This crate will be a standalone Rust MCP server binary that pushes a tagged Docker image to a container registry. The manifest follows the established pattern from `tools/cargo-build/Cargo.toml`, changing only the package name.

## Current State

- The workspace root `Cargo.toml` lists the following members:
  ```toml
  [workspace]
  resolver = "2"
  members = [
      "crates/agent-sdk",
      "crates/skill-loader",
      "crates/tool-registry",
      "crates/agent-runtime",
      "crates/mcp-tool-harness",
      "crates/orchestrator",
      "crates/mcp-test-utils",
      "tools/echo-tool",
      "tools/read-file",
      "tools/write-file",
      "tools/validate-skill",
      "tools/cargo-build",
  ]
  ```
- The `tools/docker-push/` directory does not yet exist.
- `tools/cargo-build/Cargo.toml` is the reference pattern. It uses `edition = "2024"`, `version = "0.1.0"`, and depends on `mcp-tool-harness` (a workspace-local crate), `rmcp`, `tokio`, `serde`, and `serde_json`.
- All existing tool crates use the same dependency pattern: `mcp-tool-harness` via path, `rmcp` with `transport-io`/`server`/`macros` features, `tokio` with `macros`/`rt`/`io-std`, `serde` with `derive`, and `serde_json`.
- All existing tool crates use the same dev-dependency pattern: `mcp-test-utils` via path, `tokio` with `macros`/`rt`/`rt-multi-thread`, `rmcp` with `client`/`transport-child-process`, and `serde_json`.

## Requirements

- Create the directory `tools/docker-push/src/` so that `cargo check` can locate the crate root once the workspace member is added.
- Create `tools/docker-push/Cargo.toml` with the exact contents specified below.
- The `[package]` section must have:
  - `name = "docker-push"`
  - `version = "0.1.0"`
  - `edition = "2024"`
- The `[dependencies]` section must contain exactly these five entries (no more, no less):
  - `mcp-tool-harness = { path = "../../crates/mcp-tool-harness" }`
  - `rmcp = { version = "1", features = ["transport-io", "server", "macros"] }`
  - `tokio = { version = "1", features = ["macros", "rt", "io-std"] }`
  - `serde = { version = "1", features = ["derive"] }`
  - `serde_json = "1"`
- The `[dev-dependencies]` section must contain exactly these four entries:
  - `mcp-test-utils = { path = "../../crates/mcp-test-utils" }`
  - `tokio = { version = "1", features = ["macros", "rt", "rt-multi-thread"] }`
  - `rmcp = { version = "1", features = ["client", "transport-child-process"] }`
  - `serde_json = "1"`
- A placeholder `tools/docker-push/src/main.rs` must be created with a minimal `fn main() {}` so that `cargo check` succeeds once the crate is added to the workspace. The real implementation is a separate task.

## Implementation Details

### Files to create

1. **`tools/docker-push/Cargo.toml`** -- the crate manifest with the following content:

   ```toml
   [package]
   name = "docker-push"
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

2. **`tools/docker-push/src/main.rs`** -- minimal placeholder so the crate compiles:

   ```rust
   fn main() {}
   ```

### Files NOT modified by this task

- `Cargo.toml` (root workspace) -- adding the crate to workspace members is a separate task ("Add `tools/docker-push` to workspace `Cargo.toml`").

### Key design decisions

- **Exact copy of `cargo-build` pattern:** The `Cargo.toml` is intentionally identical to `tools/cargo-build/Cargo.toml` except for the package name. This ensures consistency across all tool crates and reduces cognitive overhead for contributors.
- **`mcp-tool-harness` path dependency:** Unlike the original `echo-tool` which was standalone, newer tool crates depend on `mcp-tool-harness` for shared MCP server boilerplate (`serve_stdio_tool`). The path `../../crates/mcp-tool-harness` is correct relative to `tools/docker-push/`.
- **`rmcp` features split:** Production dependencies use `transport-io`, `server`, and `macros` for the stdio MCP server. Dev-dependencies use `client` and `transport-child-process` for spawning the tool binary and communicating with it in integration tests.
- **`tokio` feature split:** Production code needs `macros`, `rt`, and `io-std` (for stdin/stdout in the MCP stdio transport). Dev-dependencies add `rt-multi-thread` for `#[tokio::test(flavor = "multi_thread")]` in integration tests. Cargo unions features when building tests, so this is the correct idiomatic pattern.
- **`mcp-test-utils` dev-dependency:** Provides `spawn_mcp_client!` macro and `assert_single_tool` helper used by integration tests.
- **No `schemars` dependency:** Input validation uses `schemars::JsonSchema` derive, but this is re-exported through `rmcp`'s `macros` feature, so no explicit `schemars` dependency is needed.

## Dependencies

- Blocked by: Nothing (Group 1 task, no predecessors)
- Blocking: "Implement `DockerPushTool` struct and handler", "Write `main.rs`", "Write integration tests" (all require this crate manifest to exist)

## Risks & Edge Cases

- **Path dependency correctness:** The relative paths `../../crates/mcp-tool-harness` and `../../crates/mcp-test-utils` assume the crate lives at `tools/docker-push/`. If the directory structure changes, these paths will break. This is the same pattern used by all existing tool crates, so the risk is low.
- **Placeholder `main.rs` warnings:** The placeholder `fn main() {}` does not use any of the declared dependencies, which will produce unused-dependency warnings from `cargo clippy`. This is acceptable since the real implementation replaces it in the next task ("Write `main.rs`").
- **Workspace root not updated:** This task intentionally does NOT add the crate to the workspace `members` list. A separate task handles that. Until both tasks complete, `cargo build` from the workspace root will not build `docker-push`. This is by design -- the two tasks are in the same group (Group 1) and can be done in parallel.
- **Dependency version alignment:** All dependency versions (`rmcp = "1"`, `tokio = "1"`, `serde = "1"`, `serde_json = "1"`) match those used by `tools/cargo-build/Cargo.toml` exactly. Since these already resolve successfully in the workspace, no version conflicts are expected.

## Verification

- `tools/docker-push/Cargo.toml` exists and contains the exact `[package]`, `[dependencies]`, and `[dev-dependencies]` sections specified above.
- `tools/docker-push/src/main.rs` exists and contains a valid `fn main() {}`.
- The directory structure is `tools/docker-push/src/main.rs` and `tools/docker-push/Cargo.toml` (no extra files).
- After the companion task "Add `tools/docker-push` to workspace `Cargo.toml`" completes, `cargo check -p docker-push` succeeds without errors.
- The `[dependencies]` section has exactly five entries matching `tools/cargo-build/Cargo.toml`.
- The `[dev-dependencies]` section has exactly four entries matching `tools/cargo-build/Cargo.toml`.
- No dependencies are added that are not present in the `cargo-build` reference.
