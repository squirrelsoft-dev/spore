# Spec: Wire ConstraintEnforcer into main.rs

> From: .claude/tasks/issue-13.md

## Objective

Insert a `ConstraintEnforcer` decorator between the `RuntimeAgent` and the HTTP layer in the agent-runtime startup sequence. This is the integration step that activates post-invocation constraint enforcement (confidence threshold checks and escalation metadata) for every request flowing through the HTTP API. Without this wiring, the `ConstraintEnforcer` struct (built in a prior task) would exist but never be used.

## Current State

`crates/agent-runtime/src/main.rs` defines a 7-step async `main()` function:

1. Load configuration (`RuntimeConfig::from_env()`)
2. Register tool endpoints into a `ToolRegistry`
3. Connect all tool servers
4. Load skill manifest via `SkillLoader`
5. Build a provider-backed agent
6. Create runtime agent -- constructs a `RuntimeAgent` and immediately wraps it as `Arc<dyn MicroAgent>`
7. Start HTTP server -- passes `micro_agent` and `config.bind_addr` to `http::start_server()`

At step 6 (lines 66-68), the code is:
```rust
// Step 6: Wrap as MicroAgent
tracing::info!("[6/7] Creating runtime agent");
let runtime_agent = RuntimeAgent::new(manifest, agent, registry.clone());
let micro_agent: Arc<dyn MicroAgent> = Arc::new(runtime_agent);
```

The `runtime_agent` is wrapped directly in an `Arc` and assigned to `micro_agent`, which is then passed to the HTTP server. There is no intermediate enforcement layer.

`crates/agent-runtime/src/lib.rs` declares five public modules: `config`, `http`, `provider`, `runtime_agent`, and `tool_bridge`. The `constraint_enforcer` module does not exist yet -- it will be added by the "Implement ConstraintEnforcer struct" task.

The `ConstraintEnforcer` struct (to be created in `crates/agent-runtime/src/constraint_enforcer.rs`) will implement `MicroAgent` and wrap an `Arc<dyn MicroAgent>`. Its constructor is expected to be `ConstraintEnforcer::new(inner: Arc<dyn MicroAgent>)`.

`http::start_server` accepts `AppState` (which is `Arc<dyn MicroAgent>`) and a `SocketAddr`. Since `ConstraintEnforcer` implements `MicroAgent`, wrapping it in `Arc` produces a valid `AppState`.

## Requirements

- The `RuntimeAgent` must be wrapped in an `Arc` and passed to `ConstraintEnforcer::new()` before being assigned to `micro_agent`.
- The final `micro_agent: Arc<dyn MicroAgent>` must hold an `Arc<ConstraintEnforcer>`, not an `Arc<RuntimeAgent>`.
- A new log line `tracing::info!("[6.5/7] Applying constraint enforcement")` must appear between the step 6 log and the step 7 log, confirming the enforcer is applied.
- A `use` import for `agent_runtime::constraint_enforcer::ConstraintEnforcer` (or equivalent path) must be added to `main.rs`.
- The existing step numbering (1-7) must not change. The new step is labeled `[6.5/7]` to indicate it is a sub-step inserted between steps 6 and 7.
- The `http::start_server` call must continue to receive the same type (`Arc<dyn MicroAgent>`) -- the change is purely in what concrete type is behind the trait object.
- No new dependencies are added to `Cargo.toml`.

## Implementation Details

