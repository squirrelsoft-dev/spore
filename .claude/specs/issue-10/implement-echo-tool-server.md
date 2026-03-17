# Spec: Implement echo tool server

> From: .claude/tasks/issue-10.md

## Objective
Create the core MCP tool server binary for the `echo-tool` crate. This is the first `rmcp`-based tool server in the workspace and will serve as the canonical template for all future tool implementations. The echo tool accepts a message string and returns it unchanged, demonstrating the minimal viable MCP tool server pattern.

## Current State
- The `tools/` directory exists but contains only `.gitkeep`. No tool implementations exist yet.
- The workspace uses `edition = "2024"` across all crates.
- `tool-registry` is a stub (`pub struct ToolRegistry;`) -- tool registration is deferred to issue #8.
- `rmcp` is not yet in the workspace dependency tree. This task is the first to use it.
- Existing `main.rs` files in the workspace (e.g., `crates/agent-runtime/src/main.rs`, `crates/orchestrator/src/main.rs`) are placeholder `println!("Hello, world!")` stubs.
- The companion tasks "Create `tools/echo-tool/` crate with Cargo.toml" and "Add `tools/echo-tool` to workspace members" must land first, providing the `Cargo.toml` with dependencies on `rmcp`, `tokio`, `serde`, `serde_json`, `tracing`, and `tracing-subscriber`.

## Requirements
- The `echo-tool` binary starts an MCP server over stdio transport.
- The server advertises exactly one tool named `echo` with description "Returns the input message unchanged".
- The `echo` tool accepts a single required parameter `message` of type `String`.
- The `echo` tool returns the message unchanged as `Content::text(message)` inside a `CallToolResult::success`.
- All logging goes to stderr (never stdout, which is the MCP transport channel).
- The `main.rs` file stays under 50 lines. If the tool definition pushes it over, split into `main.rs` (entrypoint) and `echo.rs` (tool struct + impls).
- No `clap` dependency. No CLI argument parsing.
- The server runs until the transport closes (via `.waiting().await`).

## Implementation Details

### File: `tools/echo-tool/src/main.rs`

The entrypoint file. If the tool definition fits under 50 lines total, keep everything here. Otherwise, extract the tool definition into `echo.rs` and add `mod echo;` + `use echo::EchoTool;`.

```rust
use rmcp::ServiceExt;
use tracing_subscriber::{self, EnvFilter};

mod echo; // only if split

#[tokio::main]
async fn main() -> anyhow::Result<()> {
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
    Ok(())
}
```

Note: The task description says no `anyhow` dependency. Use `Result<(), Box<dyn std::error::Error>>` as the return type instead if `anyhow` is not in `Cargo.toml`. Check the actual `Cargo.toml` dependencies once the scaffolding task is complete.

### File: `tools/echo-tool/src/echo.rs` (or inline in `main.rs`)

Define the `EchoTool` struct, the tool router, and the `ServerHandler` impl.

**Key types and patterns** (derived from the rmcp counter example at `https://github.com/modelcontextprotocol/rust-sdk/tree/main/examples/servers`):

1. **EchoTool struct:**
   ```rust
   #[derive(Clone)]
   pub struct EchoTool {
       tool_router: ToolRouter<EchoTool>,
   }
   ```
   The `tool_router` field is required by the `#[tool_router]` macro -- it stores the generated routing table. The `ToolRouter` type comes from `rmcp::handler::server::router::tool::ToolRouter`.

2. **Constructor + tool method** (inside `#[tool_router] impl EchoTool`):
   ```rust
   #[tool_router]
   impl EchoTool {
       pub fn new() -> Self {
           Self {
               tool_router: Self::tool_router(),
           }
       }

       #[tool(description = "Returns the input message unchanged")]
       fn echo(
           &self,
           Parameters(request): Parameters<EchoRequest>,
       ) -> Result<CallToolResult, McpError> {
           Ok(CallToolResult::success(vec![Content::text(request.message)]))
       }
   }
   ```

3. **Parameter struct:**
   ```rust
   #[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
   pub struct EchoRequest {
       /// The message to echo back
       pub message: String,
   }
   ```
   The `Parameters<T>` wrapper is mandatory -- the `#[tool]` macro uses it to locate the type for JSON schema generation. `T` must implement `Deserialize` and `JsonSchema`. The doc comment on `message` becomes the parameter description in the tool schema.

