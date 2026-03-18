# Spec: Set `default_max_turns` from constraints at agent build time

> From: .claude/tasks/issue-13.md

## Objective

Wire the `max_turns` value from a skill manifest's `Constraints` into rig-core's `AgentBuilder::default_max_turns()` so that the built agent natively enforces a turn limit during its `PromptRequest` loop. This is the foundational step for max-turns enforcement: once the builder receives the limit, rig-core will automatically return `PromptError::MaxTurnsError` when the agent exceeds it, and downstream code (the "Map rig-core MaxTurnsError to AgentError::MaxTurnsExceeded" task) can translate that into a domain-specific error.

## Current State

### `crates/agent-runtime/src/tool_bridge.rs`

`build_agent_with_tools()` is a generic helper that attaches MCP tools to a rig-core `AgentBuilder` and calls `.build()`:

```rust
pub fn build_agent_with_tools<M, P>(
    builder: AgentBuilder<M, P, NoToolConfig>,
    tools: Vec<McpTool>,
) -> Agent<M, P>
where
    M: CompletionModel,
    P: PromptHook<M>,
{
    let boxed: Vec<Box<dyn ToolDyn>> = tools
        .into_iter()
        .map(|t| Box::new(t) as Box<dyn ToolDyn>)
        .collect();
    builder.tools(boxed).build()
}
```

The builder transitions from `NoToolConfig` to a tools-configured state via `.tools(boxed)`, then `.build()` finalizes it. There is no call to `.default_max_turns()`. The function has no knowledge of constraints.

### `crates/agent-runtime/src/provider.rs`

Two internal helpers call `build_agent_with_tools`:

- `build_openai_agent(model_name, preamble, temperature, tools)` -- constructs an OpenAI `AgentBuilder`, sets preamble and temperature, then delegates to `tool_bridge::build_agent_with_tools(builder, tools)`.
- `build_anthropic_agent(model_name, preamble, temperature, tools)` -- same pattern for Anthropic.

The top-level `build_agent()` function receives the full `&SkillManifest` (which contains `manifest.constraints.max_turns: u32`) but only extracts `provider`, `model_name`, `preamble`, and `temperature` before dispatching to the helpers. `constraints` is not passed through.

### `crates/agent-sdk/src/constraints.rs`

```rust
pub struct Constraints {
    pub max_turns: u32,
    pub confidence_threshold: f64,
    pub escalate_to: Option<String>,
    pub allowed_actions: Vec<String>,
}
```

`max_turns` is a `u32`. The rig-core `AgentBuilder::default_max_turns()` accepts `usize`.

### Builder method ordering

Per the task breakdown notes, `.default_max_turns()` is available on any `ToolState` of `AgentBuilder`. The chain must be `.default_max_turns(n).tools(t).build()` -- i.e., `default_max_turns` is called on the `NoToolConfig` builder before `.tools()` transitions it to the tools-configured state.

## Requirements

1. `build_agent_with_tools()` must accept a `max_turns: u32` parameter in addition to its existing `builder` and `tools` parameters.
2. `build_agent_with_tools()` must call `builder.default_max_turns(max_turns as usize)` before calling `.tools(boxed).build()`.
3. `build_openai_agent()` and `build_anthropic_agent()` must accept a `max_turns: u32` parameter and forward it to `build_agent_with_tools()`.
4. `build_agent()` must extract `manifest.constraints.max_turns` and pass it to the provider-specific helpers.
5. No new crate dependencies may be added.
6. The function signatures in `tool_bridge.rs` must remain generic over `M: CompletionModel` and `P: PromptHook<M>`.
7. `cargo check --workspace`, `cargo clippy --workspace`, and `cargo test --workspace` must all pass after the change.

## Implementation Details

### Files to modify