- **File to modify**: `crates/agent-runtime/src/main.rs`

  1. **Add import**: Add `use agent_runtime::constraint_enforcer::ConstraintEnforcer;` to the import block at the top of the file, alongside the existing `use agent_runtime::runtime_agent::RuntimeAgent;`.

  2. **Modify step 6 block**: Replace the current lines 66-68:
     ```rust
     // Step 6: Wrap as MicroAgent
     tracing::info!("[6/7] Creating runtime agent");
     let runtime_agent = RuntimeAgent::new(manifest, agent, registry.clone());
     let micro_agent: Arc<dyn MicroAgent> = Arc::new(runtime_agent);
     ```
     with:
     ```rust
     // Step 6: Wrap as MicroAgent
     tracing::info!("[6/7] Creating runtime agent");
     let runtime_agent = RuntimeAgent::new(manifest, agent, registry.clone());

     // Step 6.5: Apply constraint enforcement
     tracing::info!("[6.5/7] Applying constraint enforcement");
     let enforced = ConstraintEnforcer::new(Arc::new(runtime_agent));
     let micro_agent: Arc<dyn MicroAgent> = Arc::new(enforced);
     ```

  3. **No changes to step 7**: The `http::start_server(micro_agent, config.bind_addr)` call remains identical since the `micro_agent` variable retains the same type (`Arc<dyn MicroAgent>`).

- **No other files are created or modified** in this task. The `constraint_enforcer` module registration in `lib.rs` and the module itself are handled by the "Implement ConstraintEnforcer struct" task.

## Dependencies

- Blocked by: "Implement ConstraintEnforcer struct with confidence and escalation checks" (provides `ConstraintEnforcer` and registers the `constraint_enforcer` module in `lib.rs`)
- Blocked by: "Map rig-core MaxTurnsError to AgentError::MaxTurnsExceeded" (ensures the error type that `ConstraintEnforcer` may encounter from the inner agent is properly typed)
- Blocking: "Write tests for ConstraintEnforcer" (integration tests need the full wired-up server path to verify end-to-end enforcement)

## Risks & Edge Cases

1. **`ConstraintEnforcer::new` signature may differ**: The exact constructor signature depends on the "Implement ConstraintEnforcer struct" task. If the constructor takes additional parameters (e.g., explicit `Constraints` or `SkillManifest`), the wiring code must be adjusted. However, the task description specifies the enforcer reads constraints from the inner agent's `manifest()`, so `new(Arc<dyn MicroAgent>)` is the expected signature.

2. **Double `Arc` wrapping**: The `RuntimeAgent` is first wrapped in `Arc::new(runtime_agent)` to pass to `ConstraintEnforcer::new`, then the enforcer is wrapped in another `Arc::new(enforced)`. This double-Arc is intentional and necessary: `ConstraintEnforcer` holds an `Arc<dyn MicroAgent>` internally (not a bare value), and the HTTP layer requires `Arc<dyn MicroAgent>` for shared ownership. The overhead of two `Arc` reference counts is negligible.

3. **Trait object compatibility**: `ConstraintEnforcer` must implement `MicroAgent + Send + Sync` to be stored as `Arc<dyn MicroAgent>`. This is guaranteed by the "Implement ConstraintEnforcer struct" task, which uses `#[async_trait]` (adding `Send` bounds) and holds only `Send + Sync` fields.

4. **No behavioral change when constraints are permissive**: If `confidence_threshold` is `0.0` and `escalate_to` is `None`, the enforcer is a pass-through. The wiring is unconditional -- there is no feature flag or config option to bypass it. This is the intended design: the enforcer is always in the chain, and permissive constraints simply result in no-op checks.

5. **Module not yet registered**: If this task is attempted before `pub mod constraint_enforcer;` is added to `lib.rs`, the `use agent_runtime::constraint_enforcer::ConstraintEnforcer;` import will fail to compile. The dependency chain prevents this in practice.

## Verification

1. `cargo check -p agent-runtime` compiles without errors (assumes the `constraint_enforcer` module from the blocking task is present).
2. `cargo clippy -p agent-runtime` produces no warnings -- in particular, no unused import warnings for `ConstraintEnforcer`.
3. `cargo test` across the workspace passes with no regressions in existing tests.
4. The startup log output shows the new `[6.5/7] Applying constraint enforcement` line between `[6/7] Creating runtime agent` and `[7/7] Starting HTTP server`.
5. A manual integration test: start the runtime with a skill whose `confidence_threshold` is set high (e.g., `0.99`), send a request via `curl`, and verify the response includes `escalated: true` when the LLM returns a confidence below the threshold.