4. **ServerHandler impl:**
   ```rust
   #[tool_handler]
   impl ServerHandler for EchoTool {
       fn get_info(&self) -> ServerInfo {
           ServerInfo::new(
               ServerCapabilities::builder()
                   .enable_tools()
                   .build(),
           )
       }
   }
   ```
   The `#[tool_handler]` attribute macro wires the tool router into the `ServerHandler` trait implementation. Only `enable_tools()` is needed -- no prompts, resources, or subscriptions.

### Required imports

```rust
use rmcp::{
    ErrorData as McpError,
    ServerHandler,
    handler::server::{
        router::tool::ToolRouter,
        wrapper::Parameters,
    },
    model::*,
    tool, tool_handler, tool_router,
};
```

The `model::*` glob brings in `CallToolResult`, `Content`, `ServerInfo`, `ServerCapabilities`, `Implementation`, and related types.

### Line budget

Estimated line counts:
- `EchoRequest` struct: ~5 lines
- `EchoTool` struct: ~4 lines
- `#[tool_router] impl EchoTool` (new + echo): ~15 lines
- `#[tool_handler] impl ServerHandler`: ~10 lines
- Imports: ~10 lines
- Total tool definition: ~44 lines

If keeping everything in `main.rs`, add ~15 lines for `main()`, totaling ~59 lines -- over the 50-line limit. Therefore, **split into two files**: `main.rs` (~20 lines) and `echo.rs` (~44 lines).

## Dependencies
- Blocked by: "Create `tools/echo-tool/` crate with Cargo.toml", "Add `tools/echo-tool` to workspace members"
- Blocking: "Write unit tests for echo tool logic", "Write integration test for MCP server round-trip", "Write tool README"

## Risks & Edge Cases

1. **rmcp API version mismatch:** The counter example is based on the `main` branch of `modelcontextprotocol/rust-sdk`. The published crate version (`rmcp = "1"`) may differ in API details (e.g., `ErrorData` vs `McpError`, presence of `tool_handler` macro). Mitigation: check `cargo doc -p rmcp` after the scaffolding task lands and adjust imports/types accordingly.

2. **`tool_router` field requirement:** The `#[tool_router]` macro generates a `Self::tool_router()` constructor method that returns a `ToolRouter<Self>`. The struct must store this in a field named `tool_router`. If the macro convention changes, the field name must match. Mitigation: follow the counter example pattern exactly.

3. **Missing `schemars` dependency:** The `EchoRequest` struct derives `schemars::JsonSchema`. The `rmcp` crate re-exports `schemars`, so it should be available as `rmcp::schemars` or through `rmcp`'s own `schemars` feature. If not, `schemars` must be added as a direct dependency in `tools/echo-tool/Cargo.toml`. Mitigation: check if the scaffolding task's `Cargo.toml` includes `schemars` or if `rmcp` re-exports it.

4. **Stdout contamination:** Any accidental `println!` or default tracing subscriber writing to stdout will corrupt the MCP stdio transport. Mitigation: the `tracing_subscriber` is explicitly configured with `.with_writer(std::io::stderr)`, and no `println!` calls should appear anywhere in the crate.

5. **Graceful shutdown:** The `.waiting().await` call blocks until the transport closes. If the MCP client disconnects abruptly, the server should exit cleanly. The `rmcp` framework handles this internally, but test this during verification.

## Verification

1. **Compilation:** `cargo check -p echo-tool` succeeds with no errors.
2. **Lint:** `cargo clippy -p echo-tool` produces no warnings.
3. **Line count:** `wc -l tools/echo-tool/src/main.rs` is under 50 lines. If split, both `main.rs` and `echo.rs` are individually under 50 lines.
4. **Server starts:** `cargo run -p echo-tool` starts without errors and waits for MCP client input on stdin (verified by checking stderr log output shows "Starting echo-tool MCP server").
5. **MCP inspector:** `npx @modelcontextprotocol/inspector cargo run -p echo-tool` shows the `echo` tool listed with the correct description and `message` parameter schema.
6. **Tool invocation:** Calling the `echo` tool via MCP inspector with `{ "message": "hello" }` returns `"hello"` unchanged.
7. **No stdout leakage:** Running `cargo run -p echo-tool 2>/dev/null` produces no output on stdout (only the MCP protocol messages when a client connects).
8. **Workspace tests:** `cargo test` across the full workspace still passes (no regressions).
