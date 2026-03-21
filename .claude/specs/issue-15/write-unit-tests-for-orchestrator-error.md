# Spec: Write unit tests for OrchestratorError

> From: .claude/tasks/issue-15.md

## Objective

Create an integration test file `crates/orchestrator/tests/error_test.rs` that validates the `OrchestratorError` enum's `Display` implementation for all four variants and its `From<OrchestratorError> for AgentError` conversion. This ensures the error type produces correct human-readable messages and integrates properly with the SDK's `AgentError::Internal` boundary, which is the mechanism by which orchestrator errors flow through `MicroAgent::invoke()`.

## Current State

**`crates/agent-sdk/src/agent_error.rs`** -- Defines `AgentError` with five variants, including `Internal(String)`, which is the target of the `From<OrchestratorError>` conversion. Implements manual `Display` and `std::error::Error`. Derives `Debug, Clone, PartialEq, Serialize, Deserialize`.

**`crates/agent-sdk/tests/envelope_types_test.rs`** -- The primary test pattern to follow. Key conventions:
- One `#[test]` function per logical concern, using descriptive snake_case names (e.g., `agent_error_display_contains_expected_substrings`).
- Tests `Display` output by calling `format!("{}", error)` and asserting on substrings with `assert!(display.contains(...), "expected '...' in: {display}")`, using the custom failure message pattern to show what was actually produced.
- Tests error equality using `assert_eq!` and `assert_ne!` on the type directly.
- No test framework beyond `#[test]`; no `tokio::test`; no external test utilities.

**`crates/agent-runtime/src/provider.rs` (inline `#[cfg(test)]` block)** -- The secondary test pattern. Key conventions:
- Tests `Display` using both exact `assert_eq!(err.to_string(), "...")` for simple variants and `assert!(msg.contains(...))` for compound variants.
- Tests each variant in its own `#[test]` function rather than grouping all variants into one test.
- Uses `matches!(err, ProviderError::MissingApiKey { .. })` for variant matching.

**`crates/orchestrator/src/error.rs`** -- Does not yet exist. Per the "Define OrchestratorError enum" spec, it will contain:
- `OrchestratorError` enum with four variants: `NoRoute { input }`, `AgentUnavailable { name, reason }`, `EscalationFailed { chain, reason }`, `HttpError { url, reason }`
- Derives `Debug, Clone` (no `PartialEq`)
- Manual `Display` with these format strings:
  - `NoRoute` -> `"No route found for input: {input}"`
  - `AgentUnavailable` -> `"Agent '{name}' unavailable: {reason}"`
  - `EscalationFailed` -> `"Escalation failed through chain [{chain joined by " -> "}]: {reason}"`
  - `HttpError` -> `"HTTP error calling {url}: {reason}"`
- `impl std::error::Error for OrchestratorError {}` (empty)
- `impl From<OrchestratorError> for AgentError` converting to `AgentError::Internal(err.to_string())`

**`crates/orchestrator/tests/`** -- Directory does not yet exist.

## Requirements

1. Create `crates/orchestrator/tests/error_test.rs` as an integration test file.

2. Test `Display` output for all four `OrchestratorError` variants:
   - `NoRoute { input }` -- verify the display string contains the input value and matches the expected format.
   - `AgentUnavailable { name, reason }` -- verify the display string contains both the agent name and the reason.
   - `EscalationFailed { chain, reason }` -- verify the display string contains the chain elements joined by `" -> "` and the reason.
   - `HttpError { url, reason }` -- verify the display string contains both the URL and the reason.

3. Test `From<OrchestratorError> for AgentError` conversion:
   - For each of the four variants, construct the `OrchestratorError`, convert it to `AgentError` using `.into()`, and verify the result is `AgentError::Internal(String)` where the inner string matches the `Display` output of the original error.

4. Test `EscalationFailed` with edge cases:
   - Empty chain (`chain: vec![]`) -- display should handle gracefully.
   - Single-element chain -- no `" -> "` separator.
   - Multi-element chain -- elements joined by `" -> "`.

5. Verify `OrchestratorError` implements `std::error::Error` (compile-time check via a generic function that accepts `T: std::error::Error`).

6. Follow the test naming convention from the reference files: `{type_under_test}_{behavior_being_tested}` in snake_case (e.g., `orchestrator_error_no_route_display`, `orchestrator_error_converts_to_agent_error_internal`).

7. Use `assert_eq!` for exact display string matching where the format is fully specified, and `assert!(contains)` with custom failure messages for substring checks where exactness is secondary to content verification.

8. Do not use `assert_eq!` directly on `OrchestratorError` values -- the type does not derive `PartialEq`. Use `matches!()` for variant matching and check fields individually.

## Implementation Details

### File to create

**`crates/orchestrator/tests/error_test.rs`**

Imports:
- `use agent_sdk::AgentError;`
- `use orchestrator::error::OrchestratorError;`

Test functions (one per test, following the `provider.rs` pattern of separate functions per variant):

1. **`no_route_display_contains_input`** -- Construct `OrchestratorError::NoRoute { input: "analyze quarterly report".into() }`, call `to_string()`, assert the output equals `"No route found for input: analyze quarterly report"`.

