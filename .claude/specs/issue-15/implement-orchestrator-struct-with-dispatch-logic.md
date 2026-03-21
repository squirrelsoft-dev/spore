# Spec: Implement Orchestrator struct with dispatch logic

> From: .claude/tasks/issue-15.md

## Objective

Create the `Orchestrator` struct in `crates/orchestrator/src/orchestrator.rs` that serves as the central dispatch hub for the agent registry. The orchestrator receives incoming `AgentRequest`s, routes them to the appropriate downstream `AgentEndpoint`, handles escalation chains when agents signal low confidence, and enforces safety limits on recursion depth. This is the core logic layer that later gets wrapped with the `MicroAgent` trait (a separate task) so the orchestrator itself can run on `agent-runtime` as a homogeneous micro agent.

Semantic routing via `SemanticRouter` is deferred to issue #16. This task implements a placeholder routing strategy: exact name matching from `request.context`, falling back to substring matching against endpoint descriptions.

## Current State

**Orchestrator crate** (`crates/orchestrator/`) is a skeleton with only a `main.rs` containing `println!("Hello, world!")` and an empty `Cargo.toml`. The crate will be converted to a library crate by a prerequisite task (Group 1).

**SDK types used by this task** (all in `crates/agent-sdk/src/`):

- `AgentRequest` -- has fields: `id: Uuid`, `input: String`, `context: Option<Value>`, `caller: Option<String>`. The `context` field is a `serde_json::Value` where routing information like `{"target_agent": "agent-name"}` will be placed.
- `AgentResponse` -- has fields: `id: Uuid`, `output: Value`, `confidence: f32`, `escalated: bool`, `escalate_to: Option<String>`, `tool_calls: Vec<ToolCallRecord>`. The `escalated` and `escalate_to` fields drive the escalation chain logic.
- `SkillManifest` -- has fields: `name: String`, `version: String`, `description: String`, `model: ModelConfig`, `preamble: String`, `tools: Vec<String>`, `constraints: Constraints`, `output: OutputSchema`. The orchestrator holds its own manifest to describe itself.
- `HealthStatus` -- enum with `Healthy`, `Degraded(String)`, `Unhealthy(String)`.
- `MicroAgent` trait -- defines `manifest()`, `invoke()`, `health()`. The orchestrator will implement this trait in a separate task (Group 4).

**Prerequisite types created by sibling tasks** (not yet implemented):

- `AgentEndpoint` (from `crates/orchestrator/src/agent_endpoint.rs`) -- struct with `name: String`, `description: String`, `url: String`, and methods `invoke(&self, request: &AgentRequest) -> Result<AgentResponse, OrchestratorError>` and `health(&self) -> Result<HealthStatus, OrchestratorError>`. Uses `reqwest` to call downstream agent HTTP endpoints.
- `OrchestratorError` (from `crates/orchestrator/src/error.rs`) -- enum with variants: `NoRoute { input: String }`, `AgentUnavailable { name: String, reason: String }`, `EscalationFailed { chain: Vec<String>, reason: String }`, `HttpError { url: String, reason: String }`. Implements `Display`, `Error`, and `From<OrchestratorError> for AgentError`.
- `OrchestratorConfig` / `AgentConfig` (from `crates/orchestrator/src/config.rs`) -- YAML/env-deserializable config with `agents: Vec<AgentConfig>` where each `AgentConfig` has `name`, `description`, `url`.

**Patterns to follow:**

- The `ConstraintEnforcer` in `crates/agent-runtime/src/constraint_enforcer.rs` demonstrates the decorator pattern around `MicroAgent` and how escalation fields are set.
- The `RuntimeConfig::from_env()` in `crates/agent-runtime/src/config.rs` demonstrates the config-loading pattern.
- Error types follow manual `Display + Error` implementations (not `thiserror`), as seen in `AgentError` and `ConfigError`.

## Requirements

