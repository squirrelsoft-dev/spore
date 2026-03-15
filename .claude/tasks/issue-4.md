# Task Breakdown: Define AgentRequest/AgentResponse envelope types

> Define the standardized request/response envelope types in `agent-sdk` that form the messaging contract for inter-agent communication, along with supporting types `ToolCallRecord`, `AgentError`, and `HealthStatus`.

## Group 1 — Add dependencies

_Tasks in this group must be done first._

- [x] **Add `uuid` and `serde_json` dependencies to `agent-sdk/Cargo.toml`** `[S]`
      Add `uuid = { version = "1", features = ["v4", "serde"] }` and `serde_json = "1"` to `[dependencies]` in `crates/agent-sdk/Cargo.toml`. The `uuid` crate is needed for the `Uuid` type in `AgentRequest` and `AgentResponse`. The `serde_json` crate is needed for `serde_json::Value` fields in `AgentRequest`, `AgentResponse`, and `ToolCallRecord`. Note: `serde_json` is already a transitive dependency via `schemars`, but it must be a direct dependency since we use its types in public API surfaces. `serde` and `schemars` already exist from issue #2.
      Files: `crates/agent-sdk/Cargo.toml`
      Blocking: all tasks in Group 2 and Group 3

## Group 2 — Define supporting types

_Depends on: Group 1. Tasks in this group can be done in parallel._

- [x] **Define `ToolCallRecord` struct** `[S]`
      Create `crates/agent-sdk/src/tool_call_record.rs` with a `ToolCallRecord` struct containing: `tool_name: String`, `input: serde_json::Value`, `output: serde_json::Value`. Derive `Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema`. Follow the existing pattern from `model_config.rs` — one struct per file, `use serde::{Deserialize, Serialize}` and `use schemars::JsonSchema` at the top. No methods or impl blocks; this is a pure data carrier that records a single tool invocation within an agent's turn.
      Files: `crates/agent-sdk/src/tool_call_record.rs`
      Blocking: "Define `AgentResponse` struct"

- [x] **Define `AgentError` enum** `[M]`
      Create `crates/agent-sdk/src/agent_error.rs` with an `AgentError` enum with variants: `ToolCallFailed { tool: String, reason: String }`, `ConfidenceTooLow { confidence: f32, threshold: f32 }`, `MaxTurnsExceeded { turns: u32 }`, `Internal(String)`. Derive `Debug, Clone, PartialEq, Serialize, Deserialize`. Implement `std::fmt::Display` with meaningful messages for each variant (e.g., `"Tool call '{}' failed: {}"` for `ToolCallFailed`). Implement `std::error::Error` (the blanket impl from `Display` suffices). Do NOT derive `JsonSchema` since error types typically are not part of schema generation.
      Files: `crates/agent-sdk/src/agent_error.rs`
      Blocking: "Update `lib.rs` module declarations and re-exports"

- [x] **Define `HealthStatus` enum** `[S]`
      Create `crates/agent-sdk/src/health_status.rs` with a `HealthStatus` enum: `Healthy`, `Degraded(String)`, `Unhealthy(String)`. Derive `Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema`. No methods needed. The `String` payloads carry human-readable reasons for non-healthy states. This type is used by `MicroAgent::health()` in issue #3.
      Files: `crates/agent-sdk/src/health_status.rs`
      Blocking: "Update `lib.rs` module declarations and re-exports"

## Group 3 — Define envelope types

_Depends on: Group 2 (specifically `ToolCallRecord` for `AgentResponse`)._

- [x] **Define `AgentRequest` struct** `[S]`
      Create `crates/agent-sdk/src/agent_request.rs` with an `AgentRequest` struct containing: `id: uuid::Uuid`, `input: String`, `context: Option<serde_json::Value>`, `caller: Option<String>`. Derive `Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema`. Add a convenience constructor `AgentRequest::new(input: String) -> Self` that auto-generates a v4 UUID and sets `context` and `caller` to `None`.
      Files: `crates/agent-sdk/src/agent_request.rs`
      Blocking: "Update `lib.rs` module declarations and re-exports"

- [x] **Define `AgentResponse` struct** `[S]`
      Create `crates/agent-sdk/src/agent_response.rs` with an `AgentResponse` struct containing: `id: uuid::Uuid`, `output: serde_json::Value`, `confidence: f32`, `escalated: bool`, `tool_calls: Vec<ToolCallRecord>`. Derive `Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema`. Add a convenience constructor `AgentResponse::success(id: uuid::Uuid, output: serde_json::Value) -> Self` that sets `confidence: 1.0`, `escalated: false`, and `tool_calls: vec![]`. Import `ToolCallRecord` from `crate::tool_call_record`.
      Files: `crates/agent-sdk/src/agent_response.rs`
      Blocked by: "Define `ToolCallRecord` struct"
      Blocking: "Update `lib.rs` module declarations and re-exports"

## Group 4 — Wire up module tree

_Depends on: Group 3._

- [x] **Update `lib.rs` module declarations and re-exports** `[S]`
      Add `mod` declarations and `pub use` re-exports to `crates/agent-sdk/src/lib.rs` for all five new modules: `agent_request`, `agent_response`, `agent_error`, `health_status`, `tool_call_record`. The existing four modules (`constraints`, `model_config`, `output_schema`, `skill_manifest`) and their re-exports remain unchanged. After this task, consumers can write `use agent_sdk::{AgentRequest, AgentResponse, AgentError, HealthStatus, ToolCallRecord}`.
      Files: `crates/agent-sdk/src/lib.rs`
      Blocked by: all Group 2 and Group 3 tasks
      Blocking: "Write serialization and construction tests"

## Group 5 — Tests and verification

_Depends on: Group 4._

- [x] **Write serialization and construction tests** `[M]`
      Create `crates/agent-sdk/tests/envelope_types_test.rs` with tests that cover: (1) construct an `AgentRequest` via `AgentRequest::new()` and verify the UUID is non-nil, `context` is `None`, `caller` is `None`; (2) JSON round-trip serialize/deserialize an `AgentRequest` with all fields populated (including `context` and `caller`); (3) construct an `AgentResponse` via `AgentResponse::success()` and verify defaults (`confidence == 1.0`, `escalated == false`, `tool_calls.is_empty()`); (4) JSON round-trip serialize/deserialize an `AgentResponse` with `tool_calls` populated; (5) verify each `AgentError` variant's `Display` output contains expected substrings; (6) serialize/deserialize each `HealthStatus` variant; (7) serialize/deserialize `ToolCallRecord` with nested JSON values.
      Files: `crates/agent-sdk/tests/envelope_types_test.rs`
      Blocked by: "Update `lib.rs` module declarations and re-exports"
      Blocking: None

- [x] **Run `cargo check`, `cargo clippy`, `cargo test` to verify** `[S]`
      Run the full verification suite per CLAUDE.md project commands. Ensure no clippy warnings, all tests pass (including the existing `skill_manifest_test.rs` tests from issue #2), and the crate compiles cleanly.
      Files: (none — command-line only)
      Blocked by: "Write serialization and construction tests"
      Blocking: None
