# Spec: Define `AgentRequest` struct

> From: .claude/tasks/issue-3.md

## Objective
Define the `AgentRequest` struct in the `agent-sdk` crate. This is the inbound envelope type that carries a caller's request into any `MicroAgent::invoke` implementation. It assigns each request a unique UUID v4 identifier, carries the input prompt as a `String`, and optionally includes structured context and a caller identifier. This struct is a prerequisite for the `MicroAgent` trait (Group 3) and is part of the issue #4 envelope types pulled into issue #3 because the trait depends on it.

## Current State
- The `agent-sdk` crate lives at `crates/agent-sdk/`.
- Existing structs (`ModelConfig`, `SkillManifest`, `Constraints`, `OutputSchema`) all follow the same pattern:
  - One struct per file, named after the type in snake_case.
  - Derive `Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema`.
  - Imports: `schemars::JsonSchema` and `serde::{Deserialize, Serialize}`.
  - All fields are `pub`.
- `lib.rs` declares each module with `mod <name>;` and re-exports the public type with `pub use <name>::<Type>;`.
- Current dependencies in `Cargo.toml`: `serde` (with `derive`), `schemars` (with `derive`). Notably, `uuid` and `serde_json` are **not yet** dependencies. A sibling task ("Add `uuid` and `serde_json` dependencies") in Group 1 must land first to make them available.

## Requirements
1. Create a new file `crates/agent-sdk/src/agent_request.rs`.
2. Define a public struct `AgentRequest` with the following fields (all `pub`):
   - `id: uuid::Uuid` -- unique identifier for the request, generated at construction time.
   - `input: String` -- the natural-language prompt or instruction.
   - `context: Option<serde_json::Value>` -- optional structured context (e.g., prior conversation state, metadata).
   - `caller: Option<String>` -- optional identifier for the agent or system that originated the request.
3. Derive `Debug, Clone, Serialize, Deserialize` on the struct. Do **not** derive `PartialEq` (because `serde_json::Value` supports it, but `Uuid` equality semantics in tests should use explicit assertions rather than blanket `PartialEq`; however, if the implementor decides `PartialEq` is useful and consistent with other structs, it is acceptable). Do **not** derive `JsonSchema` unless `schemars` adds `uuid` support or a manual `JsonSchema` impl is provided -- see Risks below.
4. Implement an associated function `AgentRequest::new(input: String) -> Self` that:
   - Generates a new UUID v4 via `uuid::Uuid::new_v4()`.
   - Sets `context` to `None`.
   - Sets `caller` to `None`.
5. Add `mod agent_request;` and `pub use agent_request::AgentRequest;` to `lib.rs`. (This may be done by the "Update `lib.rs`" task in Group 3; if so, this task should still document the expectation.)

## Implementation Details

### File: `crates/agent-sdk/src/agent_request.rs` (new)
- Import `serde::{Deserialize, Serialize}` and `uuid::Uuid`.
- Define the struct and derive macros as specified.
- Implement the `new` constructor in an `impl AgentRequest` block.
- The constructor should be straightforward (under 10 lines).

### File: `crates/agent-sdk/src/lib.rs` (modify)
- Add `mod agent_request;` in the module declaration block (alphabetical order with existing modules).
- Add `pub use agent_request::AgentRequest;` in the re-export block.

### Patterns to follow
- Match the single-struct-per-file convention used by `model_config.rs`, `constraints.rs`, etc.
- Keep imports minimal; use fully qualified paths (`uuid::Uuid`) rather than glob imports.

## Dependencies
- **Blocked by**: "Add `uuid` and `serde_json` dependencies to `agent-sdk/Cargo.toml`" (Group 1) -- `uuid` and `serde_json` must be in `Cargo.toml` before this file can compile.
- **Blocking**: "Define `MicroAgent` trait" (Group 3) -- the `MicroAgent::invoke` method accepts `AgentRequest` as its parameter.

## Risks & Edge Cases
1. **`JsonSchema` derive may fail for `uuid::Uuid`**: The existing structs derive `JsonSchema` via `schemars`. The `schemars` crate supports `uuid::Uuid` only if the `uuid1` feature is enabled (`schemars = { version = "0.8", features = ["derive", "uuid1"] }`). If this feature is not added, omit `JsonSchema` from the derive list for `AgentRequest`, or add the feature as part of the dependency task. The implementor should verify and handle accordingly.
2. **`JsonSchema` derive may fail for `serde_json::Value`**: Similarly, `schemars` requires no special feature for `serde_json::Value` (it is supported out of the box), but this should be confirmed at compile time.
3. **UUID v4 requires randomness**: `uuid::Uuid::new_v4()` requires the `v4` feature on the `uuid` crate, which is specified in the dependency task. If the feature is missing, compilation will fail with a clear error.
4. **Serialization format for `Uuid`**: With the `serde` feature enabled on `uuid`, `Uuid` serializes as a hyphenated string (e.g., `"550e8400-e29b-41d4-a716-446655440000"`). This is the expected behavior and requires no custom serializer.
5. **No validation on `input`**: The `new` constructor does not validate that `input` is non-empty. This is intentional -- validation is the responsibility of the agent's `invoke` method or a higher-level orchestrator. If empty-input rejection is needed later, it should be added at the trait or orchestrator level.

## Verification
1. `cargo check` passes with no errors (struct compiles, all types resolve).
2. `cargo clippy` produces no warnings related to `agent_request.rs`.
3. `cargo test` passes (existing tests remain green; new serialization tests for `AgentRequest` are covered by the sibling "Write serialization tests for envelope types" task).
4. Manual review confirms:
   - The struct has exactly the four specified fields with correct types.
   - The `new` constructor generates a UUID v4 and sets optional fields to `None`.
   - The file follows the established single-struct-per-file pattern.
   - `AgentRequest` is re-exported from `lib.rs`.
5. A round-trip serialization test (JSON serialize then deserialize) produces an equal struct -- to be written in `crates/agent-sdk/tests/envelope_types_test.rs` by the sibling test task.
