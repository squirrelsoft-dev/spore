# Spec: Create `crates/mcp-test-utils` crate with `spawn_mcp_client!` macro

> From: Issue #56

## Objective

Create a new workspace member crate `crates/mcp-test-utils` that provides a declarative macro `spawn_mcp_client!` for spawning MCP tool binaries as child processes and connecting an MCP client. This eliminates the boilerplate `spawn_*_client()` helper that is currently duplicated in each tool's integration tests (e.g., `spawn_echo_client()` in `tools/echo-tool/tests/echo_server_test.rs`). The macro must be a `macro_rules!` macro because it wraps `env!("CARGO_BIN_EXE_...")`, which resolves at compile time in the calling crate's context.

## Current State

- Each tool crate defines its own async helper to spawn a child process and connect an MCP client. For example, `tools/echo-tool/tests/echo_server_test.rs` contains:
  ```rust
  async fn spawn_echo_client() -> RunningService<RoleClient, ()> {
      let transport = TokioChildProcess::new(Command::new(env!("CARGO_BIN_EXE_echo-tool")))
          .expect("failed to spawn echo-tool");
      ().serve(transport)
          .await
          .expect("failed to connect to echo-tool server")
  }
  ```
- This pattern is identical across tool crates except for the binary name string passed to `env!()`.
- The workspace root `Cargo.toml` currently has these members:
  ```toml
  members = [
      "crates/agent-sdk",
      "crates/skill-loader",
      "crates/tool-registry",
      "crates/agent-runtime",
      "crates/orchestrator",
      "tools/echo-tool",
      "tools/read-file",
      "tools/write-file",
      "tools/validate-skill",
  ]
  ```
- No shared test utility crate exists in the workspace.
- Tool crates each independently declare `rmcp` with `client` and `transport-child-process` features in their `[dev-dependencies]`.

## Requirements

- Create the directory `crates/mcp-test-utils/src/`.
- Create `crates/mcp-test-utils/Cargo.toml` with the fields specified in Implementation Details.
- Create `crates/mcp-test-utils/src/lib.rs` containing the `spawn_mcp_client!` macro.
- Add `"crates/mcp-test-utils"` to the `members` list in the root `Cargo.toml`.
- The macro `spawn_mcp_client!` must:
  - Accept a single expression that evaluates to a binary path (intended to be used with `env!("CARGO_BIN_EXE_<name>")`).
  - Return `RunningService<RoleClient, ()>` (from the `rmcp` crate).
  - Be an `async` block internally (the caller must `.await` the result).
  - Panic with a descriptive message if spawning or connecting fails (this is test-only code).
- The crate must re-export the `rmcp` types needed by callers so they do not need to add `rmcp` to their own dev-dependencies just for the return type: `RunningService`, `RoleClient`.
- Dependencies must be limited to: `rmcp` (with `client` and `transport-child-process` features) and `tokio` (with `process` feature).
- No proc-macro crate is needed; a `macro_rules!` declarative macro is sufficient.

## Implementation Details

### Files to create

1. **`crates/mcp-test-utils/Cargo.toml`**

   ```toml
   [package]
   name = "mcp-test-utils"
   version = "0.1.0"
   edition = "2024"

   [dependencies]
   rmcp = { version = "1", features = ["client", "transport-child-process"] }
   tokio = { version = "1", features = ["process"] }
   ```

2. **`crates/mcp-test-utils/src/lib.rs`**

   The file must:
   - Re-export `rmcp::service::RunningService` and `rmcp::RoleClient` so downstream test code can reference the return type without adding `rmcp` as a direct dependency.
   - Re-export `rmcp::ServiceExt` so callers have the `.serve()` method in scope.
   - Define the `spawn_mcp_client!` macro that expands to an async block equivalent to:
     ```rust
     async {
         let transport = rmcp::transport::TokioChildProcess::new(
             tokio::process::Command::new($bin_path)
         ).expect("failed to spawn MCP server");
         <() as rmcp::ServiceExt>::serve((), transport)
             .await
             .expect("failed to connect to MCP server")
     }
     ```
   - The macro must use fully qualified paths (e.g., `$crate::...` or `::rmcp::...`) to avoid import conflicts in the calling crate.

### Files to modify

3. **`Cargo.toml`** (workspace root)

   Add `"crates/mcp-test-utils"` to the `members` array. Place it alphabetically among the `crates/` entries (after `"crates/agent-sdk"` and before `"crates/orchestrator"`, or at the end of the crates group).

### Key design decisions

- **`macro_rules!` not `proc_macro`:** The entire reason this must be a macro is that `env!("CARGO_BIN_EXE_...")` is resolved at compile time in the crate that invokes it. A regular function would resolve `env!()` in the `mcp-test-utils` crate itself, which does not declare any binary targets. A `macro_rules!` macro expands in the caller's context, so `env!()` resolves correctly.
- **Re-exports:** By re-exporting `RunningService`, `RoleClient`, and `ServiceExt`, consuming crates only need `mcp-test-utils` in their `[dev-dependencies]`, not `rmcp` directly (unless they need additional `rmcp` types for test assertions like `CallToolRequestParams`).
- **Panic on failure:** Since this is test utility code, panicking with `.expect()` is the correct error handling strategy. Test failures will show the panic message.
- **Minimal tokio features:** Only `process` is needed for `tokio::process::Command`. The calling test crate will already have `rt` and `macros` for `#[tokio::test]`.

## Dependencies

- Blocked by: Nothing (this is a foundational test utility crate).
- Blocking: All test utility additions and test migrations that will replace per-crate `spawn_*_client()` helpers with the shared macro.

## Risks & Edge Cases

- **`env!()` expansion context:** The correctness of this approach depends on `macro_rules!` expanding in the caller's crate context. This is standard Rust behavior for declarative macros and is well-documented. The `env!("CARGO_BIN_EXE_...")` environment variable is set by Cargo during `cargo test` for binary targets in the same package; the calling crate must declare the binary as a dependency or be in the same package for this to work.
- **Feature unification:** Adding `rmcp` with `client` and `transport-child-process` features may pull in additional transitive dependencies. Since this crate is only used in `[dev-dependencies]`, these will not affect production binary sizes.
- **Edition 2024:** Consistent with all other workspace crates. Requires rustc 1.85+.
- **Macro hygiene:** The macro must use fully qualified paths to avoid name collisions. Using `::rmcp::` and `::tokio::` prefixes (or `$crate` for re-exports) ensures the macro works regardless of what the caller has imported.

## Verification

- `crates/mcp-test-utils/Cargo.toml` exists with the specified `[package]` and `[dependencies]` sections.
- `crates/mcp-test-utils/src/lib.rs` exists and defines the `spawn_mcp_client!` macro with re-exports.
- The root `Cargo.toml` includes `"crates/mcp-test-utils"` in the workspace members list.
- `cargo check -p mcp-test-utils` succeeds without errors.
- `cargo clippy -p mcp-test-utils` succeeds without warnings.
- `cargo test -p mcp-test-utils` succeeds (even if there are no tests yet, it should compile).
