# Task Breakdown: Create agent-runtime crate with rig-core integration

> Wire together SkillLoader, ToolRegistry, and rig-core into a production-ready startup flow driven by environment variables, with a `MicroAgent` trait implementation that serves as the contract for the HTTP layer (issue #12).

## Group 1 — Configuration and provider modules

_Tasks in this group can be done in parallel._

- [x] **Create config module for environment-driven settings** `[S]`
      Create `crates/agent-runtime/src/config.rs` that reads all runtime configuration from environment variables. Define a `RuntimeConfig` struct with fields: `skill_name: String` (from `SKILL_NAME`, required), `skill_dir: PathBuf` (from `SKILL_DIR`, default `./skills`), `bind_addr: SocketAddr` (from `BIND_ADDR`, default `0.0.0.0:8080`). Add a `RuntimeConfig::from_env()` constructor that reads env vars and returns `Result<Self, ConfigError>`. Define `ConfigError` enum with `MissingVar { name }` and `InvalidValue { name, value, reason }` variants. Keep it simple — no config files, just env vars.
      Files: `crates/agent-runtime/src/config.rs`
      Blocking: "Refactor main.rs to use config and RuntimeAgent"

- [x] **Create provider module for LLM client construction** `[M]`
      Create `crates/agent-runtime/src/provider.rs` with a `build_agent()` function that takes a `&SkillManifest` and a `Vec<McpTool>` and returns the built rig-core `Agent`. The function should read `ModelConfig.provider` to select the correct rig-core provider (`"openai"` → `rig::providers::openai::Client` with `OPENAI_API_KEY` from env). Use `ModelConfig.name` as the model identifier and `manifest.preamble` as the system prompt. Delegate to `tool_bridge::build_agent_with_tools()` for tool attachment. Since rig-core 0.32 bundles providers without feature flags, verify which providers are available by checking `rig::providers::*`. If only OpenAI is available, support `"openai"` and return a clear error for unsupported providers. Add `tracing` dependency and use `tracing::info!` for startup logging instead of `println!`.
      Files: `crates/agent-runtime/src/provider.rs`, `crates/agent-runtime/Cargo.toml`
      Blocking: "Implement RuntimeAgent struct with MicroAgent trait"

- [x] **Add tracing dependency and replace println with structured logging** `[S]`
      Add `tracing = "0.1"` and `tracing-subscriber = { version = "0.3", features = ["env-filter"] }` to `crates/agent-runtime/Cargo.toml`. Initialize the tracing subscriber in `main()` before any other work. Replace all `println!` calls in `main.rs` with `tracing::info!` calls. This enables `RUST_LOG` env var for log level control.
      Files: `crates/agent-runtime/Cargo.toml`, `crates/agent-runtime/src/main.rs`
      Blocking: "Refactor main.rs to use config and RuntimeAgent"

## Group 2 — RuntimeAgent implementation

_Depends on: Group 1._

- [x] **Implement RuntimeAgent struct with MicroAgent trait** `[M]`
      Create `crates/agent-runtime/src/runtime_agent.rs`. Define `RuntimeAgent` struct holding: `manifest: SkillManifest`, `agent: Agent` (the built rig-core agent — use a type-erased wrapper or generic), and `registry: Arc<ToolRegistry>`. Implement `MicroAgent` for `RuntimeAgent`: `manifest()` returns `&self.manifest`; `invoke()` calls the rig-core agent's `.prompt(&request.input).await`, wraps the result in `AgentResponse::success(request.id, output_value)`, and maps errors to `AgentError::Internal`; `health()` returns `HealthStatus::Healthy`. Note: rig-core's `Agent` type is generic over `M: CompletionModel` and `P: PromptHook<M>`, so the struct will need to be generic or use `Box<dyn CompletionModel>` if available — check rig-core API. Add `agent-sdk` re-export of `async_trait` to the impl.
      Files: `crates/agent-runtime/src/runtime_agent.rs`
      Blocked by: "Create provider module for LLM client construction"
      Blocking: "Refactor main.rs to use config and RuntimeAgent"

- [x] **Refactor main.rs to use config and RuntimeAgent** `[M]`
      Rewrite `main.rs` to use the new modules. Flow: (1) init tracing subscriber, (2) `RuntimeConfig::from_env()?` to load config, (3) create `ToolRegistry`, (4) register tools — for now keep `register_default_tools()` but read endpoint from `TOOL_ENDPOINTS` env var (comma-separated `name=endpoint` pairs, e.g. `echo-tool=mcp://localhost:7001`) with fallback to hardcoded defaults, (5) `registry.connect_all().await`, (6) create `SkillLoader` with `config.skill_dir` and load `config.skill_name`, (7) resolve MCP tools via `tool_bridge`, (8) build agent via `provider::build_agent()`, (9) construct `RuntimeAgent`, (10) wrap in `Arc<dyn MicroAgent>` and log readiness. Remove `register_default_tools()` function. The agent is ready for HTTP serving (issue #12) but this issue stops at construction — no HTTP server yet.
      Files: `crates/agent-runtime/src/main.rs`
      Blocked by: "Create config module for environment-driven settings", "Add tracing dependency and replace println with structured logging", "Implement RuntimeAgent struct with MicroAgent trait"
      Blocking: "Write unit tests for config and provider modules"

## Group 3 — Tests and verification

_Depends on: Group 2._

- [x] **Write unit tests for config and provider modules** `[M]`
      Create `crates/agent-runtime/tests/config_test.rs` with tests: missing `SKILL_NAME` returns error, valid env produces correct `RuntimeConfig`, default values for optional vars. Create `crates/agent-runtime/tests/provider_test.rs` with tests: unsupported provider returns error, missing API key returns error. Use `std::env::set_var` in tests (with `#[serial]` if needed, or use temp env). Add `serial_test` as dev-dependency if needed for env var isolation, or use a helper that restores env state.
      Files: `crates/agent-runtime/tests/config_test.rs`, `crates/agent-runtime/tests/provider_test.rs`, `crates/agent-runtime/Cargo.toml`
      Blocked by: "Refactor main.rs to use config and RuntimeAgent"
      Blocking: "Run verification suite"

- [x] **Write integration test for RuntimeAgent construction** `[M]`
      Create `crates/agent-runtime/tests/runtime_agent_test.rs`. Test that given a valid `SkillManifest` (constructed in-memory, not loaded from file), a `RuntimeAgent` can be instantiated and `manifest()` returns the correct values. Test `health()` returns `HealthStatus::Healthy`. For `invoke()`, this requires a real LLM client which won't be available in CI — mark the invoke test as `#[ignore]` with a comment explaining it requires `OPENAI_API_KEY`. Alternatively, test the full flow up to agent construction (without invoke) by loading the echo skill with `AllToolsExist` stub.
      Files: `crates/agent-runtime/tests/runtime_agent_test.rs`
      Blocked by: "Refactor main.rs to use config and RuntimeAgent"
      Blocking: "Run verification suite"

- [x] **Run verification suite** `[S]`
      Run `cargo check -p agent-runtime`, `cargo clippy -p agent-runtime`, and `cargo test -p agent-runtime` to verify everything compiles, has no warnings, and tests pass. Also run `cargo check` and `cargo test` across the full workspace to ensure no regressions.
      Files: (none — command-line only)
      Blocked by: "Write unit tests for config and provider modules", "Write integration test for RuntimeAgent construction"

## Notes for implementers

1. **rig-core 0.32 providers**: The current Cargo.lock shows rig-core 0.32 includes `rig::providers::openai` (used in existing `main.rs`). Check if `rig::providers::anthropic` is available — if not, document this limitation and support only OpenAI initially.
2. **Agent type erasure**: rig-core's `Agent<M, P>` is generic. To store it in `RuntimeAgent` (which implements `dyn MicroAgent`), you'll need either: (a) make `RuntimeAgent` generic and box it as `Box<dyn MicroAgent>`, or (b) use rig-core's trait objects if available. Option (a) is simpler.
3. **No HTTP server in this issue**: The `MicroAgent` trait implementation is the handoff point. Issue #12 will wrap `Arc<dyn MicroAgent>` in axum routes.
4. **Tool registration is still semi-hardcoded**: Full dynamic tool discovery is out of scope. The `TOOL_ENDPOINTS` env var is a pragmatic middle ground until a proper config system is built.
5. **`confidence` type mismatch**: `Constraints.confidence_threshold` is `f64` but `AgentResponse.confidence` is `f32`. Cast appropriately in RuntimeAgent when setting confidence.
