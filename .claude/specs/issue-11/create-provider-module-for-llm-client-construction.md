# Spec: Create provider module for LLM client construction

> From: .claude/tasks/issue-11.md

## Objective

Create a `provider` module in the `agent-runtime` crate that encapsulates LLM client construction. The module provides a single `build_agent()` function that reads a `SkillManifest`, selects the correct rig-core provider based on `ModelConfig.provider`, constructs the LLM client with the appropriate API key from the environment, and returns a fully-configured rig-core `Agent` with MCP tools attached. This centralizes provider selection logic so that `main.rs` (and the future `RuntimeAgent`) can build agents without knowing provider-specific details.

## Current State

- **`main.rs`** currently hardcodes `openai::Client::new("placeholder-key")` and calls `openai_client.agent("gpt-4o")` directly. There is no provider abstraction or env-based API key reading.
- **`tool_bridge.rs`** already provides `resolve_mcp_tools()` and `build_agent_with_tools(builder, tools)`. The latter takes an `AgentBuilder<M, P, NoToolConfig>` and a `Vec<McpTool>`, attaches the tools, and returns `Agent<M, P>`.
- **`SkillManifest`** contains `model: ModelConfig` with fields `provider: String`, `name: String`, `temperature: f64`.
- **rig-core 0.32.0** bundles all providers without feature flags. Available providers include: `openai`, `anthropic`, `gemini`, `cohere`, `deepseek`, `groq`, `mistral`, `ollama`, `azure`, `xai`, `perplexity`, `together`, `openrouter`, `huggingface`, `hyperbolic`, `mira`, `moonshot`, `galadriel`, `voyageai`.
- **rig-core provider API**: Each provider exposes a `Client` type alias (e.g., `rig::providers::openai::Client`) that implements `CompletionClient`. Clients are constructed via `Client::builder().api_key(key).build()` or `Client::new(key)`. The `ProviderClient` trait provides `from_env()` which reads provider-specific env vars (e.g., `OPENAI_API_KEY`, `ANTHROPIC_API_KEY`), but it panics on missing keys. The `client.agent(model_name).preamble(text).build()` chain produces an `Agent<M>`.
- **`Agent<M, P>`** is generic over the completion model type `M` and prompt hook `P`. Different providers produce different concrete `M` types (e.g., `openai::responses_api::ResponsesCompletionModel` vs `anthropic::completion::CompletionModel`). This means `build_agent()` cannot return a single concrete `Agent<M>` type without type erasure or an enum dispatch approach.
- **No `tracing` dependency** exists in `agent-runtime/Cargo.toml` yet (a sibling task will add it, but this module should use `tracing::info!` for its own logging).

## Requirements

- Create `crates/agent-runtime/src/provider.rs` with a public `build_agent()` function.
- The function signature must accept `manifest: &SkillManifest` and `tools: Vec<McpTool>`, and return a `Result` with a meaningful error type.
- Read `manifest.model.provider` to select the rig-core provider (`"openai"`, `"anthropic"`, etc.).
- Read the API key from the appropriate environment variable for each provider (e.g., `OPENAI_API_KEY` for `"openai"`, `ANTHROPIC_API_KEY` for `"anthropic"`). Do NOT use `ProviderClient::from_env()` because it panics on missing keys; instead read the env var manually and return a descriptive error.
- Use `manifest.model.name` as the model identifier passed to `client.agent(model_name)`.
- Use `manifest.preamble` as the system prompt via `.preamble()`.
- Use `manifest.model.temperature` via `.temperature()` on the builder.
- Delegate to `tool_bridge::build_agent_with_tools()` for tool attachment.
- Add `tracing = "0.1"` to `crates/agent-runtime/Cargo.toml` dependencies.
- Use `tracing::info!` to log provider selection and agent construction (e.g., `"Building agent with provider={}, model={}"`) instead of `println!`.
- Support at minimum `"openai"` provider. Support `"anthropic"` as a second provider if feasible.
- Return a clear, descriptive error for unsupported provider strings.
- Return a clear error for missing API key env vars.

## Implementation Details

### Files to create/modify

1. **`crates/agent-runtime/src/provider.rs`** (new file)
2. **`crates/agent-runtime/Cargo.toml`** (add `tracing` dependency)
3. **`crates/agent-runtime/src/main.rs`** (add `mod provider;` declaration)

### Key types and functions

#### Error type: `ProviderError`

Define an enum for provider construction failures:

```rust
#[derive(Debug)]
pub enum ProviderError {
    UnsupportedProvider { provider: String },
    MissingApiKey { provider: String, env_var: String },
    ClientBuild(String),
}
```

Implement `std::fmt::Display` and `std::error::Error` for `ProviderError`.

#### Return type challenge: Agent type erasure

Because `Agent<M, P>` is generic over the completion model and different providers produce different `M` types, `build_agent()` cannot return `Agent<SomeConcreteType>` for multiple providers. Two approaches:

