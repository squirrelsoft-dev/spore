# Task Breakdown: Define MicroAgent trait

> Define the `MicroAgent` async trait in `agent-sdk` — the core abstraction every agent implements, enabling the runtime and orchestrator to work with agents via `Box<dyn MicroAgent>`.

## Group 1 — Add dependencies

_Tasks in this group can be done in parallel._

- [ ] **Add `async-trait` dependency to `agent-sdk/Cargo.toml`** `[S]`
      Add `async-trait = "0.1"` to `[dependencies]` in `crates/agent-sdk/Cargo.toml`. This is required because native async traits in Rust 1.94 are not dyn-compatible, and the orchestrator needs `Box<dyn MicroAgent>`. Also add `tokio = { version = "1", features = ["macros", "rt"] }` to `[dev-dependencies]` so async tests can use `#[tokio::test]`.
      Files: `crates/agent-sdk/Cargo.toml`
      Blocking: "Define `MicroAgent` trait", "Write object-safety and mock-implementation tests"

- [ ] **Add `uuid` and `serde_json` dependencies to `agent-sdk/Cargo.toml`** `[S]`
      Add `uuid = { version = "1", features = ["v4", "serde"] }` and `serde_json = "1"` to `[dependencies]`. These are required by the issue #4 types (`AgentRequest.id` is `Uuid`, `AgentResponse.output` and `AgentRequest.context` use `serde_json::Value`).
      Files: `crates/agent-sdk/Cargo.toml`
      Blocking: All issue #4 type definitions in Group 2

## Group 2 — Define prerequisite types (issue #4 envelope types)

_Depends on: Group 1. Tasks in this group can be done in parallel._

_Note: These types are part of issue #4 but are prerequisites for the MicroAgent trait. If issue #4 is worked separately, skip this group and mark the trait as blocked on issue #4. If working both together (as the triage recommends), implement these here._

- [ ] **Define `ToolCallRecord` struct** `[S]`
      Create `crates/agent-sdk/src/tool_call_record.rs` with fields: `tool_name: String`, `input: serde_json::Value`, `output: serde_json::Value`. Derive `Debug`, `Clone`, `Serialize`, `Deserialize`. Follow the existing pattern from `model_config.rs`.
      Files: `crates/agent-sdk/src/tool_call_record.rs`
      Blocking: "Define `AgentResponse` struct"

- [ ] **Define `AgentRequest` struct** `[S]`
      Create `crates/agent-sdk/src/agent_request.rs` with fields: `id: uuid::Uuid`, `input: String`, `context: Option<serde_json::Value>`, `caller: Option<String>`. Derive `Debug`, `Clone`, `Serialize`, `Deserialize`. Add a `new(input: String)` constructor that generates a UUID v4.
      Files: `crates/agent-sdk/src/agent_request.rs`
      Blocking: "Define `MicroAgent` trait"

- [ ] **Define `AgentResponse` struct** `[S]`
      Create `crates/agent-sdk/src/agent_response.rs` with fields: `id: uuid::Uuid`, `output: serde_json::Value`, `confidence: f32`, `escalated: bool`, `tool_calls: Vec<ToolCallRecord>`. Derive `Debug`, `Clone`, `Serialize`, `Deserialize`.
      Files: `crates/agent-sdk/src/agent_response.rs`
      Blocking: "Define `MicroAgent` trait"

- [ ] **Define `AgentError` enum** `[S]`
      Create `crates/agent-sdk/src/agent_error.rs` with variants: `ToolCallFailed { tool: String, reason: String }`, `ConfidenceTooLow { confidence: f32, threshold: f32 }`, `MaxTurnsExceeded { turns: u32 }`, `Internal(String)`. Derive `Debug`, `Clone`, `PartialEq`. Implement `std::fmt::Display` and `std::error::Error`.
      Files: `crates/agent-sdk/src/agent_error.rs`
      Blocking: "Define `MicroAgent` trait"

- [ ] **Define `HealthStatus` enum** `[S]`
      Create `crates/agent-sdk/src/health_status.rs` with variants: `Healthy`, `Degraded(String)`, `Unhealthy(String)`. Derive `Debug`, `Clone`, `PartialEq`, `Serialize`, `Deserialize`.
      Files: `crates/agent-sdk/src/health_status.rs`
      Blocking: "Define `MicroAgent` trait"

## Group 3 — Define MicroAgent trait and update module tree

_Depends on: Group 2_

- [ ] **Define `MicroAgent` trait** `[S]`
      Create `crates/agent-sdk/src/micro_agent.rs`. Define the trait using the `#[async_trait]` macro with three methods: `fn manifest(&self) -> &SkillManifest` (synchronous, returns reference), `async fn invoke(&self, request: AgentRequest) -> Result<AgentResponse, AgentError>`, and `async fn health(&self) -> HealthStatus`. The trait must have `Send + Sync` supertraits for object safety. Import types from sibling modules via `crate::`.
      Files: `crates/agent-sdk/src/micro_agent.rs`
      Blocked by: All Group 2 types
      Blocking: "Update `lib.rs` with new module declarations and re-exports"

- [ ] **Update `lib.rs` with new module declarations and re-exports** `[S]`
      Add `mod` declarations for all new modules (`tool_call_record`, `agent_request`, `agent_response`, `agent_error`, `health_status`, `micro_agent`) and corresponding `pub use` re-exports. Also re-export `async_trait::async_trait` so downstream crates can use `#[async_trait]` when implementing `MicroAgent` without adding `async-trait` as a direct dependency.
      Files: `crates/agent-sdk/src/lib.rs`
      Blocked by: "Define `MicroAgent` trait"
      Blocking: All Group 4 tasks

## Group 4 — Tests and verification

_Depends on: Group 3. Tasks in this group can be done in parallel (except the final verification)._

- [ ] **Write object-safety and mock-implementation tests** `[M]`
      Create `crates/agent-sdk/tests/micro_agent_test.rs`. Write tests that: (1) create a `MockAgent` struct implementing `MicroAgent` to verify the trait compiles and is implementable; (2) verify `Box<dyn MicroAgent>` is valid (object safety) by boxing a `MockAgent` and calling methods through the trait object; (3) test that `invoke` returns both `Ok` and `Err` variants correctly; (4) test all three `HealthStatus` variants. Use `#[tokio::test]` for async tests. Follow the existing test pattern from `skill_manifest_test.rs`.
      Files: `crates/agent-sdk/tests/micro_agent_test.rs`
      Blocked by: lib.rs re-exports
      Blocking: "Run verification suite"

- [ ] **Write serialization tests for envelope types** `[S]`
      Create `crates/agent-sdk/tests/envelope_types_test.rs`. Write round-trip serialization tests for `AgentRequest`, `AgentResponse`, `ToolCallRecord`, and `AgentError`. Test `AgentError`'s `Display` output. Test `AgentRequest::new()` constructor. Follow the round-trip pattern from `skill_manifest_test.rs`.
      Files: `crates/agent-sdk/tests/envelope_types_test.rs`
      Blocked by: lib.rs re-exports
      Blocking: "Run verification suite"

- [ ] **Run `cargo check`, `cargo clippy`, `cargo test` to verify** `[S]`
      Run the full verification suite per CLAUDE.md project commands. Ensure no clippy warnings, all tests pass, and the crate compiles cleanly. Verify that `Box<dyn MicroAgent>` compiles without errors.
      Files: (none — command-line only)
      Blocked by: All tests in Group 4
