# Spec: Create `crates/mcp-tool-harness` crate with `serve_stdio_tool` function

> From: .claude/tasks/issue-56.md

## Objective

Create a new workspace member crate at `crates/mcp-tool-harness` that extracts the common MCP stdio server boilerplate into a reusable library function. Today, every tool binary (echo-tool, read-file, write-file, validate-skill) duplicates the same tracing setup, stdio transport initialization, and service lifecycle management in its `main.rs`. This crate provides a single `serve_stdio_tool` function that encapsulates all of that, so each tool's `main.rs` reduces to a one-liner call.

## Current State

- The workspace root `Cargo.toml` lists these members:
  ```toml
  [workspace]
  resolver = "2"
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
- No `crates/mcp-tool-harness` directory exists yet.
- Each tool binary contains duplicated boilerplate in `main.rs` that follows this pattern (taken from `tools/echo-tool/src/main.rs`):
  ```rust
  tracing_subscriber::fmt()
      .with_env_filter(EnvFilter::from_default_env().add_directive(tracing::Level::DEBUG.into()))
      .with_writer(std::io::stderr)
      .with_ansi(false)
      .init();

  tracing::info!("Starting echo-tool MCP server");

  let service = EchoTool::new()
      .serve(rmcp::transport::stdio())
      .await
      .inspect_err(|e| tracing::error!("serving error: {:?}", e))?;

  service.waiting().await?;
  ```
- All existing crates use `edition = "2024"` and `version = "0.1.0"`.

## Requirements

- Create the directory `crates/mcp-tool-harness/src/`.
- Create `crates/mcp-tool-harness/Cargo.toml` with `publish = false` and the dependencies listed below.
- Create `crates/mcp-tool-harness/src/lib.rs` implementing the `serve_stdio_tool` function.
- Add `"crates/mcp-tool-harness"` to the workspace `members` list in the root `Cargo.toml`.
- The public API must be exactly one function:
  ```rust
  pub async fn serve_stdio_tool<T: ServerHandler>(tool: T, tool_name: &str) -> Result<(), Box<dyn std::error::Error>>
  ```
- The function must:
  1. Initialize `tracing_subscriber` to write to stderr with ANSI disabled and `EnvFilter` defaulting to DEBUG level.
  2. Log an info message including the `tool_name` parameter (e.g., `"Starting {tool_name} MCP server"`).
  3. Call `.serve(rmcp::transport::stdio())` on the tool and log errors with `inspect_err`.
  4. Call `.waiting().await?` on the resulting service.
  5. Return `Ok(())`.
- The `ServerHandler` trait bound comes from `rmcp::handler::server::ServerHandler`. The function must re-export or use the correct import so callers only need `rmcp` as a dependency for their tool struct, not for the handler trait.
- Dependencies must be exactly: `rmcp`, `tokio`, `tracing`, `tracing-subscriber`. No additional dependencies.

## Implementation Details

### Files to create

1. **`crates/mcp-tool-harness/Cargo.toml`**

   ```toml
   [package]
   name = "mcp-tool-harness"
   version = "0.1.0"
   edition = "2024"
   publish = false

   [dependencies]
   rmcp = { version = "1", features = ["transport-io", "server"] }
   tokio = { version = "1", features = ["macros", "rt", "io-std"] }
   tracing = "0.1"
   tracing-subscriber = { version = "0.3", features = ["env-filter"] }
   ```

   Key points:
   - `publish = false` since this is an internal workspace utility, not intended for crates.io.
   - `rmcp` features mirror what the tool binaries already use: `transport-io` for `rmcp::transport::stdio()` and `server` for `ServerHandler` and `ServiceExt`.
   - `tokio` needs `io-std` for stdio transport access. `macros` and `rt` for the async runtime used by callers.
   - No `serde` or `serde_json` -- those are tool-specific concerns, not harness concerns.

2. **`crates/mcp-tool-harness/src/lib.rs`**

   Must contain:
   - A `pub use rmcp::handler::server::ServerHandler;` re-export so downstream tool crates can reference the trait without importing `rmcp` handler internals directly.
   - The `serve_stdio_tool` function with the exact signature specified above.
   - The function body must replicate the existing boilerplate pattern from the tool binaries, parameterized by `tool` and `tool_name`.

   Approximate structure (implementation must match this logic):
   ```rust
   use rmcp::ServiceExt;
   use tracing_subscriber::EnvFilter;

   pub use rmcp::handler::server::ServerHandler;

   pub async fn serve_stdio_tool<T: ServerHandler>(
       tool: T,
       tool_name: &str,
   ) -> Result<(), Box<dyn std::error::Error>> {
       tracing_subscriber::fmt()
           .with_env_filter(
               EnvFilter::from_default_env()
                   .add_directive(tracing::Level::DEBUG.into()),
           )
           .with_writer(std::io::stderr)
           .with_ansi(false)
           .init();

       tracing::info!("Starting {tool_name} MCP server");

       let service = tool
           .serve(rmcp::transport::stdio())
           .await
           .inspect_err(|e| tracing::error!("serving error: {:?}", e))?;

       service.waiting().await?;
       Ok(())
   }
   ```

### Files to modify

3. **`Cargo.toml`** (workspace root)

   Add `"crates/mcp-tool-harness"` to the `members` array. Insert it among the `crates/` entries, before the `tools/` entries, to maintain the current grouping convention:
   ```toml
   members = [
       "crates/agent-sdk",
       "crates/skill-loader",
       "crates/tool-registry",
       "crates/agent-runtime",
       "crates/mcp-tool-harness",
       "crates/orchestrator",
       "tools/echo-tool",
       "tools/read-file",
       "tools/write-file",
       "tools/validate-skill",
   ]
   ```

### Key design decisions

- **Library crate, not binary:** This is a `lib.rs` crate (no `main.rs`) because it provides a reusable function, not a standalone executable.
- **`publish = false`:** This is an internal utility for this workspace only. It should never be published to crates.io.
- **Single function API:** The entire public surface is one function. This keeps the API minimal and easy to evolve. If future tools need different transport options (e.g., HTTP SSE), new functions can be added without breaking existing callers.
- **Re-export `ServerHandler`:** Downstream tool crates need the `ServerHandler` trait in scope for the generic bound to resolve. Re-exporting it from the harness avoids requiring tools to know about `rmcp::handler::server` internals.
- **`Box<dyn std::error::Error>` return type:** Matches the existing `main.rs` return type in all tool binaries, making migration straightforward.
- **No `serde`/`serde_json` dependency:** Those are tool-specific for defining tool schemas. The harness only deals with transport and lifecycle.

## Dependencies

- Blocked by: Nothing (this is a foundational crate)
- Blocking: All 4 `main.rs` migrations (echo-tool, read-file, write-file, validate-skill must be updated to use `serve_stdio_tool` instead of inline boilerplate)

## Risks & Edge Cases

- **`tracing_subscriber` double-init:** If a caller accidentally initializes tracing before calling `serve_stdio_tool`, the `tracing_subscriber::fmt().init()` call will panic (or silently fail depending on version). This is acceptable for now since the function is intended to be called exactly once at program startup. A future enhancement could use `try_init()` instead, but that changes error semantics.
- **`ServerHandler` trait path:** The re-export path `rmcp::handler::server::ServerHandler` must be verified against the actual `rmcp` v1 API. If the module path differs, the import must be adjusted. The `ServiceExt` trait import (`rmcp::ServiceExt`) is also required for the `.serve()` method.
- **`rmcp` version compatibility:** The `rmcp = { version = "1", ... }` specifier must match what the existing tool crates already use. Since `tools/echo-tool/Cargo.toml` already uses `version = "1"`, this is consistent.
- **Generic bound `T: ServerHandler`:** The `ServerHandler` trait from `rmcp` may have additional bounds (e.g., `Send`, `Sync`). The generic function signature must satisfy whatever `ServiceExt::serve` requires. If additional trait bounds are needed, they should be added to the function signature.

## Verification

- `crates/mcp-tool-harness/Cargo.toml` exists with `publish = false` and the four dependencies listed above.
- `crates/mcp-tool-harness/src/lib.rs` exists and contains the `serve_stdio_tool` function with the specified signature.
- `Cargo.toml` (root) includes `"crates/mcp-tool-harness"` in the workspace members.
- `cargo check -p mcp-tool-harness` succeeds without errors.
- `cargo clippy -p mcp-tool-harness` passes without warnings.
- `cargo test -p mcp-tool-harness` passes (even if there are no tests yet).
- The function re-exports `ServerHandler` so downstream crates can use `mcp_tool_harness::ServerHandler`.
- No dependencies beyond `rmcp`, `tokio`, `tracing`, `tracing-subscriber` are present.