- **R1**: Define a `pub struct Orchestrator` with fields `registry: HashMap<String, AgentEndpoint>` and `manifest: SkillManifest`.
- **R2**: Implement `Orchestrator::new(manifest: SkillManifest, agents: Vec<AgentEndpoint>) -> Self` that populates the `registry` HashMap keyed by each agent's `name` field.
- **R3**: Implement `Orchestrator::register(&mut self, endpoint: AgentEndpoint)` that inserts a single agent into the registry, keyed by its name. If an agent with the same name already exists, it is replaced.
- **R4**: Implement `Orchestrator::route(&self, request: &AgentRequest) -> Result<&AgentEndpoint, OrchestratorError>` with two-phase lookup:
  - Phase 1: If `request.context` is `Some(Value::Object(map))` and the map contains a `"target_agent"` key with a string value, look up that name in the registry. Return the endpoint if found, or `OrchestratorError::NoRoute` if not.
  - Phase 2 (fallback): If no `target_agent` is specified in context, iterate all registry entries and check if `request.input` (lowercased) contains any word from the endpoint's `description` (lowercased, split by whitespace). Return the first match. This is a placeholder heuristic until `SemanticRouter` (issue #16) is implemented.
  - Return `OrchestratorError::NoRoute { input }` if neither phase produces a match.
- **R5**: Implement `Orchestrator::dispatch(&self, request: AgentRequest) -> Result<AgentResponse, OrchestratorError>` that:
  - Calls `self.route(&request)` to find the target endpoint.
  - Delegates to `try_invoke()` to perform the health check and invocation.
  - Delegates to `handle_escalation()` if the response indicates escalation.
  - Returns the final `AgentResponse` or an appropriate `OrchestratorError`.
- **R6**: Implement helper `try_invoke(&self, endpoint: &AgentEndpoint, request: &AgentRequest) -> Result<AgentResponse, OrchestratorError>` that:
  - Calls `endpoint.health()`. If the health status is `Unhealthy`, return `OrchestratorError::AgentUnavailable`.
  - If health is `Healthy` or `Degraded`, call `endpoint.invoke(request)` and return the result.
- **R7**: Implement helper `handle_escalation(&self, response: AgentResponse, chain: Vec<String>) -> Result<AgentResponse, OrchestratorError>` that:
  - If `response.escalated` is `false`, return the response as-is.
  - If `response.escalated` is `true` and `response.escalate_to` is `Some(name)`:
    - Check the escalation chain length against `MAX_ESCALATION_DEPTH` (constant, value `5`). If exceeded, return `OrchestratorError::EscalationFailed { chain, reason: "max escalation depth exceeded" }`.
    - Check if `name` is already in `chain` to detect cycles. If so, return `OrchestratorError::EscalationFailed { chain, reason: "escalation cycle detected" }`.
    - Look up the escalation target in the registry. If not found, return `OrchestratorError::EscalationFailed { chain, reason: "escalation target not found" }`.
    - Construct a new `AgentRequest` with the same `id` and `input`, but with `context` set to `{"target_agent": name}` and `caller` set to the previous agent's name.
    - Call `try_invoke()` on the escalation target, append the target name to `chain`, and recursively call `handle_escalation()` on the result.
  - If `response.escalated` is `true` but `response.escalate_to` is `None`, return the response as-is (the agent signaled low confidence but provided no escalation target).
- **R8**: Implement `Orchestrator::from_config(config: OrchestratorConfig) -> Result<Self, OrchestratorError>` that:
  - Iterates `config.agents` and constructs `AgentEndpoint::new(name, description, url)` for each.
  - Constructs a default `SkillManifest` for the orchestrator itself (name: `"orchestrator"`, version: `"0.1.0"`, description: `"Routes requests to specialized agents"`, with sensible defaults for model, preamble, tools, constraints, and output).
  - Calls `Orchestrator::new()` with the manifest and endpoints.
