# Spec: Refactor main.rs to use config and RuntimeAgent

> From: .claude/tasks/issue-11.md

## Objective

Rewrite `main.rs` to replace the current ad-hoc startup sequence with a structured flow that uses `RuntimeConfig` for environment-driven settings, `provider::build_agent()` for LLM client construction, and `RuntimeAgent` wrapped as `Arc<dyn MicroAgent>`. This transforms the binary from a hardcoded prototype into a configurable, trait-based runtime ready for HTTP serving in issue #12. The `register_default_tools()` function is removed in favor of `TOOL_ENDPOINTS` env var parsing with hardcoded fallbacks.

## Current State

`crates/agent-runtime/src/main.rs` has a 10-step startup flow using `println!` for logging (to be replaced by tracing in a prerequisite task) and a hardcoded `register_default_tools()` function that creates a single `ToolEntry` for `echo-tool` at `mcp://localhost:7001`. The agent is built directly via `openai::Client::new("placeholder-key")` with no config-driven provider selection. The built agent (`_agent`) is unused and immediately dropped. There is no `MicroAgent` trait implementation wrapping the agent.

Key files in the current state:
- `crates/agent-runtime/src/main.rs` — 61 lines, contains `main()` and `register_default_tools()`.
- `crates/agent-runtime/src/tool_bridge.rs` — provides `resolve_mcp_tools()` and `build_agent_with_tools()`.
- `crates/agent-runtime/Cargo.toml` — depends on `rig-core`, `rmcp`, `tool-registry`, `agent-sdk`, `skill-loader`, `tokio`, `futures`. After the tracing prerequisite task, will also have `tracing` and `tracing-subscriber`.

By the time this task is implemented, three prerequisite tasks will have created:
- `crates/agent-runtime/src/config.rs` — `RuntimeConfig` struct with `from_env()` returning `Result<Self, ConfigError>`, fields: `skill_name`, `skill_dir`, `bind_addr`.
- `crates/agent-runtime/src/provider.rs` — `build_agent()` function taking `&SkillManifest` and `Vec<McpTool>`, returning a built rig-core `Agent`.
- `crates/agent-runtime/src/runtime_agent.rs` — `RuntimeAgent` struct implementing `MicroAgent` trait, holding `SkillManifest`, the rig-core agent, and `Arc<ToolRegistry>`.

## Requirements

1. **Tracing subscriber initialization**: The first operation in `main()` must initialize the tracing subscriber (already set up by the tracing prerequisite task). This is a no-op change if the tracing task has already been applied.

2. **Config loading**: Call `RuntimeConfig::from_env()?` immediately after tracing init. Log the loaded config values at `info` level (skill name, skill dir, bind addr).

3. **Tool registration via TOOL_ENDPOINTS env var**: Parse the `TOOL_ENDPOINTS` environment variable as a comma-separated list of `name=endpoint` pairs (e.g., `echo-tool=mcp://localhost:7001,other-tool=mcp://localhost:7002`). Each pair creates a `ToolEntry` with `version: "0.1.0"` and `handle: None`. If `TOOL_ENDPOINTS` is not set or is empty, fall back to a hardcoded default of `echo-tool=mcp://localhost:7001`. Log each registered tool at `info` level.

4. **Remove `register_default_tools()`**: The standalone function is eliminated. Tool registration logic moves inline into `main()` using the env var parsing described above.

5. **Tool connection**: Call `registry.connect_all().await?` as before.

6. **Skill loading**: Create `SkillLoader` using `config.skill_dir` (from `RuntimeConfig`) instead of the hardcoded `"./skills"` path. Load `config.skill_name` instead of the hardcoded `"echo"`.

7. **MCP tool resolution**: Call `tool_bridge::resolve_mcp_tools(&registry, &manifest).await?` as before.

8. **Agent construction via provider module**: Call `provider::build_agent(&manifest, tools)` instead of directly constructing `openai::Client` and calling `build_agent_with_tools`. This delegates provider selection, API key reading, and model configuration to the provider module.

9. **RuntimeAgent construction**: Construct a `RuntimeAgent` from the manifest, built agent, and registry.

10. **Wrap as `Arc<dyn MicroAgent>`**: Wrap the `RuntimeAgent` in `Arc<dyn MicroAgent>` and log readiness. The `Arc<dyn MicroAgent>` is the handoff point for the HTTP server in issue #12.

