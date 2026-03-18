# Task Breakdown: Implement constraint enforcement

> Enforce the four constraint fields declared in skill files (`max_turns`, `confidence_threshold`, `escalate_to`, `allowed_actions`) at runtime during agent invocation, using rig-core's native `default_max_turns` and structural tool filtering where possible, and post-invocation checks for confidence/escalation.

## Group 1 — SDK type changes

_Tasks in this group can be done in parallel._

- [x] **Add `escalate_to` field to `AgentResponse`** `[S]`
      Add `escalate_to: Option<String>` to the `AgentResponse` struct in `crates/agent-sdk/src/agent_response.rs`. It must be `#[serde(default, skip_serializing_if = "Option::is_none")]` to avoid breaking existing JSON consumers. Update `AgentResponse::success()` to initialize it as `None`. Update all test files that construct `AgentResponse` directly (in `crates/agent-sdk/tests/micro_agent_test.rs`, `crates/agent-runtime/tests/http_test.rs`) to include the new field.
      Files: `crates/agent-sdk/src/agent_response.rs`, `crates/agent-sdk/tests/micro_agent_test.rs`, `crates/agent-runtime/tests/http_test.rs`
      Blocking: "Implement ConstraintEnforcer struct with confidence and escalation checks"

- [x] **Add `ActionDisallowed` variant to `AgentError`** `[S]`
      Add a new `AgentError::ActionDisallowed { action: String, allowed: Vec<String> }` variant to `crates/agent-sdk/src/agent_error.rs`. Implement `Display` for it with a message like `"Action 'write' is not in allowed actions: [read, query]"`. Add a corresponding HTTP status mapping in `crates/agent-runtime/src/http.rs` — map it to `403 Forbidden` in the `AppError::into_response()` impl.
      Files: `crates/agent-sdk/src/agent_error.rs`, `crates/agent-runtime/src/http.rs`
      Blocking: "Filter tools by `allowed_actions` in tool resolution"

## Group 2 — Structural enforcement (build-time)

_Depends on: Group 1._

- [x] **Set `default_max_turns` from constraints at agent build time** `[S]`
      Modify `crates/agent-runtime/src/tool_bridge.rs` in `build_agent_with_tools()` to accept `&Constraints` as a parameter (or just `max_turns: u32`) and call `.default_max_turns(constraints.max_turns as usize)` on the builder before calling `.tools(boxed).build()`. Update the call site in `crates/agent-runtime/src/provider.rs` (`build_openai_agent` and `build_anthropic_agent`) to pass the manifest's constraints. This delegates turn enforcement to rig-core's native `PromptRequest` loop, which returns `PromptError::MaxTurnsError` when the limit is hit.
      Files: `crates/agent-runtime/src/tool_bridge.rs`, `crates/agent-runtime/src/provider.rs`
      Blocked by: none (no dependency on Group 1, but grouped here for sequencing clarity)
      Blocking: "Map rig-core MaxTurnsError to AgentError::MaxTurnsExceeded"

- [x] **Filter tools by `allowed_actions` in tool resolution** `[M]`
      Add an `action_type: Option<String>` field to `ToolEntry` in `crates/tool-registry/src/tool_entry.rs` (with `#[serde(default, skip_serializing_if = "Option::is_none")]`). In `crates/agent-runtime/src/tool_bridge.rs`, modify `resolve_mcp_tools()` to accept `&[String]` for allowed_actions and filter `entries` by `action_type` — if `allowed_actions` is non-empty, exclude tools whose `action_type` is `Some(t)` where `t` is not in the allowed list. If a tool's `action_type` is `None`, include it (no restriction). This is structural enforcement: disallowed tools are never given to the LLM.
      Files: `crates/tool-registry/src/tool_entry.rs`, `crates/agent-runtime/src/tool_bridge.rs`, `crates/agent-runtime/src/main.rs`
      Blocked by: "Add `ActionDisallowed` variant to `AgentError`"
      Blocking: "Write tests for allowed_actions filtering"

## Group 3 — Runtime enforcement (post-invocation)

_Depends on: Group 1._

- [x] **Implement ConstraintEnforcer struct with confidence and escalation checks** `[M]`
      Create `crates/agent-runtime/src/constraint_enforcer.rs`. Define a `ConstraintEnforcer` struct wrapping an `Arc<dyn MicroAgent>` and implementing `MicroAgent` itself. The enforcer:
      1. Delegates `manifest()` and `health()` to the inner agent.
      2. In `invoke()`, calls `self.inner.invoke(request).await`, then performs a post-invocation confidence check: if `(response.confidence as f64) < manifest.constraints.confidence_threshold`, set `response.escalated = true` and `response.escalate_to = manifest.constraints.escalate_to.clone()`.
      3. Low confidence is a successful response with escalation metadata, not an error.
      Register the module in `crates/agent-runtime/src/lib.rs` with `pub mod constraint_enforcer;`.
      Files: `crates/agent-runtime/src/constraint_enforcer.rs`, `crates/agent-runtime/src/lib.rs`
      Blocked by: "Add `escalate_to` field to `AgentResponse`"
      Blocking: "Wire ConstraintEnforcer into main.rs"

