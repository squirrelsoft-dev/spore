# Spec: Update agent-runtime main.rs with skeleton startup flow

> From: .claude/tasks/issue-9.md

## Objective

Replace the `println!("Hello, world!")` stub in `crates/agent-runtime/src/main.rs` with a `#[tokio::main]` async main function that demonstrates the full agent startup sequence: creating a tool registry, registering tool entries, connecting to MCP servers, loading a skill manifest, resolving MCP tools via the tool bridge, and building a rig-core agent. This is a scaffold that proves the integration between `tool-registry`, `skill-loader`, and the new `tool_bridge` module. Actual HTTP serving, config file parsing, and production error handling are explicitly deferred.

## Current State

`crates/agent-runtime/src/main.rs` contains only a synchronous hello-world stub:

```rust
fn main() {
    println!("Hello, world!");
}
```

`crates/agent-runtime/Cargo.toml` currently has no dependencies but will have the following added by the prerequisite task "Add `rig-core` and `rmcp` dependencies to agent-runtime Cargo.toml":

```toml
[dependencies]
rig-core = { version = "0.32", features = ["rmcp"] }
rmcp = { version = "0.16", features = ["client", "transport-async-rw"] }
tool-registry = { path = "../tool-registry" }
agent-sdk = { path = "../agent-sdk" }
skill-loader = { path = "../skill-loader" }
tokio = { version = "1", features = ["full"] }
```

The supporting crates provide:

- **`tool-registry`**: `ToolRegistry` with `new()`, `register()`, `connect_all()`, `get_handle()`, and `resolve_for_skill()`. `ToolEntry` describes a tool server endpoint. `McpHandle` wraps an rmcp client session.
- **`skill-loader`**: `SkillLoader::new(skill_dir, tool_registry, tool_checker)` and `SkillLoader::load(skill_name)` to parse a skill markdown file into a `SkillManifest`. Requires a `Box<dyn ToolExists + Send + Sync>` for validation.
- **`agent-sdk`**: `SkillManifest` struct containing `name`, `version`, `description`, `model`, `preamble`, `tools`, `constraints`, `output`.
- **`tool_bridge` (in agent-runtime)**: `resolve_mcp_tools(registry, manifest)` returns `Vec<McpTool>`, and `build_agent_with_tools(builder, tools)` returns a configured rig-core agent. This module is created by the prerequisite task "Implement MCP-to-rig-core bridge in agent-runtime".

## Requirements

1. Replace the synchronous `fn main()` with `#[tokio::main] async fn main()`.
2. The main function must demonstrate the following steps in order:
   - **(Step 1) Create a `ToolRegistry`**: Call `ToolRegistry::new()` and wrap in `Arc`.
   - **(Step 2) Register tool entries**: Call `registry.register(entry)` with at least one hardcoded example `ToolEntry` (e.g., an echo-tool at `mcp://localhost:7001`). This is placeholder data -- config-driven registration is deferred.
   - **(Step 3) Connect all tools**: Call `registry.connect_all().await` to establish MCP client connections for all registered entries.
   - **(Step 4) Load a skill manifest**: Create a `SkillLoader` and call `loader.load("echo").await` (or similar) to parse a skill markdown file. Use a hardcoded skill directory path (e.g., `./skills`).
   - **(Step 5) Resolve MCP tools**: Call `tool_bridge::resolve_mcp_tools(&registry, &manifest)` to get `Vec<McpTool>`.
   - **(Step 6) Build rig-core agent**: Call `tool_bridge::build_agent_with_tools(builder, tools)` to construct the agent with the resolved tools.
3. Each step must include a `println!` or `eprintln!` log line indicating progress (e.g., `"[startup] Registering tools..."`), so the startup flow is visible when running the binary.
4. Error handling must use `.expect("descriptive message")` or `?` with a `Result<(), Box<dyn std::error::Error>>` return type. Production error handling (structured logging, graceful degradation) is deferred.
5. The `mod tool_bridge;` declaration must be present in `main.rs` (or in a `lib.rs` if one exists) so the bridge module is accessible.
6. The code must compile with `cargo check -p agent-runtime` (given that all prerequisite tasks are complete).
7. The code must pass `cargo clippy -p agent-runtime` with no warnings.
8. No HTTP server, no config file parsing, no CLI argument handling -- these are explicitly out of scope.

## Implementation Details

### File to modify

- **`crates/agent-runtime/src/main.rs`** -- complete rewrite of the file.

### Structure of the new `main.rs`

```
mod tool_bridge;

use std::path::PathBuf;
use std::sync::Arc;

use tool_registry::{ToolEntry, ToolRegistry};
use skill_loader::SkillLoader;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Step 1: Create registry
    // Step 2: Register hardcoded tool entries
    // Step 3: Connect all tool servers
    // Step 4: Load skill manifest
    // Step 5: Resolve MCP tools for the skill
    // Step 6: Build rig-core agent
    // Print confirmation and exit
}
```

### Key details per step