1. **`crates/agent-runtime/src/tool_bridge.rs`**
   - Add `max_turns: u32` as the third parameter to `build_agent_with_tools()`:
     ```rust
     pub fn build_agent_with_tools<M, P>(
         builder: AgentBuilder<M, P, NoToolConfig>,
         tools: Vec<McpTool>,
         max_turns: u32,
     ) -> Agent<M, P>
     ```
   - Insert `.default_max_turns(max_turns as usize)` on the builder before `.tools(boxed)`:
     ```rust
     builder
         .default_max_turns(max_turns as usize)
         .tools(boxed)
         .build()
     ```
   - Update the doc comment to mention that `max_turns` configures rig-core's native turn enforcement.

2. **`crates/agent-runtime/src/provider.rs`**
   - Add `max_turns: u32` as a parameter to both `build_openai_agent()` and `build_anthropic_agent()`.
   - Forward `max_turns` to `tool_bridge::build_agent_with_tools(builder, tools, max_turns)` in both helpers.
   - In `build_agent()`, extract `manifest.constraints.max_turns` and pass it to the match arms:
     ```rust
     let max_turns = manifest.constraints.max_turns;
     match provider {
         "openai" => build_openai_agent(model_name, preamble, temperature, tools, max_turns),
         "anthropic" => build_anthropic_agent(model_name, preamble, temperature, tools, max_turns),
         ...
     }
     ```

### No new types or modules

This task only modifies function signatures and adds a single builder method call. No new structs, enums, traits, or modules are introduced.

### Integration points

- **Downstream consumer**: The "Map rig-core MaxTurnsError to AgentError::MaxTurnsExceeded" task depends on this change. Once `default_max_turns` is set, rig-core's `PromptRequest` loop will produce `PromptError::MaxTurnsError` when the limit is hit. That error flows through `BuiltAgent::prompt()` as `ProviderError::Prompt(String)`, which the downstream task will inspect and map to `AgentError::MaxTurnsExceeded`.
- **No impact on HTTP layer**: The HTTP handler calls `MicroAgent::invoke()`, which calls `BuiltAgent::prompt()`. The max-turns limit is enforced inside the rig-core prompt loop, transparent to the HTTP layer.

## Dependencies

- Blocked by: none
- Blocking: "Map rig-core MaxTurnsError to AgentError::MaxTurnsExceeded"

## Risks & Edge Cases

- **`max_turns: 0` in manifest**: If a skill file sets `max_turns: 0`, this would be passed as `default_max_turns(0)`. Depending on rig-core's implementation, this could either mean "no limit" or "zero turns allowed" (immediate error). The `Constraints` validation in `skill-loader` should already reject `max_turns: 0` as invalid, but the implementer should verify this. If rig-core treats `0` as "unlimited", there is no issue; if it treats `0` as "zero turns", the validation layer is the correct place to guard against it.
- **Type cast overflow**: `max_turns` is `u32` and `default_max_turns` takes `usize`. On all supported platforms (64-bit), `u32` fits in `usize` without truncation. On a hypothetical 16-bit platform, `u32 as usize` would truncate, but this is not a realistic deployment target.
- **No runtime tests in this task**: This task is purely a build-time wiring change. Verifying that rig-core actually enforces the limit requires an integration test with a real (or mocked) LLM that triggers multiple tool-call turns, which is out of scope here and covered by the "Write tests for max_turns enforcement" task in Group 5.
- **Builder method availability**: The task breakdown notes state `.default_max_turns()` is available on `AgentBuilder` in the `NoToolConfig` state. If the rig-core 0.32 API differs (e.g., the method is on a different state or has a different name), compilation will fail immediately, making the issue easy to diagnose.

## Verification

1. `cargo check --workspace` compiles with zero errors, confirming that `AgentBuilder::default_max_turns()` exists and accepts `usize`.
2. `cargo clippy --workspace` produces no new warnings.
3. `cargo test --workspace` passes all existing tests with no regressions. (Existing tests do not exercise `default_max_turns` behavior, but they must not break from the signature changes.)
4. Read the modified `build_agent_with_tools()` and confirm `.default_max_turns(max_turns as usize)` is called before `.tools(boxed).build()`.
5. Read the modified `build_agent()` and confirm `manifest.constraints.max_turns` is extracted and threaded through to both provider helpers.