- **R9**: Every public method must be under 50 lines. The `dispatch` method must be decomposed into `try_invoke` and `handle_escalation` helpers.
- **R10**: The struct must be designed so that a `SemanticRouter` field can be added later (issue #16) without major refactoring. The `route()` method should be the single point where routing logic lives, making it straightforward to replace the substring heuristic with semantic routing.
- **R11**: `dispatch`, `try_invoke`, and `handle_escalation` must be `async` methods since `AgentEndpoint::invoke()` and `AgentEndpoint::health()` are async (they make HTTP calls via `reqwest`).

## Implementation Details

### File to create

**`crates/orchestrator/src/orchestrator.rs`**

### Constants

```rust
const MAX_ESCALATION_DEPTH: usize = 5;
```

### Struct definition

```rust
use std::collections::HashMap;
use agent_sdk::{AgentRequest, AgentResponse, HealthStatus, SkillManifest};
use crate::agent_endpoint::AgentEndpoint;
use crate::config::OrchestratorConfig;
use crate::error::OrchestratorError;

pub struct Orchestrator {
    registry: HashMap<String, AgentEndpoint>,
    manifest: SkillManifest,
}
```

### Method signatures

```rust
impl Orchestrator {
    pub fn new(manifest: SkillManifest, agents: Vec<AgentEndpoint>) -> Self
    pub fn register(&mut self, endpoint: AgentEndpoint)
    pub fn route(&self, request: &AgentRequest) -> Result<&AgentEndpoint, OrchestratorError>
    pub async fn dispatch(&self, request: AgentRequest) -> Result<AgentResponse, OrchestratorError>
    pub fn from_config(config: OrchestratorConfig) -> Result<Self, OrchestratorError>
}

// Private helpers
impl Orchestrator {
    async fn try_invoke(&self, endpoint: &AgentEndpoint, request: &AgentRequest) -> Result<AgentResponse, OrchestratorError>
    async fn handle_escalation(&self, response: AgentResponse, chain: Vec<String>) -> Result<AgentResponse, OrchestratorError>
}
```

### Routing logic detail

In `route()`:
1. Extract `target_agent` from context: `request.context.as_ref().and_then(|v| v.get("target_agent")).and_then(|v| v.as_str())`.
2. If a target name is found, do `self.registry.get(name)` and return a reference, or return `NoRoute`.
3. If no target name, iterate `self.registry.values()` and for each endpoint, check if any whitespace-delimited word from the endpoint's `description` (lowercased) appears as a substring in `request.input` (lowercased). Return the first match.
4. Return `OrchestratorError::NoRoute { input: request.input.clone() }` if nothing matches.

### Escalation logic detail

In `handle_escalation()`:
1. If `!response.escalated`, return `Ok(response)` immediately.
2. Extract `escalate_to` name. If `None`, return `Ok(response)`.
3. Check `chain.len() >= MAX_ESCALATION_DEPTH` for depth limit.
4. Check `chain.contains(&name)` for cycle detection.
5. Look up `name` in `self.registry`.
6. Build a new `AgentRequest` preserving the original `id` and `input`, setting `context` to `json!({"target_agent": name})` and `caller` to `chain.last()`.
7. Call `self.try_invoke(target, &new_request)`.
8. Append `name` to `chain` and recurse into `handle_escalation(result, chain)`.

### `from_config` logic detail

Build `AgentEndpoint` instances from each `AgentConfig` entry. Construct a default `SkillManifest`:
- `name`: `"orchestrator"`
- `version`: `"0.1.0"`
- `description`: `"Routes requests to specialized agents"`
- `model`: `ModelConfig { provider: "none".into(), name: "none".into(), temperature: 0.0 }` (orchestrator does not use an LLM)
- `preamble`: empty string
- `tools`: empty vec
- `constraints`: `Constraints { max_turns: 1, confidence_threshold: 0.0, escalate_to: None, allowed_actions: vec![] }`
- `output`: `OutputSchema { format: "json".into(), schema: HashMap::new() }`

### Integration points

- **Imports from `agent-sdk`**: `AgentRequest`, `AgentResponse`, `HealthStatus`, `SkillManifest`, `ModelConfig`, `Constraints`, `OutputSchema`.
- **Imports from sibling modules**: `AgentEndpoint` (from `crate::agent_endpoint`), `OrchestratorConfig` (from `crate::config`), `OrchestratorError` (from `crate::error`).
- **The `MicroAgent` trait implementation** will be added in a separate task (Group 4) in the same file. The `dispatch()` method is designed to be called by `MicroAgent::invoke()` with error conversion.
- **`serde_json`** is needed for constructing context JSON (`json!()` macro) in escalation re-dispatch.

### Module declaration

The parent `lib.rs` (created by a prerequisite task) must contain `pub mod orchestrator;` to expose this module.

## Dependencies

- **Blocked by**:
  - "Convert orchestrator from binary to library crate" -- `lib.rs` must exist and declare `pub mod orchestrator;`
  - "Update orchestrator Cargo.toml with dependencies" -- `agent-sdk`, `serde_json`, `tokio`, and `async-trait` must be in `Cargo.toml`
  - "Define OrchestratorError enum" -- `crate::error::OrchestratorError` must exist with `NoRoute`, `AgentUnavailable`, `EscalationFailed`, `HttpError` variants
  - "Implement AgentEndpoint struct" -- `crate::agent_endpoint::AgentEndpoint` must exist with `name`, `description`, `invoke()`, and `health()`
  - "Define registry config format and loader" -- `crate::config::OrchestratorConfig` and `AgentConfig` must exist

- **Blocking**:
  - "Implement MicroAgent for Orchestrator" -- needs `dispatch()` to delegate from `MicroAgent::invoke()`
  - "Write unit tests for Orchestrator dispatch and routing" -- tests exercise all methods defined here

## Risks & Edge Cases

- **Infinite escalation loops**: Agent A escalates to Agent B, which escalates back to Agent A. Mitigated by cycle detection (checking if the target name is already in the escalation chain) and the hard cap at `MAX_ESCALATION_DEPTH = 5`.
- **Escalation target not in registry**: An agent may specify `escalate_to: Some("unknown-agent")`. The `handle_escalation` method must handle this gracefully with `EscalationFailed` rather than panicking.
- **Race condition on health checks**: An agent might become unhealthy between the health check and the invocation. This is acceptable for now; the `invoke()` call will fail with an `HttpError` which propagates cleanly. A retry-with-fallback strategy is out of scope.
- **Multiple substring matches in route()**: The fallback heuristic may match multiple agents. The current design returns the first match found during `HashMap` iteration, which has nondeterministic ordering. This is acceptable because the substring heuristic is explicitly a placeholder -- `SemanticRouter` (issue #16) will replace it with ranked scoring. Document this limitation with a code comment.
- **Empty registry**: If no agents are registered, all `route()` calls return `NoRoute`. This is valid behavior, not an error in construction.
- **Large input strings**: The substring matching iterates all registry entries and does string comparisons. With a small registry (expected: tens of agents, not thousands), this is not a performance concern.
- **Context field conflicts**: If `request.context` contains `"target_agent"` but the value is not a string (e.g., it is a number or object), the extraction via `as_str()` returns `None` and falls through to the heuristic. This is safe but could be surprising. Add a comment noting this behavior.
- **`from_config` default manifest**: The default `SkillManifest` uses placeholder values for `model` and `output` since the orchestrator does not use an LLM. If these fields are later validated elsewhere, the placeholders must pass validation. Using `"none"` for provider/model and `"json"` for output format (which is in `ALLOWED_OUTPUT_FORMATS`) avoids this.

## Verification

- **Compilation**: `cargo check -p orchestrator` succeeds with no errors (requires all prerequisite tasks to be complete).
- **Lint**: `cargo clippy -p orchestrator` produces no warnings.
- **Unit tests** (defined in the sibling test task, but verifying this task's correctness):
  1. `new()` populates registry with correct keys; agents are retrievable by name.
  2. `register()` adds an agent that can be routed to; replacing an existing name overwrites cleanly.
  3. `route()` returns the correct endpoint when `context` contains `target_agent`.
  4. `route()` falls back to substring matching when no `target_agent` is in context.
  5. `route()` returns `NoRoute` when no agent matches.
  6. `dispatch()` calls `route()`, checks health, invokes the agent, and returns the response.
  7. `dispatch()` returns `AgentUnavailable` when the target agent is `Unhealthy`.
  8. `dispatch()` handles a single escalation hop correctly.
  9. `dispatch()` returns `EscalationFailed` when escalation depth exceeds `MAX_ESCALATION_DEPTH`.
  10. `dispatch()` returns `EscalationFailed` when an escalation cycle is detected.
  11. `dispatch()` returns `EscalationFailed` when the escalation target is not in the registry.
  12. `from_config()` builds a valid `Orchestrator` from an `OrchestratorConfig`.
  13. All methods are under 50 lines.
- **Integration**: The orchestrator's `dispatch()` method signature is compatible with the planned `MicroAgent::invoke()` delegation pattern (takes `AgentRequest`, returns `Result<AgentResponse, _>` with error convertible to `AgentError`).