- [x] **Map rig-core MaxTurnsError to AgentError::MaxTurnsExceeded** `[S]`
      In `crates/agent-runtime/src/runtime_agent.rs`, update the `invoke()` method to inspect the error from `self.agent.prompt()`. Currently all errors are mapped to `AgentError::Internal(e.to_string())`. Change this to detect `MaxTurnsError` and map it to `AgentError::MaxTurnsExceeded { turns: manifest.constraints.max_turns }`. Since `BuiltAgent::prompt()` returns `ProviderError::Prompt(String)` which loses the type, check for the "MaxTurnError" substring as a pragmatic fallback, or improve `ProviderError` to carry a typed enum.
      Files: `crates/agent-runtime/src/runtime_agent.rs`, `crates/agent-runtime/src/provider.rs`
      Blocked by: "Set `default_max_turns` from constraints at agent build time"
      Blocking: "Wire ConstraintEnforcer into main.rs"

## Group 4 — Integration

_Depends on: Groups 2 and 3._

- [ ] **Wire ConstraintEnforcer into main.rs** `[S]`
      Modify `crates/agent-runtime/src/main.rs` to wrap the `RuntimeAgent` with `ConstraintEnforcer` before passing it to the HTTP layer. Between step 6 ("Creating runtime agent") and step 7 ("Starting HTTP server"), wrap: `let enforced = ConstraintEnforcer::new(Arc::new(runtime_agent));` then `let micro_agent: Arc<dyn MicroAgent> = Arc::new(enforced);`. Add a log line like `tracing::info!("[6.5/7] Applying constraint enforcement")`.
      Files: `crates/agent-runtime/src/main.rs`
      Blocked by: "Implement ConstraintEnforcer struct with confidence and escalation checks", "Map rig-core MaxTurnsError to AgentError::MaxTurnsExceeded"
      Blocking: "Write tests for ConstraintEnforcer"

## Group 5 — Tests and verification

_Depends on: Group 4._

- [ ] **Write tests for ConstraintEnforcer** `[M]`
      Create `crates/agent-runtime/tests/constraint_enforcer_test.rs`. Use a `MockAgent` pattern (follow `crates/agent-sdk/tests/micro_agent_test.rs`). Tests:
      1. Confidence above threshold: response passes through unchanged, `escalated: false`, `escalate_to: None`.
      2. Confidence below threshold triggers escalation: response has `escalated: true`, `escalate_to: Some("fallback-agent")`.
      3. Confidence below threshold without escalate_to: response has `escalated: true`, `escalate_to: None`.
      4. Manifest and health delegate correctly.
      5. Error propagation: inner agent error passes through unchanged.
      Files: `crates/agent-runtime/tests/constraint_enforcer_test.rs`
      Blocked by: "Wire ConstraintEnforcer into main.rs"
      Blocking: "Run verification suite"

- [ ] **Write tests for allowed_actions filtering** `[M]`
      Add tests verifying: when `resolve_mcp_tools()` is called with `allowed_actions: ["read", "query"]`, tools with `action_type: Some("write")` are excluded, tools with `action_type: Some("read")` are included, and tools with `action_type: None` are included. Also test the empty `allowed_actions` case (all tools pass through).
      Files: `crates/tool-registry/tests/tool_registry_test.rs` or `crates/agent-runtime/tests/tool_bridge_test.rs`
      Blocked by: "Filter tools by `allowed_actions` in tool resolution"
      Blocking: "Run verification suite"

- [ ] **Write tests for max_turns enforcement** `[S]`
      Add a test verifying that when the agent returns a `MaxTurnsError`-like error, the `RuntimeAgent` maps it to `AgentError::MaxTurnsExceeded`. Also verify that `default_max_turns` is correctly passed through `build_agent_with_tools` by checking the built agent's field.
      Files: `crates/agent-runtime/tests/runtime_agent_test.rs`
      Blocked by: "Map rig-core MaxTurnsError to AgentError::MaxTurnsExceeded"
      Blocking: "Run verification suite"

- [ ] **Run verification suite** `[S]`
      Run `cargo check`, `cargo clippy`, and `cargo test` across the full workspace to verify everything compiles, has no warnings, and all tests pass. Verify no regressions in existing tests.
      Files: (none — command-line only)
      Blocked by: "Write tests for ConstraintEnforcer", "Write tests for allowed_actions filtering", "Write tests for max_turns enforcement"

## Implementation Notes

1. **rig-core 0.32 has native turn enforcement**: `AgentBuilder` supports `.default_max_turns(usize)` which propagates to `Agent.default_max_turns` and is used by `PromptRequest` to limit the tool-calling loop.

2. **Structural tool filtering over hooks**: rig-core's `PromptHook` could enforce `allowed_actions` at the tool-call level, but it changes the generic type `P` on `Agent<M, P>` which would require updating `BuiltAgent` enum variants. Filtering tools at resolve time is simpler.

3. **`BuiltAgent::prompt()` loses error type information**: `ProviderError::Prompt(String)` erases the `PromptError` enum. Consider improving this to preserve `MaxTurnsError` distinction.

4. **Confidence is self-reported by the LLM**: The enforcement layer checks `response.confidence` against `constraints.confidence_threshold`. The enforcer just checks the number; it does not generate it.

5. **Type mismatch**: `Constraints.confidence_threshold` is `f64`, `AgentResponse.confidence` is `f32`. Cast `response.confidence as f64` before comparing.

6. **Builder method ordering matters**: On `AgentBuilder`, `.hook()` is only available in the `NoToolConfig` state. `.default_max_turns()` is available on any `ToolState`. Chain must be `.default_max_turns(n).tools(t).build()`.
