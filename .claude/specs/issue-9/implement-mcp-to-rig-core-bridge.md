# Spec: Implement MCP-to-rig-core bridge in agent-runtime

> From: .claude/tasks/issue-9.md

## Objective

Create a bridge module in the `agent-runtime` crate that translates MCP tool handles from the `tool-registry` into `rig-core` `McpTool` instances that can be attached to a rig-core `Agent`. This is the integration point between the MCP protocol layer (rmcp) and the LLM agent framework (rig-core). Without this bridge, the agent-runtime has no way to give the rig-core agent access to MCP-connected tools.

The module provides two public functions: `resolve_mcp_tools` (discovers MCP tools from connected handles and wraps them as `McpTool`) and `build_agent_with_tools` (attaches resolved tools to a rig-core `AgentBuilder` and produces an `Agent`).

## Current State

- **`crates/agent-runtime/src/main.rs`**: Contains only a `println!("Hello, world!")` stub. No modules, no imports, no async runtime.
- **`crates/agent-runtime/Cargo.toml`**: Currently has no dependencies. The prerequisite task "Add `rig-core` and `rmcp` dependencies to agent-runtime Cargo.toml" will add `rig-core`, `rmcp`, `tool-registry`, `agent-sdk`, `skill-loader`, and `tokio`.
- **`crates/tool-registry/src/lib.rs`**: Currently a stub `pub struct ToolRegistry;`. The prerequisite tasks from issue-8 and earlier issue-9 groups will populate this with:
  - `ToolRegistry` struct with `resolve_for_skill(&self, manifest: &SkillManifest) -> Result<Vec<ToolEntry>, RegistryError>` and `get_handle(name: &str) -> Option<McpHandle>`.
  - `ToolEntry` struct with fields `name: String`, `version: String`, `endpoint: String`, and `handle: Option<McpHandle>` (where `handle` is `#[serde(skip)]`).
  - `McpHandle` newtype wrapping `RunningService<RoleClient, ()>` with `peer(&self) -> &Peer<RoleClient>` method.
  - `RegistryError` enum with `ToolNotFound`, `DuplicateEntry`, and `ConnectionFailed` variants.
- **`agent-sdk::SkillManifest`**: Defined in `crates/agent-sdk/src/skill_manifest.rs` with fields `name`, `version`, `description`, `model: ModelConfig`, `preamble: String`, `tools: Vec<String>`, `constraints: Constraints`, `output: OutputSchema`.
- **rig-core API** (version 0.32 with `rmcp` feature):
  - `rig::providers::anthropic::Client` (or other providers) create completion models.
  - `AgentBuilder` is obtained from `client.agent(model_name)` and supports `.tool(impl Tool)` or `.tools(vec)` methods.
  - `McpTool` is a rig-core type (gated behind the `rmcp` feature) that wraps an rmcp `Tool` definition together with a `ServerSink` for dispatching calls. It implements rig-core's `Tool` trait.
  - `Agent` is the final built agent (from `AgentBuilder::build()`).
- **rmcp API** (version 0.16):
  - `Peer<RoleClient>` has `list_tools(None).await` returning `Result<ListToolsResult, _>` where `ListToolsResult` contains a `tools: Vec<Tool>` field.
  - `ServerSink` is obtained from `RunningService::peer()` -- it is the send half used by `McpTool` to dispatch tool calls to the MCP server.
  - `McpTool::new(tool_definition, server_sink)` creates an `McpTool` from an rmcp `Tool` definition and a `ServerSink`.

## Requirements