11. **No HTTP server**: The function ends after logging readiness. No HTTP server is started. The `Arc<dyn MicroAgent>` is created and held but not served.

12. **Error handling**: `main()` continues to return `Result<(), Box<dyn std::error::Error>>`. All new operations use `?` for error propagation. The `ConfigError` from `RuntimeConfig::from_env()` must implement `std::error::Error` (ensured by the config module spec). Invalid `TOOL_ENDPOINTS` format should produce a clear error message logged at `error` level before returning an error.

13. **Module declarations**: `main.rs` must declare `mod config;`, `mod provider;`, and `mod runtime_agent;` in addition to the existing `mod tool_bridge;`.

## Implementation Details

### Files to modify

**`crates/agent-runtime/src/main.rs`** (complete rewrite)

- Add module declarations at the top:
  ```rust
  mod config;
  mod provider;
  mod runtime_agent;
  mod tool_bridge;
  ```

- Add imports:
  - `use std::sync::Arc;`
  - `use agent_sdk::MicroAgent;`
  - `use skill_loader::SkillLoader;`
  - `use tool_registry::{ToolEntry, ToolRegistry};`
  - `use config::RuntimeConfig;`
  - `use runtime_agent::RuntimeAgent;`
  - Tracing imports (already present from prerequisite task)

- Rewrite `main()` with the following 10-step flow:

  1. **Init tracing** — already in place from prerequisite task.
  2. **Load config** — `let config = RuntimeConfig::from_env()?;` followed by `tracing::info!(skill_name = %config.skill_name, skill_dir = ?config.skill_dir, bind_addr = %config.bind_addr, "Configuration loaded");`
  3. **Create registry** — `let registry = Arc::new(ToolRegistry::new());`
  4. **Register tools** — Parse `TOOL_ENDPOINTS` env var. Implementation:
     ```rust
     let endpoints = std::env::var("TOOL_ENDPOINTS")
         .unwrap_or_else(|_| "echo-tool=mcp://localhost:7001".to_string());
     for pair in endpoints.split(',') {
         let pair = pair.trim();
         if pair.is_empty() { continue; }
         let (name, endpoint) = pair.split_once('=')
             .ok_or_else(|| format!("Invalid TOOL_ENDPOINTS entry: '{pair}' (expected name=endpoint)"))?;
         let entry = ToolEntry {
             name: name.trim().to_string(),
             version: "0.1.0".to_string(),
             endpoint: endpoint.trim().to_string(),
             handle: None,
         };
         tracing::info!(name = %entry.name, endpoint = %entry.endpoint, "Registering tool");
         registry.register(entry)?;
     }
     ```
  5. **Connect tools** — `registry.connect_all().await?;`
  6. **Load skill** — Create `SkillLoader` with `config.skill_dir` and `registry.clone()`, load `config.skill_name`.
  7. **Resolve MCP tools** — `let tools = tool_bridge::resolve_mcp_tools(&registry, &manifest).await?;`
  8. **Build agent** — `let agent = provider::build_agent(&manifest, tools)?;` (the exact signature depends on the provider module spec, but this is the expected call pattern).
  9. **Construct RuntimeAgent** — `let runtime_agent = RuntimeAgent::new(manifest, agent, registry.clone());` (constructor signature depends on runtime_agent module spec).
  10. **Wrap and log** — `let _micro_agent: Arc<dyn MicroAgent> = Arc::new(runtime_agent);` followed by `tracing::info!("Agent ready");`

- Remove `register_default_tools()` function entirely.

### Key functions/types/interfaces

- **`RuntimeConfig::from_env()`** — consumed from `config` module. Returns `Result<RuntimeConfig, ConfigError>`.
- **`provider::build_agent()`** — consumed from `provider` module. Takes `&SkillManifest` and `Vec<McpTool>`, returns the built agent (exact return type depends on provider module, likely generic or type-erased).
- **`RuntimeAgent::new()`** — consumed from `runtime_agent` module. Constructor taking manifest, agent, and registry.
- **`TOOL_ENDPOINTS` env var parsing** — new inline logic in `main()`. No separate function needed since it is straightforward and single-use.

### Integration points

