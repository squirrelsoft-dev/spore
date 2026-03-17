# Spec: Write integration test for RuntimeAgent construction

> From: .claude/tasks/issue-11.md

## Objective

Create an integration test that validates `RuntimeAgent` can be instantiated from an in-memory `SkillManifest` and that its `MicroAgent` trait implementation behaves correctly for non-LLM operations. Specifically, test that `manifest()` returns the correct values, `health()` returns `HealthStatus::Healthy`, and the agent can be used as a `Box<dyn MicroAgent>` trait object. The `invoke()` method requires a live LLM client (e.g., OpenAI API key), so include an invoke test marked `#[ignore]` for manual/CI-with-secrets execution. This test file serves as the integration-level contract test for the `RuntimeAgent` struct before the HTTP layer (issue #12) wraps it.

## Current State

- **`RuntimeAgent` does not exist yet.** It will be created in `crates/agent-runtime/src/runtime_agent.rs` by the "Implement RuntimeAgent struct with MicroAgent trait" task. Per the task description, it will:
  - Hold a `manifest: SkillManifest`, a rig-core `Agent` (generic or type-erased), and a `registry: Arc<ToolRegistry>`.
  - Implement `MicroAgent` where: `manifest()` returns `&self.manifest`, `invoke()` calls the rig-core agent's `.prompt()`, and `health()` returns `HealthStatus::Healthy`.

- **`MicroAgent` trait** (in `agent-sdk`): An `#[async_trait]` trait with three methods: `fn manifest(&self) -> &SkillManifest`, `async fn invoke(&self, request: AgentRequest) -> Result<AgentResponse, AgentError>`, and `async fn health(&self) -> HealthStatus`. The trait is dyn-compatible (`Box<dyn MicroAgent>`).

- **`SkillManifest`** (in `agent-sdk`): A struct with fields `name`, `version`, `description`, `model: ModelConfig`, `preamble`, `tools: Vec<String>`, `constraints: Constraints`, `output: OutputSchema`. All fields are public and the struct derives `Debug, Clone, PartialEq`.

- **Existing test patterns:**
  - `crates/agent-sdk/tests/micro_agent_test.rs` uses a `MockAgent` struct with a `make_manifest()` helper that constructs a `SkillManifest` in-memory. Tests validate `manifest()`, `invoke()`, and `health()` independently. Each test is a separate `#[tokio::test] async fn` with a descriptive name.
  - `crates/skill-loader/tests/skill_loader_test.rs` uses `AllToolsExist` stub and `Arc<ToolRegistry>` for test setup.
  - `tools/echo-tool/tests/echo_server_test.rs` uses `#[tokio::test(flavor = "multi_thread", worker_threads = 2)]` for MCP server tests.
  - No mocking frameworks are used anywhere in the workspace. Stubs are hand-written.

- **`provider::build_agent()`** (from the "Create provider module" task): Will take a `&SkillManifest` and `Vec<McpTool>` and return a built rig-core `Agent`. It reads `OPENAI_API_KEY` from env for the OpenAI provider. This means constructing a `RuntimeAgent` for tests requires either a real API key or a way to build the agent without invoking the LLM.

- **`tool_bridge` module** (`crates/agent-runtime/src/tool_bridge.rs`): Contains `resolve_mcp_tools()` (needs connected registry handles) and `build_agent_with_tools()` (takes an `AgentBuilder` and `Vec<McpTool>`, returns `Agent`).

- **Current `Cargo.toml` dev-dependencies** for `agent-runtime`: none listed (only regular dependencies exist). The test file will need `tokio` and `agent-sdk` at minimum; `agent-sdk` is already a regular dependency so it is available in tests.

## Requirements

1. **File location:** Create `crates/agent-runtime/tests/runtime_agent_test.rs` as an integration test file.

2. **In-memory manifest construction:** Tests must construct a `SkillManifest` directly in Rust code (not loaded from a file), following the same pattern as `make_manifest()` in `crates/agent-sdk/tests/micro_agent_test.rs`. Use the echo skill's values (provider: `"openai"`, model name: `"gpt-4o"`, temperature: `0.0`, tools: `[]`, max_turns: `1`, confidence_threshold: `1.0`, format: `"text"`, empty schema, preamble: `"Echo back the input exactly as received."`).

3. **Test: `manifest_returns_correct_values`** -- Construct a `RuntimeAgent` with the in-memory manifest. Call `manifest()` and assert all fields match the constructed values: `name`, `version`, `description`, `model.provider`, `model.name`, `model.temperature`, `preamble`, `tools` (empty vec), `constraints.max_turns`, `constraints.confidence_threshold`, `output.format`.

4. **Test: `health_returns_healthy`** -- Construct a `RuntimeAgent` and call `health().await`. Assert the return value equals `HealthStatus::Healthy`.

5. **Test: `runtime_agent_is_dyn_compatible`** -- Construct a `RuntimeAgent`, box it as `Box<dyn MicroAgent>`, and verify `manifest()` and `health()` work through the trait object. This validates that the agent can be stored as `Arc<dyn MicroAgent>` for the HTTP layer.

6. **Test: `invoke_with_real_llm`** -- Marked `#[ignore]` with a doc comment: `// Requires OPENAI_API_KEY environment variable`. Construct a `RuntimeAgent` with a real OpenAI client, send an `AgentRequest` with input `"hello"`, and assert the response has a non-empty output and the correct `id`. This test validates the full invoke path but only runs when explicitly opted in (e.g., `cargo test -p agent-runtime -- --ignored`).

7. **Construction helper:** Since `RuntimeAgent` construction depends on the provider module (which needs an API key for OpenAI), the non-invoke tests need a way to build a `RuntimeAgent` without a live LLM. Two approaches:
   - **Option A (preferred):** If `RuntimeAgent::new()` accepts an already-built rig-core `Agent`, construct a minimal agent with a placeholder key (the agent won't be called in manifest/health tests). The OpenAI client constructor (`openai::Client::new("placeholder")`) may succeed since it only validates the key format, not connectivity.
   - **Option B:** If the `RuntimeAgent` constructor requires going through `provider::build_agent()`, set `OPENAI_API_KEY=test-placeholder` in the test env. The agent object will be constructed but never invoked, so no real API call occurs.

   The implementer should choose whichever approach works with the actual `RuntimeAgent` constructor API. The key constraint is: `manifest()` and `health()` tests must not require a real API key and must not make network calls.

8. **Dev-dependencies:** Add `tokio = { version = "1", features = ["macros", "rt-multi-thread"] }` to `[dev-dependencies]` in `crates/agent-runtime/Cargo.toml` if not already present. Also ensure `agent-sdk` types are available (they should be, since `agent-sdk` is a regular dependency).

9. **No file-based skill loading:** Tests must not depend on the `skills/` directory or any `.md` files on disk. All manifests are constructed in-memory.

10. **No MCP tool server spawning:** Tests in this file do not start any MCP tool servers. The manifest uses `tools: []` so no tool resolution is needed for construction.

## Implementation Details

### File to create: `crates/agent-runtime/tests/runtime_agent_test.rs`

**Imports needed:**
- `agent_sdk::{SkillManifest, ModelConfig, Constraints, OutputSchema, HealthStatus, MicroAgent, AgentRequest}` -- for constructing the manifest and asserting trait behavior.
- `agent_runtime::RuntimeAgent` -- the struct under test (requires `RuntimeAgent` to be `pub` and re-exported from `agent_runtime`'s `lib.rs` or accessible as a public module path).
- `std::collections::HashMap` -- for constructing the empty `OutputSchema.schema`.
- `std::sync::Arc` -- if `RuntimeAgent` needs `Arc<ToolRegistry>`.
- `tool_registry::ToolRegistry` -- if `RuntimeAgent` constructor requires it.

**Helper function: `make_echo_manifest()`**

Constructs a `SkillManifest` matching the echo skill:
```
name: "echo"
version: "1.0"
description: "Echoes input back for testing"
model: { provider: "openai", name: "gpt-4o", temperature: 0.0 }
preamble: "Echo back the input exactly as received. Do not modify, summarize, or interpret."
tools: []
constraints: { max_turns: 1, confidence_threshold: 1.0, escalate_to: None, allowed_actions: [] }
output: { format: "text", schema: {} }
```

**Helper function: `make_runtime_agent(manifest)`**

Encapsulates the boilerplate of constructing a `RuntimeAgent` from a manifest. This will depend on the actual `RuntimeAgent` constructor signature (determined by the blocked-by task). It should:
1. Create a `ToolRegistry` (empty, no connections needed).
2. Build a rig-core agent with a placeholder API key (no network calls will be made in non-invoke tests).
3. Return the `RuntimeAgent`.

**Test functions:**

| # | Test name | Async | Attribute | Purpose |
|---|-----------|-------|-----------|---------|
| 1 | `manifest_returns_correct_values` | yes | `#[tokio::test]` | Assert all manifest fields match |
| 2 | `health_returns_healthy` | yes | `#[tokio::test]` | Assert health is `Healthy` |
| 3 | `runtime_agent_is_dyn_compatible` | yes | `#[tokio::test]` | Box as `dyn MicroAgent`, call manifest + health |
| 4 | `invoke_with_real_llm` | yes | `#[tokio::test]`, `#[ignore]` | Full invoke with real API key |

### File to modify: `crates/agent-runtime/Cargo.toml`

Add to `[dev-dependencies]`:
```toml
[dev-dependencies]
tokio = { version = "1", features = ["macros", "rt-multi-thread"] }
```

The `agent-sdk` and `tool-registry` crates are already regular dependencies, so their types are available in integration tests without re-listing them.

### File to modify: `crates/agent-runtime/src/main.rs` (or `lib.rs`)

The `RuntimeAgent` struct must be publicly accessible from integration tests. Since `agent-runtime` is currently a binary crate (only has `main.rs`), the implementer may need to:
- Create a `crates/agent-runtime/src/lib.rs` that re-exports `RuntimeAgent` and other public types.
- Or restructure `main.rs` to use `mod runtime_agent; pub use runtime_agent::RuntimeAgent;` in a lib target.

This is a consideration for the "Implement RuntimeAgent struct" task, not this test task. However, the test spec should note this dependency: if `RuntimeAgent` is not publicly importable from `agent_runtime::RuntimeAgent`, the integration test cannot compile.

### Integration points

- The test depends on the `RuntimeAgent` struct's public constructor API, which will be defined by the "Implement RuntimeAgent struct with MicroAgent trait" task.
- The test depends on the `provider` module for constructing the rig-core agent (or on `RuntimeAgent` accepting a pre-built agent).
- The `#[ignore]` invoke test depends on `OPENAI_API_KEY` being set in the environment.

## Dependencies

- **Blocked by:**
  - "Refactor main.rs to use config and RuntimeAgent" -- the `RuntimeAgent` struct and its `MicroAgent` impl must exist and be publicly accessible. The provider module must exist for agent construction.
  - Transitively: "Implement RuntimeAgent struct with MicroAgent trait" and "Create provider module for LLM client construction".

- **Blocking:**
  - "Run verification suite" -- the verification task runs `cargo test -p agent-runtime` and depends on these tests existing and passing.

## Risks & Edge Cases

1. **`RuntimeAgent` constructor API is unknown.** The struct does not exist yet. The test helpers will need to be adapted once the actual constructor is implemented. The spec describes the intent (construct with in-memory manifest, no network calls for non-invoke tests) and the implementer must adapt to the actual API. If `RuntimeAgent::new()` is generic over the model type, the helper may need type annotations.

2. **rig-core `Agent` type is generic.** The `Agent<M, P>` type in rig-core is generic over the completion model and prompt hook. If `RuntimeAgent` is also generic, the test will need to specify concrete type parameters. If `RuntimeAgent` uses type erasure (e.g., `Box<dyn CompletionModel>`), this is simpler. The implementer should check the actual `RuntimeAgent` definition.

3. **Placeholder API key may fail at construction.** If `openai::Client::new("placeholder")` validates the key format (e.g., must start with `sk-`), use `"sk-test-placeholder-key-for-unit-tests"` instead. The key only needs to pass construction validation, not authenticate with the API. If even client construction fails without a valid-looking key, consider using `std::env::set_var("OPENAI_API_KEY", "sk-test...")` in the test setup and cleaning up after.

4. **Binary crate accessibility.** Integration tests in `tests/` can only access public items from a library crate (`lib.rs`), not from `main.rs`. If `agent-runtime` remains a pure binary crate, integration tests cannot import `RuntimeAgent`. The implementer of the RuntimeAgent task must ensure a library target exists. This test spec assumes `agent_runtime::RuntimeAgent` is importable.

5. **`confidence` type mismatch.** The task notes mention that `Constraints.confidence_threshold` is `f64` but `AgentResponse.confidence` is `f32`. The invoke test should use approximate float comparison (`(response.confidence - expected).abs() < f32::EPSILON`) rather than exact equality.

6. **Test isolation for `#[ignore]` invoke test.** The ignored test depends on `OPENAI_API_KEY` in the environment. It makes a real network call and may be slow or flaky. It should use `tokio::time::timeout` to avoid hanging if the API is unresponsive. A 30-second timeout is reasonable for a single LLM call.

7. **Empty tools vector.** The echo skill has no tools (`tools: []`). This means `resolve_mcp_tools` should return an empty vec and no MCP connections are needed. This simplifies test setup significantly.

## Verification

1. `cargo check -p agent-runtime --tests` compiles with no errors (requires RuntimeAgent to exist first).
2. `cargo clippy -p agent-runtime --tests` reports no warnings on the integration test file.
3. `cargo test -p agent-runtime --test runtime_agent_test` runs the non-ignored tests and all pass.
4. `manifest_returns_correct_values` asserts every field of the manifest matches the constructed values.
5. `health_returns_healthy` confirms `HealthStatus::Healthy` is returned.
6. `runtime_agent_is_dyn_compatible` confirms the agent works as `Box<dyn MicroAgent>`.
7. `invoke_with_real_llm` is skipped by default (shown as "ignored" in test output) and runs successfully when `OPENAI_API_KEY` is set and `--ignored` flag is passed.
8. No tests depend on files on disk, network connectivity (except the ignored test), or MCP tool servers.
9. `cargo test` across the full workspace still passes (no regressions).