1. **ToolRegistry creation**: `let registry = Arc::new(ToolRegistry::new());` -- must be `Arc` because `SkillLoader::new` takes `Arc<ToolRegistry>`.

2. **Register tool entries**: Construct a `ToolEntry` with hardcoded values:
   - `name`: `"echo-tool"` (matches the planned echo-tool from issue-10)
   - `version`: `"0.1.0"`
   - `endpoint`: `"mcp://localhost:7001"`
   - `handle`: `None` (not connected yet)
   Call `registry.register(entry)?;` or equivalent.

3. **Connect all**: `registry.connect_all().await?;` -- this iterates all registered entries and establishes MCP connections.

4. **Load skill manifest**: Create a `SkillLoader` with `SkillLoader::new(PathBuf::from("./skills"), registry.clone(), tool_checker)` where `tool_checker` is constructed from the registry (the `ToolRegistry` implements `ToolExists` per issue-8). Call `loader.load("echo").await?` to get the manifest.

5. **Resolve MCP tools**: `let tools = tool_bridge::resolve_mcp_tools(&registry, &manifest)?;` -- maps skill tool names to connected `McpTool` instances.

6. **Build agent**: Use rig-core's `AgentBuilder` (obtained via a provider client, e.g., `openai::Client::from_env().agent("gpt-4o")`) and pass the tools via `tool_bridge::build_agent_with_tools(builder, tools).await`. The provider and model are hardcoded placeholders. Actual model selection from `SkillManifest.model` is deferred.

### Integration points

- Depends on `tool_bridge` module existing at `crates/agent-runtime/src/tool_bridge.rs` (created by the "Implement MCP-to-rig-core bridge" task).
- Depends on `ToolRegistry` having `register()`, `connect_all()`, and other methods (from issue-8 and issue-9 Group 3).
- Depends on `ToolEntry` having a constructor or public fields including `handle: Option<McpHandle>` (from issue-9 Group 2).
- Depends on `SkillLoader::new()` accepting `Arc<ToolRegistry>` and `Box<dyn ToolExists + Send + Sync>`.

## Dependencies

- **Blocked by**: "Implement MCP-to-rig-core bridge in agent-runtime" (which itself depends on Groups 1-3 of issue-9 and on issue-8 for `ToolRegistry` methods)
- **Blocking**: None -- this is the final task in the issue-9 dependency chain.

## Risks & Edge Cases

1. **API surface not finalized**: The `ToolRegistry`, `ToolEntry`, and `tool_bridge` APIs are being built concurrently. If method signatures differ from what this spec assumes (e.g., `register` takes `&self` vs `&mut self`, or `connect_all` returns a different error type), the main.rs code will need adjustment. Mitigation: review the actual implementations from the prerequisite tasks before writing main.rs.

2. **rig-core provider initialization**: Building a rig-core agent requires a provider client (e.g., `openai::Client`). This typically needs an API key via environment variable. Since this is a scaffold, hardcode a provider and document that the binary will panic at runtime if the API key is missing. Alternatively, use a comment/todo indicating where provider initialization will be replaced with config-driven setup.

3. **`ToolRegistry` implements `ToolExists`**: The `SkillLoader::new()` requires a `Box<dyn ToolExists + Send + Sync>`. Issue-8 specifies that `ToolRegistry` implements the `ToolExists` trait. If the implementation wraps `Arc<ToolRegistry>`, the boxing may need `AllToolsExist` as a fallback or a newtype adapter. Verify the actual trait implementation from issue-8.

4. **Hardcoded skill directory**: The path `./skills` is relative to the working directory at runtime. If the binary is run from a different directory, the skill file will not be found. This is acceptable for a scaffold but should be noted with a comment.

5. **No graceful shutdown**: The skeleton does not handle SIGINT/SIGTERM or clean up MCP connections. This is acceptable for a scaffold but should be noted as a future concern.

6. **Compilation without running tool servers**: The binary will compile but will fail at runtime if no MCP tool server is listening on the hardcoded endpoint. This is expected for a scaffold. The startup flow is meant to demonstrate the code path, not to be a runnable demo without infrastructure.

## Verification

1. `cargo check -p agent-runtime` succeeds with no errors (requires all prerequisite tasks to be complete).
2. `cargo clippy -p agent-runtime` produces no warnings.
3. `cargo test` across the workspace passes with no regressions.
4. Reading `crates/agent-runtime/src/main.rs` confirms:
   - `#[tokio::main]` attribute is present on `main`.
   - `main` is `async` and returns `Result<(), Box<dyn std::error::Error>>`.
   - All six startup steps are present in order: registry creation, tool registration, connect_all, skill loading, tool resolution, agent building.
   - Each step has a progress log line.
   - `mod tool_bridge;` declaration is present.
   - No HTTP server code, no config file parsing, no CLI argument handling.
5. The file contains no commented-out code or debug statements beyond the intentional progress log lines.
6. No function in the file exceeds 50 lines (the main function may approach this limit; if so, extract helpers like `register_tools`, `load_skill`, etc.).