2. **`agent_unavailable_display_contains_name_and_reason`** -- Construct `OrchestratorError::AgentUnavailable { name: "summarizer".into(), reason: "connection refused".into() }`, call `to_string()`, assert the output contains `"summarizer"` and `"connection refused"`. Can also use `assert_eq!` for the exact string `"Agent 'summarizer' unavailable: connection refused"`.

3. **`escalation_failed_display_contains_chain_and_reason`** -- Construct with `chain: vec!["agent-a".into(), "agent-b".into(), "agent-c".into()], reason: "all agents exhausted".into()`, assert the display output contains `"agent-a -> agent-b -> agent-c"` and `"all agents exhausted"`.

4. **`escalation_failed_display_with_single_agent_chain`** -- Construct with `chain: vec!["solo-agent".into()], reason: "declined".into()`, assert display contains `"solo-agent"` without any `" -> "` separator.

5. **`escalation_failed_display_with_empty_chain`** -- Construct with `chain: vec![], reason: "no agents configured".into()`, assert display handles it gracefully (the `[]` in the output will be empty, resulting in `"Escalation failed through chain []: no agents configured"`).

6. **`http_error_display_contains_url_and_reason`** -- Construct `OrchestratorError::HttpError { url: "http://localhost:8080/invoke".into(), reason: "timeout".into() }`, assert the output equals `"HTTP error calling http://localhost:8080/invoke: timeout"`.

7. **`no_route_converts_to_agent_error_internal`** -- Construct `NoRoute`, convert via `let agent_err: AgentError = err.into();`, use `matches!(agent_err, AgentError::Internal(ref msg) if msg.contains("No route"))` or destructure and assert the inner string equals `err_display`.

8. **`agent_unavailable_converts_to_agent_error_internal`** -- Same pattern for `AgentUnavailable`.

9. **`escalation_failed_converts_to_agent_error_internal`** -- Same pattern for `EscalationFailed`.

10. **`http_error_converts_to_agent_error_internal`** -- Same pattern for `HttpError`.

11. **`orchestrator_error_implements_std_error`** -- A compile-time verification function:
    ```rust
    fn assert_is_std_error<T: std::error::Error>(_: &T) {}
    ```
    Call it with each variant to confirm the `Error` trait is implemented. This test passes if it compiles.

### Key patterns from reference files

From `envelope_types_test.rs`:
```rust
let display = format!("{}", tool_call_failed);
assert!(display.contains("web_search"), "expected 'web_search' in: {display}");
```

From `provider.rs` tests:
```rust
let err = ProviderError::UnsupportedProvider { provider: "cohere".to_string() };
assert_eq!(err.to_string(), "unsupported provider: cohere");
```

The test file should use a blend of both: `assert_eq!` for variants with a simple, fully predictable format, and `assert!(contains)` with custom messages for variants where testing key substrings is more robust (particularly `EscalationFailed` with its chain joining logic).

### Integration points

- The test file is an integration test (in `tests/` directory), so it imports `orchestrator::error::OrchestratorError` and `agent_sdk::AgentError` as external crate paths.
- Requires the `orchestrator` crate to be a library crate (depends on the "Convert orchestrator from binary to library crate" task being complete).
- Requires the `orchestrator` Cargo.toml to have `agent-sdk` as a dependency (depends on the "Update orchestrator Cargo.toml with dependencies" task).
- The `orchestrator` Cargo.toml does NOT need a dev-dependency on `agent-sdk` because `agent-sdk` is already a regular dependency -- integration tests can use it directly.

## Dependencies

- **Blocked by**: "Define OrchestratorError enum" (the type under test must exist), which itself is blocked by "Convert orchestrator from binary to library crate" and "Update orchestrator Cargo.toml with dependencies"
- **Blocking**: Nothing -- this is a leaf task in the dependency graph

## Risks & Edge Cases

- **No `PartialEq` on `OrchestratorError`**: Per the error enum spec, `OrchestratorError` does not derive `PartialEq`, so tests cannot use `assert_eq!` on error values directly. Tests must use `matches!()` for variant matching and check individual fields or use `Display` output comparison. The `AgentError` type does derive `PartialEq`, so `assert_eq!` can be used on the converted `AgentError` value directly.
- **Display format coupling**: Tests that use `assert_eq!` on exact display strings are coupled to the format strings in `error.rs`. If the format changes, these tests must be updated. This is intentional -- the tests exist specifically to lock down the display format.
- **EscalationFailed chain join format**: The `" -> "` join format for the chain vector is a specific behavior that must be tested. Edge cases (empty vec, single element) should be covered to prevent panics or unexpected output.
- **Integration test vs inline test**: The task specifies an integration test file (`tests/error_test.rs`) rather than inline `#[cfg(test)]` tests. This is the correct choice since it tests the public API surface of the `error` module as external consumers would use it.

## Verification

1. `cargo test -p orchestrator --test error_test` runs all tests and they pass.
2. `cargo clippy -p orchestrator --tests` produces no warnings on the test file.
3. All four `Display` variant outputs are covered by at least one test.
4. All four `From<OrchestratorError> for AgentError` conversions are covered by at least one test.
5. The `EscalationFailed` chain edge cases (empty, single, multi) are each covered.
6. `cargo test` across the full workspace still passes (no regressions).