1. Create a new file `crates/agent-runtime/src/tool_bridge.rs`.
2. Define `pub async fn resolve_mcp_tools(registry: &ToolRegistry, manifest: &SkillManifest) -> Result<Vec<McpTool>, RegistryError>` that:
   - Calls `registry.resolve_for_skill(manifest)` to get the `Vec<ToolEntry>` matching the skill's declared tools.
   - Iterates over the returned entries and filters to those with a connected `McpHandle` (i.e., `entry.handle.is_some()`).
   - For each entry with a handle, calls `handle.peer().list_tools(None).await` to discover the tools exposed by that MCP server.
   - Maps connection/protocol errors from `list_tools` to `RegistryError::ConnectionFailed` with the entry's endpoint and the error description as the reason.
   - For each `Tool` definition returned by `list_tools`, creates an `McpTool` instance pairing the tool definition with the `ServerSink` (obtained from the handle's peer/running service).
   - Collects all `McpTool` instances across all entries into a single `Vec<McpTool>` and returns it.
   - Entries without a connected handle (where `handle` is `None`) are silently skipped -- they represent tools that are registered but not yet connected, which is a valid state during incremental startup.
3. Define `pub async fn build_agent_with_tools(builder: AgentBuilder, tools: Vec<McpTool>) -> Agent` that:
   - Takes an `AgentBuilder` (already configured with model, preamble, etc.) and a `Vec<McpTool>`.
   - Attaches each `McpTool` to the builder using the appropriate rig-core API (`.tool()` per tool, or a bulk method if available).
   - Calls `.build()` on the builder and returns the resulting `Agent`.
4. The function `resolve_mcp_tools` must be `async` because `list_tools` is an async operation.
5. Both functions must be under 50 lines each (per project rules).
6. No test module in this file. Tests will be written as part of integration testing tasks.
7. No commented-out code or debug statements.
8. Add `mod tool_bridge;` to `main.rs` (or `lib.rs` if one is created) so the module is compiled. This wiring is minimal -- just the module declaration.

## Implementation Details

### File to create

**`crates/agent-runtime/src/tool_bridge.rs`**

- **Imports needed at the top of the file:**
  ```rust
  use agent_sdk::SkillManifest;
  use rig::agent::{Agent, AgentBuilder};
  use rig_rmcp::McpTool;      // or rig::tool::McpTool depending on re-export path
  use tool_registry::{ToolRegistry, RegistryError};
  ```
  Note: The exact import paths for rig-core types (`Agent`, `AgentBuilder`, `McpTool`) depend on how rig-core 0.32 with the `rmcp` feature organizes its public API. The implementer should verify the actual module paths by checking `rig-core`'s documentation or running `cargo doc -p rig-core --open`. Common paths include `rig::agent::Agent`, `rig::agent::AgentBuilder`, and the `McpTool` type may be at `rig::tool::McpTool` or re-exported through a `rig::providers::rmcp` module.

- **`resolve_mcp_tools` implementation sketch:**
  ```rust
  pub async fn resolve_mcp_tools(
      registry: &ToolRegistry,
      manifest: &SkillManifest,
  ) -> Result<Vec<McpTool>, RegistryError> {
      let entries = registry.resolve_for_skill(manifest)?;
      let mut mcp_tools = Vec::new();

      for entry in &entries {
          let handle = match &entry.handle {
              Some(h) => h,
              None => continue,
          };

          let tools_result = handle.peer().list_tools(None).await.map_err(|e| {
              RegistryError::ConnectionFailed {
                  endpoint: entry.endpoint.clone(),
                  reason: e.to_string(),
              }
          })?;

          let server_sink = handle.server_sink();
          for tool_def in tools_result.tools {
              mcp_tools.push(McpTool::new(tool_def, server_sink.clone()));
          }
      }

      Ok(mcp_tools)
  }
  ```
  The exact method to obtain the `ServerSink` depends on the `McpHandle` API. The handle wraps `RunningService<RoleClient, ()>`, and `RunningService` exposes both `.peer()` (for sending requests like `list_tools`) and a way to get the `ServerSink` (which `McpTool` needs for dispatching tool calls). The implementer must check the rmcp 0.16 API to determine whether `ServerSink` is obtained from `peer()` directly, from the `RunningService`, or through another accessor.

- **`build_agent_with_tools` implementation sketch:**
  ```rust
  pub async fn build_agent_with_tools(
      builder: AgentBuilder,
      tools: Vec<McpTool>,
  ) -> Agent {
      let mut builder = builder;
      for tool in tools {
          builder = builder.tool(tool);
      }
      builder.build()
  }
  ```
  This function is intentionally simple. The caller is responsible for configuring the `AgentBuilder` with the model provider, preamble, temperature, etc. before passing it here. This function only attaches tools and finalizes the build.

### File to modify

**`crates/agent-runtime/src/main.rs`**

- Add `mod tool_bridge;` declaration so the module is compiled as part of the crate. The existing `fn main()` stub remains unchanged (the "Update agent-runtime main.rs" task will replace it later).

### Key types and their relationships

```
SkillManifest.tools: Vec<String>
        |
        v  (resolve_for_skill)
Vec<ToolEntry>  (each has name, endpoint, handle: Option<McpHandle>)
        |
        v  (filter to Some(handle), call list_tools)
Vec<rmcp::Tool>  (tool definitions from MCP servers)
        |
        v  (pair with ServerSink, wrap in McpTool)
Vec<McpTool>  (rig-core compatible tool wrappers)
        |
        v  (build_agent_with_tools)
Agent  (rig-core agent with tools attached)
```

### Integration points

- **Upstream (tool-registry):** Depends on `ToolRegistry::resolve_for_skill()`, `ToolEntry.handle`, `McpHandle::peer()`, and `RegistryError`.
- **Upstream (agent-sdk):** Uses `SkillManifest` to determine which tools to resolve.
- **Downstream (main.rs):** The "Update agent-runtime main.rs" task will call `resolve_mcp_tools` and `build_agent_with_tools` in the startup flow.
- **Lateral (rig-core):** Consumes `AgentBuilder` and produces `Agent`. The caller obtains the `AgentBuilder` from a rig-core provider client (e.g., `client.agent("claude-sonnet-4-20250514")`).

## Dependencies

- **Blocked by:**
  - "Add `rig-core` and `rmcp` dependencies to agent-runtime Cargo.toml" (Group 4) -- without these dependencies, the rig-core and rmcp types cannot be imported.
  - "Implement `connect()` with real MCP client logic" (Group 3) -- without connected handles, `resolve_mcp_tools` would skip all entries. More importantly, the `McpHandle` type and `ToolEntry.handle` field must exist for this code to compile.
- **Blocking:**
  - "Update agent-runtime main.rs with skeleton startup flow" (Group 4) -- the main.rs task calls both functions from this module.

## Risks & Edge Cases

1. **rig-core API surface uncertainty.** The exact import paths and method signatures for `McpTool`, `AgentBuilder`, and `Agent` in rig-core 0.32 may differ from what is sketched here. The `rmcp` feature gate may expose types under different module paths. Mitigation: the implementer must verify paths by checking rig-core's docs or source before writing imports. Run `cargo doc -p rig-core --open` after dependencies are added.

2. **`McpTool::new` constructor signature.** The `McpTool` constructor in rig-core 0.32 may take different arguments than `(Tool, ServerSink)`. It may require a reference, an `Arc`, or a different sink type. Mitigation: check rig-core's `McpTool` source or documentation for the exact constructor.

3. **`list_tools` error type mismatch.** The error returned by `handle.peer().list_tools()` is an rmcp error type, not a `RegistryError`. The `.map_err()` conversion must handle whatever error type rmcp 0.16 returns. If rmcp's error type does not implement `Display` or `ToString`, the conversion will need adjustment.

4. **Entries without handles silently skipped.** If a skill declares a tool that is registered but not connected, `resolve_mcp_tools` silently skips it. This means the agent will lack that tool. This is intentional for incremental startup, but callers should be aware that the returned `Vec<McpTool>` may contain fewer tools than `manifest.tools.len()`. A future enhancement could log a warning or return a partial-success result.

5. **Multiple tools per MCP server.** A single MCP server (one `ToolEntry`) may expose multiple tools via `list_tools`. The function correctly handles this by iterating `tools_result.tools` and creating one `McpTool` per tool definition. The returned vector may be larger than the number of entries.

6. **Tool name collisions.** If two different MCP servers expose tools with the same name, both will be added to the agent. Rig-core's behavior in this case is undefined (it may use the first or last tool with that name). This is an edge case that should be documented but does not need to be solved in this task.

7. **`build_agent_with_tools` is synchronous internally.** The `AgentBuilder::tool()` and `AgentBuilder::build()` methods in rig-core may be synchronous. If so, `build_agent_with_tools` does not strictly need to be `async`. However, declaring it `async` provides forward compatibility if rig-core adds async build steps later, and it matches the async calling context in `main.rs`. If rig-core's `build()` is async, the function signature is already correct.

8. **`ServerSink` cloning.** The `ServerSink` is cloned for each tool definition from the same MCP server. This is safe because `ServerSink` is designed to be cloned (it is a channel sender). However, if `McpHandle` or the running service does not support `Clone` on the sink, an `Arc` wrapper may be needed.

## Verification

After implementation (and after all blocking tasks are complete), run:

```bash
cargo check -p agent-runtime
cargo clippy -p agent-runtime
cargo test -p agent-runtime
```

All three must pass with no errors and no warnings. Additionally verify:

- The file `crates/agent-runtime/src/tool_bridge.rs` exists.
- `main.rs` contains `mod tool_bridge;`.
- `resolve_mcp_tools` is `pub async fn` with the specified signature.
- `build_agent_with_tools` is `pub async fn` (or `pub fn` if rig-core's build is synchronous) with the specified signature.
- Both functions are under 50 lines each.
- No test module, no commented-out code, no debug statements in the file.
- The function correctly filters entries to those with `Some(handle)` and iterates `list_tools` results.
- Error mapping from rmcp errors to `RegistryError::ConnectionFailed` is present and includes the endpoint and reason.
