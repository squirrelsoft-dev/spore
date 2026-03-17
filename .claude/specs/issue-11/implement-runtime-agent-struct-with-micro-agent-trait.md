# Spec: Implement RuntimeAgent struct with MicroAgent trait

> From: .claude/tasks/issue-11.md

## Objective

Create a `RuntimeAgent` struct in the `agent-runtime` crate that wraps a rig-core `Agent` and implements the `MicroAgent` trait from `agent-sdk`. This is the bridge between the rig-core LLM agent and the spore micro-agent abstraction, allowing the runtime agent to be stored as `Arc<dyn MicroAgent>` for the HTTP serving layer (issue #12). The struct holds the skill manifest, the built rig-core agent, and a reference to the tool registry, and delegates `invoke()` to rig-core's `Prompt::prompt()` method.

## Current State

- **`MicroAgent` trait** (`crates/agent-sdk/src/micro_agent.rs`): An `#[async_trait]` trait with three methods: `fn manifest(&self) -> &SkillManifest`, `async fn invoke(&self, request: AgentRequest) -> Result<AgentResponse, AgentError>`, and `async fn health(&self) -> HealthStatus`. The trait is dyn-compatible (`Box<dyn MicroAgent>` / `Arc<dyn MicroAgent>`).

- **`AgentRequest`** (`crates/agent-sdk/src/agent_request.rs`): Contains `id: Uuid`, `input: String`, `context: Option<Value>`, `caller: Option<String>`.

- **`AgentResponse`** (`crates/agent-sdk/src/agent_response.rs`): Contains `id: Uuid`, `output: Value`, `confidence: f32`, `escalated: bool`, `tool_calls: Vec<ToolCallRecord>`. Has `AgentResponse::success(id, output)` constructor that sets `confidence: 1.0`, `escalated: false`, `tool_calls: vec![]`.

- **`AgentError`** (`crates/agent-sdk/src/agent_error.rs`): Enum with `ToolCallFailed`, `ConfidenceTooLow`, `MaxTurnsExceeded`, and `Internal(String)` variants.

- **`HealthStatus`** (`crates/agent-sdk/src/health_status.rs`): Enum with `Healthy`, `Degraded(String)`, `Unhealthy(String)` variants.

- **`agent-sdk` re-exports**: `lib.rs` re-exports `async_trait` from the `async_trait` crate via `pub use async_trait::async_trait;`.

- **rig-core 0.32 `Agent<M, P>`** (`rig::agent::Agent`): A generic struct parameterized over `M: CompletionModel` and `P: PromptHook<M>` (default `P = ()`). The `Agent` implements `rig::completion::Prompt`, where `prompt(&self, input)` returns a `PromptRequest` that implements `IntoFuture` with `Output = Result<String, PromptError>`. So calling `agent.prompt("input").await` yields `Result<String, PromptError>`. The `PromptError` enum includes `CompletionError`, `ToolError`, `ToolServerError`, `MaxTurnsError`, and `PromptCancelled` variants.

- **`CompletionModel` is NOT object-safe**: The `CompletionModel` trait has associated types (`Response`, `StreamingResponse`, `Client`) and uses `impl Future` return types, making it non-dyn-compatible. This means `Box<dyn CompletionModel>` is not possible.

- **`PromptHook<M>` for `()`**: The unit type `()` implements `PromptHook<M>` for all `M: CompletionModel`, so `Agent<M>` (i.e., `Agent<M, ()>`) is the common case when no hook is used.

- **Provider module** (`crates/agent-runtime/src/provider.rs`, from sibling task): Will define a `BuiltAgent` enum with one variant per supported provider (e.g., `OpenAI(Agent<openai_model_type>)`, `Anthropic(Agent<anthropic_model_type>)`). The `build_agent()` function returns `Result<BuiltAgent, ProviderError>`.

- **`ToolRegistry`** (`crates/tool-registry/src/tool_registry.rs`): Holds registered tool entries. Wrapped in `Arc<ToolRegistry>` throughout the runtime.

- **`agent-runtime` is a binary crate**: Currently only has `main.rs` and `tool_bridge.rs`. No `lib.rs` exists. Integration tests (from sibling task) need `RuntimeAgent` to be publicly accessible, which requires either creating a `lib.rs` or restructuring.

## Requirements

- Define a `RuntimeAgent` struct in `crates/agent-runtime/src/runtime_agent.rs` that holds: `manifest: SkillManifest`, the rig-core agent (via the `BuiltAgent` enum from the provider module), and `registry: Arc<ToolRegistry>`.
- Implement the `MicroAgent` trait for `RuntimeAgent` using `agent_sdk::async_trait`.
- `manifest()` returns `&self.manifest`.
- `invoke()` calls rig-core's `Prompt::prompt()` on the inner agent with `request.input`, wraps the `String` result in `AgentResponse::success(request.id, serde_json::Value::String(output))`, and maps any `PromptError` to `AgentError::Internal(error.to_string())`.
- `health()` returns `HealthStatus::Healthy` (no health check logic for now; the LLM provider is assumed reachable).
- Provide a `RuntimeAgent::new(manifest, agent, registry)` constructor.
- The struct and its constructor must be `pub` so integration tests and `main.rs` can use it.
- Add `mod runtime_agent;` to `main.rs` (alongside existing `mod tool_bridge;`).

## Implementation Details

### Files to create

**`crates/agent-runtime/src/runtime_agent.rs`** (new file)

#### `RuntimeAgent` struct

Because `Agent<M, P>` is generic and `CompletionModel` is not object-safe, the struct cannot use `Box<dyn CompletionModel>`. Instead, it stores the `BuiltAgent` enum from the `provider` module, which has one variant per supported provider with the concrete `Agent<M>` type inside each variant.

```rust
use std::sync::Arc;

use agent_sdk::{
    async_trait, AgentError, AgentRequest, AgentResponse, HealthStatus,
    MicroAgent, SkillManifest,
};
use rig::completion::Prompt;
use serde_json::Value;
use tool_registry::ToolRegistry;

use crate::provider::BuiltAgent;

pub struct RuntimeAgent {
    manifest: SkillManifest,
    agent: BuiltAgent,
    registry: Arc<ToolRegistry>,
}
```

#### Constructor

```rust
impl RuntimeAgent {
    pub fn new(
        manifest: SkillManifest,
        agent: BuiltAgent,
        registry: Arc<ToolRegistry>,
    ) -> Self {
        Self { manifest, agent, registry }
    }
}
```

#### `MicroAgent` implementation

The `invoke()` method must dispatch on the `BuiltAgent` enum to call `prompt()` on the concrete agent type. Each match arm calls `variant.prompt(&request.input).await`, which returns `Result<String, PromptError>`. The string output is wrapped in `Value::String(...)` and passed to `AgentResponse::success()`.

```rust
#[async_trait]
impl MicroAgent for RuntimeAgent {
    fn manifest(&self) -> &SkillManifest {
        &self.manifest
    }

    async fn invoke(&self, request: AgentRequest) -> Result<AgentResponse, AgentError> {
        let output = match &self.agent {
            BuiltAgent::OpenAI(agent) => {
                agent.prompt(&request.input).await
            }
            BuiltAgent::Anthropic(agent) => {
                agent.prompt(&request.input).await
            }
            // Add match arms for additional provider variants as they are added
        }
        .map_err(|e| AgentError::Internal(e.to_string()))?;

        Ok(AgentResponse::success(
            request.id,
            Value::String(output),
        ))
    }

    async fn health(&self) -> HealthStatus {
        HealthStatus::Healthy
    }
}
```

#### Key design decisions

1. **Enum dispatch over generic struct**: Making `RuntimeAgent` a concrete (non-generic) struct that holds a `BuiltAgent` enum is preferred over making `RuntimeAgent<M, P>` generic. The generic approach would propagate type parameters through `main.rs` and make `Arc<dyn MicroAgent>` wrapping awkward (you would need to explicitly specify the generic parameters at the boxing site). With the enum, `RuntimeAgent` is a single concrete type that directly implements `dyn MicroAgent`.

2. **`Prompt` trait import**: The `rig::completion::Prompt` trait must be in scope for `.prompt()` to be callable on `Agent<M, P>`. Import it in `runtime_agent.rs`.

3. **Output as `Value::String`**: The `Prompt::prompt()` method returns `String`. Wrapping it in `Value::String(...)` is the simplest mapping. If structured output is needed in the future, `invoke()` could attempt `serde_json::from_str()` to parse JSON strings into `Value` objects, but for now plain string wrapping is correct.

4. **`serde_json` dependency**: Already available transitively through `agent-sdk` (which depends on `serde_json`). Verify it is also listed in `agent-runtime/Cargo.toml` or add it if needed.

### Files to modify

**`crates/agent-runtime/src/main.rs`**

Add the module declaration:
```rust
mod runtime_agent;
```

This should be added alongside the existing `mod tool_bridge;` declaration. The "Refactor main.rs" task will handle the actual usage of `RuntimeAgent` in the main function.

**`crates/agent-runtime/Cargo.toml`**

Verify `serde_json` is available. If not already a dependency, add:
```toml
serde_json = "1"
```

### Integration points

- **Depends on `provider::BuiltAgent`**: The `BuiltAgent` enum type from the provider module determines the match arms in `invoke()`. If the provider module adds new variants (e.g., `Gemini`, `DeepSeek`), corresponding match arms must be added to `invoke()`.
- **Consumed by `main.rs`**: The "Refactor main.rs" task constructs a `RuntimeAgent` via `RuntimeAgent::new(manifest, agent, registry)` and wraps it in `Arc<dyn MicroAgent>`.
- **Consumed by integration tests**: The "Write integration test" task constructs `RuntimeAgent` and tests `manifest()`, `health()`, and `invoke()` (the latter as `#[ignore]`).

## Dependencies

- **Blocked by**: "Create provider module for LLM client construction" -- `RuntimeAgent` stores a `BuiltAgent` from the provider module. The `BuiltAgent` enum definition and its variants must exist before this task can compile.
- **Blocking**: "Refactor main.rs to use config and RuntimeAgent" -- `main.rs` needs `RuntimeAgent` to construct the final `Arc<dyn MicroAgent>`.

## Risks & Edge Cases

1. **`BuiltAgent` enum shape is unknown until provider task completes.** The exact variant names and inner types depend on the provider module implementation. The spec assumes `BuiltAgent::OpenAI(Agent<...>)` and `BuiltAgent::Anthropic(Agent<...>)` based on the provider spec, but the implementer must adapt the match arms to the actual enum definition.

2. **`Prompt` trait uses `impl Into<Message>` for the prompt argument.** The `prompt()` method accepts `impl Into<Message> + WasmCompatSend`. A `&String` (or `&str`) should convert to `Message` automatically via rig-core's `From` implementations. If not, the implementer may need to call `.clone()` on `request.input` or convert it explicitly. Check rig-core's `Message` type for `From<String>` / `From<&str>` implementations.

3. **`async_trait` and `Prompt` interaction.** The `MicroAgent` trait uses `#[async_trait]` (which desugars async methods to `Pin<Box<dyn Future>>`), while rig-core's `Prompt::prompt()` returns `impl IntoFuture`. These should compose correctly since `prompt().await` resolves the future inline before the `async_trait` boxing happens. No compatibility issues are expected.

4. **`confidence` type mismatch.** `AgentResponse::success()` sets `confidence: 1.0` (as `f32`), and `Constraints.confidence_threshold` is `f64`. This is acceptable for now since `invoke()` uses the `success()` constructor. If confidence scoring is added later, the cast from `f64` to `f32` should use `as f32` and document potential precision loss.

5. **Thread safety.** `RuntimeAgent` must be `Send + Sync` (required by `MicroAgent: Send + Sync`). `SkillManifest` is `Send + Sync` (derives `Clone`, `Serialize`, etc.). `Arc<ToolRegistry>` is `Send + Sync`. `BuiltAgent` must also be `Send + Sync` -- rig-core's `Agent<M, P>` is `Send + Sync` when `M` and `P` are (which they are, given `CompletionModel: WasmCompatSend + WasmCompatSync` and `PromptHook: WasmCompatSend + WasmCompatSync`). The provider module's `BuiltAgent` enum should be `Send + Sync` automatically.

6. **`registry` field may be unused initially.** The `registry` field is stored for potential future use (e.g., dynamic tool re-resolution, health checking of tool connections) but is not accessed in any of the three `MicroAgent` methods. The implementer should add `#[allow(dead_code)]` or use it in `health()` if clippy warns. Alternatively, `health()` could check `registry` connection status in a future iteration.

7. **Error granularity.** Mapping all `PromptError` variants to `AgentError::Internal` loses specificity. In particular, `PromptError::MaxTurnsError` could map to `AgentError::MaxTurnsExceeded`, and `PromptError::ToolError` could map to `AgentError::ToolCallFailed`. However, for the initial implementation, the blanket `Internal(e.to_string())` mapping is acceptable. This can be refined later.

8. **Binary crate accessibility for tests.** `RuntimeAgent` defined in `main.rs`'s module tree (via `mod runtime_agent;`) is not accessible to integration tests in `tests/`. The "Refactor main.rs" or "Write integration test" tasks may need to create a `lib.rs` that re-exports `RuntimeAgent`. This is noted here as a coordination point but is not this task's responsibility to resolve.

## Verification

- `cargo check -p agent-runtime` compiles without errors after the provider module is in place and `mod runtime_agent;` is added to `main.rs`.
- `cargo clippy -p agent-runtime` produces no warnings related to the new module (allow `dead_code` on `registry` if needed).
- The `RuntimeAgent` struct can be constructed with `RuntimeAgent::new(manifest, built_agent, registry)`.
- `manifest()` returns a reference to the stored `SkillManifest`.
- `health()` returns `HealthStatus::Healthy`.
- `invoke()` delegates to the rig-core agent's `prompt()` method and wraps the result correctly (tested via the integration test task with `#[ignore]` for the live LLM call).
- `RuntimeAgent` is `Send + Sync` and can be wrapped in `Arc<dyn MicroAgent>`.
- Full workspace `cargo check` and `cargo test` pass with no regressions.
