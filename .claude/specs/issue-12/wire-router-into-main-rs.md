# Spec: Wire router into main.rs

> From: .claude/tasks/issue-12.md

## Objective

Integrate the HTTP server into the agent-runtime startup sequence so that after the `MicroAgent` is constructed, the runtime binds to a TCP address and begins serving `POST /invoke` and `GET /health` requests. This is the final wiring step that turns the agent-runtime from a build-and-exit binary into a long-running HTTP service.

## Current State

`crates/agent-runtime/src/main.rs` defines an async `main()` function with a 6-step startup sequence:

1. Load configuration (`RuntimeConfig::from_env()`)
2. Register tool endpoints into a `ToolRegistry`
3. Connect all tool servers
4. Load skill manifest via `SkillLoader`
5. Build a provider-backed agent
6. Wrap the agent as `Arc<dyn MicroAgent>`

At step 6 (line 67), the agent is assigned to `_micro_agent` with an underscore prefix, meaning it is intentionally unused. After construction, the function logs "Agent startup complete" and returns `Ok(())`, so the binary exits immediately.

`crates/agent-runtime/src/config.rs` already parses a `bind_addr: SocketAddr` field from the `BIND_ADDR` environment variable (defaulting to `0.0.0.0:8080`). This value is logged at step 1 but never consumed.

The `main()` return type is `Result<(), Box<dyn std::error::Error>>`, which can propagate `std::io::Error` from the TCP listener without any conversion.

The `http` module (containing `AppState`, `build_router`, and `start_server`) does not exist yet -- it will be created by the "Create HTTP handler module" task, which this task is blocked by.

## Requirements

- The `_micro_agent` variable on line 67 must be renamed to `micro_agent` (remove underscore prefix) so it is actively used rather than silently dropped.
- The `micro_agent` value must be wrapped as the `AppState` type defined in the `http` module (expected to be a newtype or alias around `Arc<dyn MicroAgent>`).
- The HTTP server must be started by calling `agent_runtime::http::start_server(state, config.bind_addr).await?`, which binds a `TcpListener` and serves the axum router.
- The step numbering must change from a 6-step sequence to a 7-step sequence: all existing `[N/6]` labels become `[N/7]`, and a new `[7/7] Starting HTTP server` step is added.
- A `tracing::info!` log line must emit the bind address before the server starts, so operators can see which address the runtime is listening on.
- The "Agent startup complete" log line should be removed or moved to before the server starts (since `start_server` blocks until shutdown, any log after it would never execute).
- A `use agent_runtime::http;` import (or equivalent) must be added to `main.rs`.
- `config.bind_addr` must not be consumed before step 7 (it is currently only used in the step-1 log via `%config.bind_addr`, which borrows it -- since `SocketAddr` is `Copy`, this is not an issue).

## Implementation Details

- **File to modify**: `crates/agent-runtime/src/main.rs`

  1. **Add import**: Add `use agent_runtime::http;` (or `use agent_runtime::http::AppState;` plus `use agent_runtime::http::start_server;` if preferred) to the import block at the top of the file.

  2. **Update step labels**: Change all `[N/6]` log strings to `[N/7]`:
     - Line 32: `"[1/6] Loading configuration"` becomes `"[1/7] Loading configuration"`
     - Line 42: `"[2/6] Registering tool entries"` becomes `"[2/7] Registering tool entries"`
     - Line 47: `"[3/6] Connecting to tool servers"` becomes `"[3/7] Connecting to tool servers"`
     - Line 51: `"[4/6] Loading skill manifest"` becomes `"[4/7] Loading skill manifest"`
     - Line 61: `"[5/6] Building agent"` becomes `"[5/7] Building agent"`
     - Line 65: `"[6/6] Creating runtime agent"` becomes `"[6/7] Creating runtime agent"`

  3. **Rename variable**: Change line 67 from `let _micro_agent: Arc<dyn MicroAgent> = Arc::new(runtime_agent);` to `let micro_agent: Arc<dyn MicroAgent> = Arc::new(runtime_agent);`.

  4. **Wrap as AppState**: Add `let state = http::AppState::new(micro_agent);` (or however AppState is constructed -- this depends on the "Create HTTP handler module" task's design, but the task description says it wraps `Arc<dyn MicroAgent>`).

  5. **Add step 7 with logging**: Replace the current "Agent startup complete" log and `Ok(())` with:
     ```rust
     // Step 7: Start HTTP server
     tracing::info!(bind_addr = %config.bind_addr, "[7/7] Starting HTTP server");
     http::start_server(state, config.bind_addr).await?;
     Ok(())
     ```
     Note: The `Ok(())` after `start_server` is technically unreachable during normal operation (the server runs until shutdown), but it satisfies the return type and will execute if the server shuts down gracefully.

- **No other files are created or modified** in this task. The `http` module registration in `lib.rs` and the module itself are handled by the "Create HTTP handler module" task.

## Dependencies

- Blocked by: "Create HTTP handler module" (provides the `http` module with `AppState`, `start_server`)
- Blocking: "Write handler integration tests" (tests need the full wired-up server path)

## Risks & Edge Cases

- **`AppState` constructor may differ**: The exact API for constructing `AppState` depends on how the "Create HTTP handler module" task defines it. If `AppState` is a type alias for `Arc<dyn MicroAgent>`, no wrapping call is needed -- just pass `micro_agent` directly. If it is a newtype struct, a constructor like `AppState::new(micro_agent)` or `AppState(micro_agent)` is needed. The implementer should read the `http` module's definition at implementation time.
- **`config.bind_addr` moved before step 7**: `config.skill_dir` is moved into the `SkillLoader` at step 4 (line 53), but `config.bind_addr` is `SocketAddr` which implements `Copy`, so it remains usable at step 7 regardless of prior borrows.
- **Server blocks forever**: `start_server` will block the async runtime until the process receives a signal or the listener errors. This is expected behavior. Graceful shutdown is out of scope (tracked in issue #14).
- **Port already in use**: If `BIND_ADDR` points to a port already bound, `TcpListener::bind` will return `std::io::Error` with `AddrInUse`, which propagates cleanly through the `?` operator. The error message from the I/O error is descriptive enough.

## Verification

1. `cargo check -p agent-runtime` compiles without errors or warnings (assumes the `http` module from the blocking task is present).
2. `cargo clippy -p agent-runtime` produces no warnings -- in particular, no `unused_variable` warning for `micro_agent` and no `dead_code` warning.
3. The step log output in a manual run shows `[1/7]` through `[7/7]` in sequence with no gaps or duplicates.
4. The bind address logged at step 7 matches the `BIND_ADDR` environment variable (or the default `0.0.0.0:8080` when unset).
5. `cargo test` across the workspace passes with no regressions.
6. After starting the binary (with appropriate env vars), `curl http://localhost:8080/health` returns a successful JSON response, confirming the server is actually listening.