- **Option A (Recommended): Enum dispatch.** Define a `BuiltAgent` enum with one variant per supported provider. The downstream `RuntimeAgent` (in a blocking task) will match on this enum to call the appropriate `.prompt()` method. This avoids trait object complexity and keeps types concrete.

  ```rust
  pub enum BuiltAgent {
      OpenAI(Agent<openai_completion_model_type>),
      Anthropic(Agent<anthropic_completion_model_type>),
  }
  ```

- **Option B: Make `build_agent()` generic or use separate functions.** Provide `build_openai_agent()`, `build_anthropic_agent()`, etc., and let the caller dispatch. This is simpler but pushes dispatch logic to the caller.

- **Option C: Use rig-core's dynamic client if available.** Check if rig-core 0.32 offers a `dyn CompletionModel` or `Box<dyn CompletionModel>` pattern. If `CompletionModel` is object-safe, the agent could be `Agent<Box<dyn CompletionModel>>`.

The implementer should check whether `CompletionModel` is object-safe in rig-core 0.32. If it is, use `Box<dyn CompletionModel>` for a clean return type. If not, use Option A (enum dispatch) since `RuntimeAgent` (the blocking task) will need to handle each variant anyway.

#### `build_agent()` function

```rust
pub fn build_agent(
    manifest: &SkillManifest,
    tools: Vec<McpTool>,
) -> Result<BuiltAgent, ProviderError> {
    // 1. Read manifest.model.provider
    // 2. Match on provider string
    // 3. Read API key from env (return ProviderError::MissingApiKey on failure)
    // 4. Construct provider client via Client::builder().api_key(key).build()
    // 5. Call client.agent(manifest.model.name).preamble(manifest.preamble).temperature(manifest.model.temperature)
    // 6. Pass builder + tools to tool_bridge::build_agent_with_tools()
    // 7. Log with tracing::info!
    // 8. Return wrapped agent
}
```

#### Environment variable mapping

| Provider       | Env var              |
|----------------|----------------------|
| `"openai"`     | `OPENAI_API_KEY`     |
| `"anthropic"`  | `ANTHROPIC_API_KEY`  |

#### Provider-specific notes

- **OpenAI**: `rig::providers::openai::Client` defaults to the Responses API. The existing `main.rs` uses `openai::Client::new(key)?` and `client.agent(model)`. The `Client::new()` returns `http_client::Result<Self>` so the error must be mapped to `ProviderError::ClientBuild`.
- **Anthropic**: `rig::providers::anthropic::Client` uses `Client::builder().api_key(key).build()`. It uses `x-api-key` header instead of Bearer auth.

### Cargo.toml change

Add to `[dependencies]`:
```toml
tracing = "0.1"
```

### Module registration

Add `mod provider;` to `main.rs` (or `lib.rs` if one exists). Currently `main.rs` only has `mod tool_bridge;`, so add `mod provider;` alongside it.

## Dependencies

- **Blocked by**: Nothing (Group 1 task, can start immediately)
- **Blocking**: "Implement RuntimeAgent struct with MicroAgent trait" -- `RuntimeAgent` will store the `BuiltAgent` (or equivalent) and call into it for `invoke()`.

## Risks & Edge Cases

1. **`Agent<M, P>` is not type-erasable**: If `CompletionModel` is not object-safe (likely, given it uses associated types and generics), the return type must use enum dispatch or separate builder functions. The implementer must verify object safety and choose accordingly.
2. **OpenAI Responses API vs Completions API**: rig-core 0.32 defaults to the Responses API (`openai::Client`). If the skill requires the traditional Chat Completions API, the implementer would need `client.completions_api()`. For now, use the default (Responses API) since the existing `main.rs` uses it.
3. **`ProviderClient::from_env()` panics**: Must NOT use this. Read env vars manually with `std::env::var()` and convert `Err` to `ProviderError::MissingApiKey`.
4. **`Client::new()` can fail**: `openai::Client::new(key)` returns `Result`. Map this error to `ProviderError::ClientBuild`.
5. **Temperature type**: `ModelConfig.temperature` is `f64`. The `AgentBuilder::temperature()` method also takes `f64`, so no conversion is needed.
6. **Concurrency with sibling tasks**: The "Add tracing dependency" task also modifies `Cargo.toml`. If done in parallel, merge conflicts on `Cargo.toml` are possible. The `tracing` dependency added here is the same one that task would add, so either task can add it.
7. **Future provider expansion**: The design should make it straightforward to add new providers (e.g., `"gemini"`, `"deepseek"`) by adding match arms and env var mappings.

## Verification

- `cargo check -p agent-runtime` compiles without errors after adding `mod provider;` to `main.rs`.
- `cargo clippy -p agent-runtime` produces no warnings related to the new module.
- The `build_agent()` function is callable from `main.rs` (replacing the current hardcoded OpenAI construction), though full integration is deferred to the "Refactor main.rs" task.
- Unit tests (in the sibling "Write unit tests" task) will verify:
  - Passing `provider: "unsupported_provider"` returns `ProviderError::UnsupportedProvider`.
  - Missing `OPENAI_API_KEY` when `provider: "openai"` returns `ProviderError::MissingApiKey`.
  - With a valid (but fake) API key, `build_agent()` constructs without error (client builds succeed with any string key; actual API validation happens at request time).
