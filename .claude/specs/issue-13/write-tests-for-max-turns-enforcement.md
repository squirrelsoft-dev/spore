# Spec: Write tests for max_turns enforcement

> From: .claude/tasks/issue-13.md

## Objective

Add tests to `crates/agent-runtime/tests/runtime_agent_test.rs` that verify two aspects of max-turns enforcement:

1. When `BuiltAgent::prompt()` returns an error whose string representation contains the rig-core `MaxTurnsError` marker, `RuntimeAgent::invoke()` maps it to `AgentError::MaxTurnsExceeded { turns }` (not `AgentError::Internal`).
2. The `default_max_turns` value from `manifest.constraints.max_turns` is correctly threaded through `build_agent_with_tools()` into the built rig-core agent.

These tests validate the work done in the two upstream tasks ("Set `default_max_turns` from constraints at agent build time" and "Map rig-core MaxTurnsError to AgentError::MaxTurnsExceeded") and serve as the regression safety net before the final verification suite.

## Current State

### `crates/agent-runtime/src/runtime_agent.rs`

`RuntimeAgent` wraps a `BuiltAgent` and implements `MicroAgent`. The `invoke()` method currently maps all errors from `self.agent.prompt()` to `AgentError::Internal(e.to_string())`:

```rust
async fn invoke(&self, request: AgentRequest) -> Result<AgentResponse, AgentError> {
    let output = self
        .agent
        .prompt(&request.input)
        .await
        .map_err(|e| AgentError::Internal(e.to_string()))?;
    Ok(AgentResponse::success(request.id, Value::String(output)))
}
```

After the blocking task "Map rig-core MaxTurnsError to AgentError::MaxTurnsExceeded" is implemented, this method will detect `MaxTurnsError` in the error (either via typed enum matching or substring detection on the stringified `ProviderError::Prompt(String)`) and produce `AgentError::MaxTurnsExceeded { turns: manifest.constraints.max_turns }`.

### `crates/agent-runtime/src/provider.rs`

`BuiltAgent` is an enum with `OpenAi(Agent<OpenAiModel>)` and `Anthropic(Agent<AnthropicModel>)` variants. Its `prompt()` method delegates to the inner rig-core agent and maps errors to `ProviderError::Prompt(String)`, which erases the original `PromptError` type. The `build_agent()` function constructs the agent via `build_agent_with_tools()`.

After the blocking task "Set `default_max_turns` from constraints at agent build time" is implemented, `build_agent()` will pass `manifest.constraints.max_turns` through to `build_agent_with_tools()`, which will call `.default_max_turns(max_turns as usize)` on the builder.

### `crates/agent-runtime/src/tool_bridge.rs`

`build_agent_with_tools()` currently accepts `(builder, tools)` and calls `builder.tools(boxed).build()`. After the upstream task, it will accept an additional `max_turns: u32` parameter and call `builder.default_max_turns(max_turns as usize).tools(boxed).build()`.

### `crates/agent-runtime/tests/runtime_agent_test.rs`

Contains four tests: `test_manifest_returns_correct_values`, `test_health_returns_healthy`, `test_runtime_agent_is_dyn_compatible`, and `test_invoke_with_real_llm` (ignored). Uses a `build_test_runtime_agent()` helper that sets a fake API key and builds a real `RuntimeAgent` via `provider::build_agent()`. None of the existing tests exercise error mapping or max-turns behavior.

### `crates/agent-sdk/src/agent_error.rs`

`AgentError::MaxTurnsExceeded { turns: u32 }` already exists with `Display`, `Debug`, `Clone`, `PartialEq`, `Serialize`, and `Deserialize` implementations.

### Test architecture constraint

`RuntimeAgent::new()` requires a `BuiltAgent`, which is an enum wrapping concrete rig-core `Agent` types. There is no way to inject a mock `BuiltAgent` since it is a closed enum, not a trait. This constrains testing approaches:

