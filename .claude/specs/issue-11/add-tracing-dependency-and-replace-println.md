# Spec: Add tracing dependency and replace println with structured logging

> From: .claude/tasks/issue-11.md

## Objective

Replace ad-hoc `println!` progress messages in `agent-runtime`'s `main.rs` with structured logging via the `tracing` crate. This enables runtime log-level control through the `RUST_LOG` environment variable (e.g., `RUST_LOG=info`, `RUST_LOG=agent_runtime=debug`) and establishes a consistent logging pattern already used by `echo-tool`.

## Current State

- `crates/agent-runtime/Cargo.toml` has no `tracing` or `tracing-subscriber` dependency.
- `crates/agent-runtime/src/main.rs` uses seven `println!` calls for progress logging during startup (steps 1/6 through 6/6, plus a final "Agent startup complete" message).
- `tools/echo-tool` already uses `tracing = "0.1"` and `tracing-subscriber = { version = "0.3", features = ["env-filter"] }` with a `tracing_subscriber::fmt()` initialization pattern that writes to stderr with env-filter support. This is the established pattern in the workspace.

## Requirements

1. Add `tracing = "0.1"` and `tracing-subscriber = { version = "0.3", features = ["env-filter"] }` to `crates/agent-runtime/Cargo.toml` under `[dependencies]`.
2. Initialize the tracing subscriber as the first operation in `main()`, before any other work. Use the same pattern as `echo-tool`:
   - `tracing_subscriber::fmt()` with `.with_env_filter(EnvFilter::from_default_env())`.
   - Use a default directive of `info` level (not `debug` as echo-tool does, since agent-runtime is the top-level binary and `info` is a better default for production use).
   - Write to stderr (`.with_writer(std::io::stderr)`), keeping stdout clean for structured output.
   - Disable ANSI color codes (`.with_ansi(false)`) for clean log output in CI/containers.
3. Replace all seven `println!` calls in `main.rs` with `tracing::info!` calls, preserving the existing message content.
4. The `RUST_LOG` environment variable must control log verbosity at runtime (this is automatic when using `EnvFilter`).
5. No other behavioral changes to `main.rs` — the startup flow, error handling, and function signatures remain identical.

## Implementation Details

### Files to modify

**`crates/agent-runtime/Cargo.toml`**
- Add two new lines under `[dependencies]`:
  - `tracing = "0.1"`
  - `tracing-subscriber = { version = "0.3", features = ["env-filter"] }`

**`crates/agent-runtime/src/main.rs`**
- Add import: `use tracing_subscriber::{self, EnvFilter};`
- Insert tracing subscriber initialization as the first statement in `main()`, before the "Creating tool registry" step:
  ```rust
  tracing_subscriber::fmt()
      .with_env_filter(
          EnvFilter::from_default_env()
              .add_directive(tracing::Level::INFO.into()),
      )
      .with_writer(std::io::stderr)
      .with_ansi(false)
      .init();
  ```
- Replace each `println!(...)` with `tracing::info!(...)`. The seven replacements are:
  1. `println!("[1/6] Creating tool registry")` -> `tracing::info!("[1/6] Creating tool registry")`
  2. `println!("[2/6] Registering tool entries")` -> `tracing::info!("[2/6] Registering tool entries")`
  3. `println!("[3/6] Connecting to tool servers")` -> `tracing::info!("[3/6] Connecting to tool servers")`
  4. `println!("[4/6] Loading skill manifest")` -> `tracing::info!("[4/6] Loading skill manifest")`
  5. `println!("[5/6] Resolving MCP tools")` -> `tracing::info!("[5/6] Resolving MCP tools")`
  6. `println!("[6/6] Building agent")` -> `tracing::info!("[6/6] Building agent")`
  7. `println!("Agent startup complete")` -> `tracing::info!("Agent startup complete")`

### Integration points

- The tracing subscriber must be initialized before any `tracing::info!` call, which means it must be the first statement in `main()`.
- Other modules in `agent-runtime` (current and future, such as `provider.rs`) can use `tracing::info!`, `tracing::debug!`, `tracing::error!`, etc., without any additional initialization since the subscriber is global.
- The `tracing` crate dependency added here will also be used by the provider module task (Group 1 sibling task).

## Dependencies

- Blocked by: nothing (Group 1 task, can start immediately)
- Blocking: "Refactor main.rs to use config and RuntimeAgent" (Group 2 task that depends on tracing being available)

## Risks & Edge Cases

- **Double subscriber initialization**: If a dependency (e.g., a test harness or future HTTP framework) also tries to initialize a tracing subscriber, the second `init()` call will panic. Mitigation: use `.try_init()` instead of `.init()` if this becomes an issue, but for now `.init()` matches the echo-tool pattern and is fine for a binary crate's `main()`.
- **Log noise from dependencies**: Without `RUST_LOG` set, the default `INFO` directive will emit info-level logs from all crates (including dependencies like `hyper`, `tokio`, etc.). Mitigation: the default level of `INFO` is reasonable; users can further restrict with `RUST_LOG=agent_runtime=info` to silence dependency logs if needed.
- **No behavioral change risk**: This is a pure logging infrastructure change. The startup flow, error handling, and return values are unchanged. The only observable difference is that log output moves from stdout to stderr and gains timestamps/level prefixes.

## Verification

1. `cargo check -p agent-runtime` compiles without errors.
2. `cargo clippy -p agent-runtime` produces no warnings.
3. `cargo test -p agent-runtime` passes (existing tests, if any, still work).
4. `cargo build -p agent-runtime` succeeds.
5. Running the binary without `RUST_LOG` set produces info-level log lines on stderr with timestamps and level indicators.
6. Running with `RUST_LOG=debug` produces more verbose output; `RUST_LOG=error` suppresses info messages.
7. No `println!` calls remain in `main.rs` (verified by grep).
8. Full workspace check: `cargo check` and `cargo test` pass with no regressions.