- `config::RuntimeConfig` provides `skill_name: String`, `skill_dir: PathBuf`, and `bind_addr: SocketAddr`.
- `SkillLoader::new()` takes `PathBuf`, `Arc<ToolRegistry>`, `Box<dyn ToolExists>` — the `skill_dir` comes from config now.
- `SkillLoader::load()` takes `&str` — the skill name comes from config now.
- `provider::build_agent()` replaces the direct `openai::Client::new()` + `build_agent_with_tools()` pattern. The `openai` import from `rig::providers::openai` is no longer needed in `main.rs`.
- `RuntimeAgent` implements `MicroAgent`, so it can be wrapped in `Arc<dyn MicroAgent>`.
- The `rig::client::CompletionClient` import is no longer needed in `main.rs` (moved to provider module).

## Dependencies

- Blocked by:
  - "Create config module for environment-driven settings" — provides `RuntimeConfig` and `ConfigError`
  - "Add tracing dependency and replace println with structured logging" — provides tracing infrastructure
  - "Implement RuntimeAgent struct with MicroAgent trait" — provides `RuntimeAgent` (which itself depends on the provider module)
- Blocking:
  - "Write unit tests for config and provider modules" — tests that depend on the refactored main.rs flow

## Risks & Edge Cases

- **`TOOL_ENDPOINTS` parsing edge cases**: Empty string, trailing comma, whitespace around `=` or `,`, missing `=` in a pair, duplicate tool names. Mitigation: trim whitespace, skip empty segments after split, return a clear error on missing `=`, let `registry.register()` catch duplicates via `RegistryError::DuplicateEntry`.
- **Provider module return type compatibility**: The exact return type of `provider::build_agent()` must be compatible with what `RuntimeAgent::new()` expects. Since rig-core's `Agent<M, P>` is generic, both modules must agree on whether the agent is generic or type-erased. The provider and runtime_agent specs must coordinate on this. If `RuntimeAgent` is generic (e.g., `RuntimeAgent<M, P>`), then `main.rs` will need to use the concrete types; if type-erased, it will be simpler. This is a coordination risk between three specs.
- **Error type mismatch**: `main()` returns `Box<dyn std::error::Error>`. `ConfigError`, `RegistryError`, `SkillError`, and any provider error must all implement `std::error::Error`. The `TOOL_ENDPOINTS` parsing error is currently a `String` — wrap it in a concrete error type or use `Box<dyn std::error::Error>` directly via `.into()`.
- **`bind_addr` unused**: `RuntimeConfig.bind_addr` is loaded and logged but not used in this task (HTTP server is issue #12). This is intentional — the field exists for forward compatibility.
- **`confidence` type mismatch**: `Constraints.confidence_threshold` is `f64` but `AgentResponse.confidence` is `f32`. This is relevant to `RuntimeAgent.invoke()` implementation, not `main.rs` itself, but the implementer should be aware of it for the overall flow.
- **`AllToolsExist` vs real validation**: The current code uses `skill_loader::AllToolsExist` as the tool checker, which always returns true. Since the `ToolRegistry` now implements `ToolExists`, the refactored code should pass the registry itself (via `Arc<ToolRegistry>`) as the tool checker instead. This gives real validation that registered tools match the skill manifest.

## Verification

1. `cargo check -p agent-runtime` compiles without errors after all prerequisite modules are in place.
2. `cargo clippy -p agent-runtime` produces no warnings.
3. `cargo test -p agent-runtime` passes (existing and new tests).
4. No `println!` calls remain in `main.rs`.
5. No `register_default_tools()` function remains in `main.rs`.
6. No direct `openai::Client` or `rig::providers::openai` import in `main.rs` (delegated to provider module).
7. `main.rs` declares modules: `config`, `provider`, `runtime_agent`, `tool_bridge`.
8. With `SKILL_NAME=echo` and no `TOOL_ENDPOINTS` set, the binary starts and logs the default echo-tool registration and reaches "Agent ready" (assuming echo-tool MCP server is running).
9. With `TOOL_ENDPOINTS=echo-tool=mcp://localhost:7001`, behavior is identical to the default fallback.
10. With `TOOL_ENDPOINTS=foo=mcp://host:1234,bar=mcp://host:5678`, two tools are registered.
11. With `TOOL_ENDPOINTS=invalid-format`, the binary exits with a clear error message about invalid format.
12. Full workspace check: `cargo check` and `cargo test` pass with no regressions.