- **MaxTurnsError mapping test**: Cannot unit-test `invoke()` in isolation with a mock agent because `BuiltAgent` cannot be mocked. Two viable approaches: (a) add a test variant to `BuiltAgent` (e.g., `#[cfg(test)] Mock(...)`) that can return arbitrary errors, or (b) test the error-mapping logic by extracting it into a standalone function that takes a `Result<String, ProviderError>` and returns `Result<String, AgentError>`, then unit-test that function directly.
- **`default_max_turns` passthrough test**: The built `Agent<M>` struct in rig-core has a `default_max_turns` field. After building an agent via the test helper (which uses `provider::build_agent()`), the test can access the `BuiltAgent` enum, match on the variant, and inspect the inner agent's `default_max_turns` field to verify it equals `manifest.constraints.max_turns as usize`. This requires either: (a) the `Agent` struct's field is `pub`, or (b) there is a getter method. If the field is not accessible, an alternative is to verify indirectly through behavior (prompt a real LLM, which is impractical), or to trust the builder call and test only the wiring.

## Requirements

1. **Test: MaxTurnsError maps to AgentError::MaxTurnsExceeded** -- When `RuntimeAgent::invoke()` encounters a `MaxTurnsError`-pattern error from the underlying agent, it must return `Err(AgentError::MaxTurnsExceeded { turns })` where `turns` equals `manifest.constraints.max_turns`. The test must assert:
   - The result is `Err`, not `Ok`.
   - The error variant is `AgentError::MaxTurnsExceeded`.
   - The `turns` field matches the manifest's `constraints.max_turns`.

2. **Test: non-MaxTurns errors still map to AgentError::Internal** -- When `RuntimeAgent::invoke()` encounters a non-MaxTurns error, it must return `Err(AgentError::Internal(_))` (the existing behavior is preserved).

3. **Test: `default_max_turns` is set on the built agent** -- After building an agent via `provider::build_agent()` with a manifest that has `constraints.max_turns = N`, the resulting rig-core agent's `default_max_turns` field must equal `Some(N as usize)`. The test must verify this by inspecting the built agent.

4. All new tests must be in `crates/agent-runtime/tests/runtime_agent_test.rs`.

5. Tests must not require a valid API key or network access (no `#[ignore]` attribute).

6. Tests must compile and pass with `cargo test -p agent-runtime`.

## Implementation Details

### Files to modify

**`crates/agent-runtime/tests/runtime_agent_test.rs`** -- Add the following tests.

#### Test 1: `test_max_turns_error_maps_to_max_turns_exceeded`

This test verifies the error-mapping logic in `RuntimeAgent::invoke()`. The approach depends on how the blocking task implements the mapping:

- **Option A (extracted function)**: If the blocking task extracts the error-mapping logic into a public helper function (e.g., `pub fn map_prompt_error(err: ProviderError, max_turns: u32) -> AgentError`), the test calls that function directly with a `ProviderError::Prompt(...)` whose string contains the `MaxTurnsError` marker and asserts it returns `AgentError::MaxTurnsExceeded { turns: max_turns }`.

- **Option B (test variant on BuiltAgent)**: If `BuiltAgent` gains a `#[cfg(test)]` mock variant that can return controlled errors, construct a `RuntimeAgent` with that variant, invoke it, and assert the error mapping.

- **Option C (integration-style with substring)**: Build a `RuntimeAgent` via the existing test helper. Since we cannot trigger a real `MaxTurnsError` without an LLM, construct a `ProviderError::Prompt(String)` that contains the rig-core `MaxTurnsError` signature string, and test the mapping function in isolation.

The implementer should choose whichever approach the blocking task's design makes feasible, preferring the approach that provides the most direct test coverage without adding unnecessary complexity.

Regardless of approach, assert:
```rust
assert!(matches!(result, Err(AgentError::MaxTurnsExceeded { turns }) if turns == expected_max_turns));
```

#### Test 2: `test_non_max_turns_error_maps_to_internal`

Same setup as Test 1, but with a generic error string (e.g., `"connection timeout"`) that does not contain the `MaxTurnsError` marker. Assert:
```rust
assert!(matches!(result, Err(AgentError::Internal(_))));
```

#### Test 3: `test_default_max_turns_set_on_built_agent`

Build an agent using the existing `build_test_runtime_agent()` helper (which uses `test_manifest()` with `constraints.max_turns = 1`). Access the inner `BuiltAgent` from `RuntimeAgent` and match on the enum variant to inspect the inner rig-core `Agent`'s `default_max_turns` field.

This requires either:
- A public getter on `RuntimeAgent` that exposes the inner `BuiltAgent` (e.g., `pub fn agent(&self) -> &BuiltAgent`), or
- Making `RuntimeAgent.agent` field `pub(crate)` or `pub`, or
- Testing through `build_agent_with_tools()` directly by calling it from the test and inspecting the returned `Agent`.

