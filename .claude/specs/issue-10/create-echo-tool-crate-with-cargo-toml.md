# Spec: Create `tools/echo-tool/` crate with Cargo.toml

> From: .claude/tasks/issue-10.md

## Objective

Scaffold the `tools/echo-tool/` crate by creating its directory structure and `Cargo.toml` manifest. This is the first tool crate in the `tools/` directory and establishes the pattern for all future MCP tool implementations. The crate brings in `rmcp` as a dependency for the first time in the workspace, enabling MCP server functionality over stdio transport.

## Current State

- The workspace root `Cargo.toml` lists five members under `crates/`:
  ```toml
  [workspace]
  resolver = "2"
  members = [
      "crates/agent-sdk",
      "crates/skill-loader",
      "crates/tool-registry",
      "crates/agent-runtime",
      "crates/orchestrator",
  ]
  ```
- The `tools/` directory exists but is empty aside from a `.gitkeep` placeholder.
- No `tools/echo-tool/` directory or any tool crate exists yet.
- All existing crates use `edition = "2024"` and `version = "0.1.0"`.
- The `tool-registry` crate is a stub (`pub struct ToolRegistry;`) with no dependencies. Registration of echo-tool in the registry is deferred to a later issue.
- `rmcp` is not yet used anywhere in the workspace; this task introduces it.

## Requirements

- Create the directory `tools/echo-tool/src/` (the `src/` subdirectory must exist so that `cargo check` can find the crate root, even though `main.rs` is not created by this task).
- Create `tools/echo-tool/Cargo.toml` with the exact contents specified below.
- The `[package]` section must have:
  - `name = "echo-tool"`
  - `version = "0.1.0"`
  - `edition = "2024"`
- The `[dependencies]` section must contain exactly these six entries (no more, no less):
  - `rmcp = { version = "1", features = ["transport-io", "server"] }`
  - `tokio = { version = "1", features = ["macros", "rt", "io-std"] }`
  - `serde = { version = "1", features = ["derive"] }`
  - `serde_json = "1"`
  - `tracing = "0.1"`
  - `tracing-subscriber = { version = "0.3", features = ["env-filter"] }`
- The `[dev-dependencies]` section must contain exactly:
  - `tokio = { version = "1", features = ["macros", "rt"] }`
- `clap` must NOT be included (intentionally omitted per CLAUDE.md to keep dependencies minimal).
- No path dependencies to other workspace crates (echo-tool is a standalone binary).
- A placeholder `tools/echo-tool/src/main.rs` must be created with a minimal `fn main() {}` so that `cargo check` succeeds once the crate is added to the workspace (the real implementation is a separate task).

## Implementation Details

### Files to create

1. **`tools/echo-tool/Cargo.toml`** -- the crate manifest with the following content:

   ```toml
   [package]
   name = "echo-tool"
   version = "0.1.0"
   edition = "2024"

   [dependencies]
   rmcp = { version = "1", features = ["transport-io", "server"] }
   tokio = { version = "1", features = ["macros", "rt", "io-std"] }
   serde = { version = "1", features = ["derive"] }
   serde_json = "1"
   tracing = "0.1"
   tracing-subscriber = { version = "0.3", features = ["env-filter"] }

   [dev-dependencies]
   tokio = { version = "1", features = ["macros", "rt"] }
   ```

2. **`tools/echo-tool/src/main.rs`** -- minimal placeholder so the crate compiles:

   ```rust
   fn main() {}
   ```

### Files NOT modified by this task

- `Cargo.toml` (root workspace) -- adding the crate to workspace members is a separate task ("Add `tools/echo-tool` to workspace members").

### Key design decisions

- **`tokio` feature split:** Production code needs `macros`, `rt`, and `io-std` (for stdin/stdout access in the MCP stdio transport). Dev-dependencies add `macros` and `rt` for `#[tokio::test]`. Cargo unions features when building tests, so this is the correct idiomatic pattern.
- **`rmcp` features:** `transport-io` provides `rmcp::transport::stdio()` for the stdio transport. `server` provides `ServerHandler`, `#[tool_router]`, and server-side MCP types. The default `macros` and `base64` features are included implicitly.
- **`tracing` + `tracing-subscriber`:** MCP servers using stdio transport must never write to stdout (it is the transport channel). Logging is directed to stderr via `tracing_subscriber` with `env-filter` for `RUST_LOG` control.
- **Standalone binary:** Unlike crates under `crates/` which are libraries, echo-tool is a binary crate with no path dependencies to other workspace crates. It will be a reference implementation that tool authors can copy.

## Dependencies

- Blocked by: Nothing (Group 1 task, no predecessors)
- Blocking: "Implement echo tool server", "Write unit tests for echo tool logic", "Write integration test for MCP server round-trip" (all in Groups 2-3 require this crate to exist)

## Risks & Edge Cases

- **`rmcp` version availability:** The task specifies `rmcp = { version = "1", ... }`. If the published version on crates.io does not match `1.x`, the dependency resolution will fail. Mitigation: verify `rmcp` version availability during implementation and adjust if needed (e.g., use `0.1` if v1 is not yet published). Document any deviation.
- **Edition 2024 compatibility:** All specified dependency versions must be compatible with Rust edition 2024 (which requires rustc 1.85+). The dependencies listed are all widely-used crates that support recent editions.
- **`io-std` tokio feature:** The `io-std` feature for tokio enables `tokio::io::stdin()` and `tokio::io::stdout()`. Confirm this feature name is correct (it is -- it has been stable since tokio 1.0).
- **Placeholder main.rs:** The placeholder `fn main() {}` does not use any of the declared dependencies, which will produce unused-dependency warnings from `cargo clippy`. This is acceptable since the real implementation replaces it in the next task. If warnings are problematic, the placeholder can add `use` statements, but this is not recommended since it couples the placeholder to API details that may change.
- **Workspace root not updated:** This task intentionally does NOT add the crate to the workspace `members` list. A separate task handles that. Until both tasks complete, `cargo build` from the workspace root will not build echo-tool. This is by design -- the tasks are in the same group (Group 1) and can be done in parallel.

## Verification

- `tools/echo-tool/Cargo.toml` exists and contains the exact `[package]`, `[dependencies]`, and `[dev-dependencies]` sections specified above.
- `tools/echo-tool/src/main.rs` exists and contains a valid `fn main() {}`.
- The directory structure is `tools/echo-tool/src/main.rs` and `tools/echo-tool/Cargo.toml` (no extra files).
- After the companion task "Add `tools/echo-tool` to workspace members" completes, `cargo check -p echo-tool` succeeds without errors.
- `clap` is NOT present in any dependency section.
- No path dependencies to other workspace crates are present.