If accessing the `Agent`'s `default_max_turns` field directly is not possible (it may be private in rig-core), alternative verification:
- Build two agents with different `max_turns` values and verify both build successfully (compile-time + runtime wiring check) and rely on the error-mapping tests to validate the behavioral effect.
- Or call `build_agent_with_tools()` directly from the test, which is a public function, and check the returned agent.

Assert:
```rust
// If direct field access is available:
assert_eq!(agent.default_max_turns, Some(1));

// If only build verification is possible:
// assert that building with max_turns = N succeeds and the agent is usable
```

#### Test 4: `test_default_max_turns_varies_with_manifest`

Build two agents with different `max_turns` values (e.g., 1 and 10) and verify each has the correct `default_max_turns` set. This confirms the value is threaded from the manifest rather than hardcoded.

### Key functions/types involved

- `RuntimeAgent::invoke()` -- the method under test for error mapping
- `BuiltAgent::prompt()` -- returns `Result<String, ProviderError>`
- `ProviderError::Prompt(String)` -- the error type that wraps the stringified rig-core error
- `AgentError::MaxTurnsExceeded { turns: u32 }` -- the expected mapped error
- `tool_bridge::build_agent_with_tools(builder, tools, max_turns)` -- the function that sets `default_max_turns` (after upstream task)
- `provider::build_agent(manifest, registry)` -- top-level builder that threads `max_turns`

### Integration points with existing code

- Tests import from `agent_runtime::provider`, `agent_runtime::runtime_agent::RuntimeAgent`, `agent_sdk::AgentError`, and `agent_sdk::AgentRequest`.
- Reuse the existing `test_manifest()` and `build_test_runtime_agent()` helpers where possible.
- May need to add a variant of `test_manifest()` that accepts a custom `max_turns` value (e.g., `test_manifest_with_max_turns(max_turns: u32) -> SkillManifest`).

## Dependencies

- Blocked by: "Map rig-core MaxTurnsError to AgentError::MaxTurnsExceeded" -- the error-mapping logic must exist before tests can verify it. Also transitively blocked by "Set `default_max_turns` from constraints at agent build time" since the mapping task depends on it.
- Blocking: "Run verification suite" -- these tests must pass before the final suite run.

## Risks & Edge Cases

1. **`BuiltAgent` is not mockable.** It is a closed enum with concrete provider-specific variants, not a trait. If the blocking task does not expose a test seam (extracted function, test variant, or public error-mapping helper), the error-mapping tests may need to be restructured as unit tests on an extracted helper rather than integration tests on `RuntimeAgent::invoke()`. The implementer of the blocking task should be aware of this testing need.

2. **rig-core `Agent.default_max_turns` field visibility.** If this field is private in rig-core 0.32, the `test_default_max_turns_set_on_built_agent` test cannot directly assert its value. In that case, fall back to verifying the build succeeds with different `max_turns` values (compile-time + runtime wiring check) and rely on the error-mapping tests to validate the behavioral effect.

3. **`MaxTurnsError` string format.** The blocking task maps errors by detecting a `MaxTurnsError` substring (or similar marker) in the stringified `ProviderError::Prompt(String)`. If rig-core changes its error `Display` format in a future version, the substring check and thus the tests may break. Tests should use the same marker string that the production code checks for, ensuring consistency.

4. **Fake API key and agent construction.** The existing test helper sets `OPENAI_API_KEY` to a fake value via `set_var`, which is `unsafe` due to potential data races. Tests are serialized by tokio's single-threaded runtime, mitigating this, but adding more tests that call `build_test_runtime_agent()` increases the surface area. No new risk is introduced as long as all tests use the same pattern.

5. **Test isolation.** If `test_default_max_turns_varies_with_manifest` constructs agents with different manifest values, ensure the `OPENAI_API_KEY` env var is set before each build call (the existing helper already does this).

## Verification

1. `cargo test -p agent-runtime -- runtime_agent_test` runs all new tests and they pass.
2. `cargo test -p agent-runtime` passes with no regressions to existing tests.
3. `cargo clippy -p agent-runtime` produces no warnings in the test file.
4. The `test_max_turns_error_maps_to_max_turns_exceeded` test fails if the error-mapping logic is removed (i.e., it would catch a regression to the old blanket `Internal` mapping).
5. The `test_default_max_turns_set_on_built_agent` test fails if the `default_max_turns()` call is removed from `build_agent_with_tools()`.
