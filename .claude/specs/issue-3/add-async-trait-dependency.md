# Spec: Add `async-trait` dependency to `agent-sdk/Cargo.toml`

> From: .claude/tasks/issue-3.md

## Objective

Add the `async-trait` crate as a dependency and `tokio` as a dev-dependency to the `agent-sdk` crate. This is required because native async traits in Rust (even as of edition 2024) are not dyn-compatible -- meaning you cannot write `Box<dyn MicroAgent>` with bare `async fn` trait methods. The `async-trait` proc macro desugars async methods into `Pin<Box<dyn Future>>` return types, restoring object safety. The orchestrator requires `Box<dyn MicroAgent>` to work with agents generically, making this a hard prerequisite.

The `tokio` dev-dependency provides the `#[tokio::test]` attribute macro needed to write async integration tests for the `MicroAgent` trait.

## Current State

`crates/agent-sdk/Cargo.toml` currently declares:

- **edition**: `2024`
- **[dependencies]**: `serde` (1, with `derive` feature), `schemars` (0.8, with `derive` feature)
- **[dev-dependencies]**: `serde_yaml` (0.9)

There are no async-related dependencies. The crate currently defines only synchronous types (`SkillManifest`, config structs).

## Requirements

- Add `async-trait = "0.1"` to the `[dependencies]` section of `crates/agent-sdk/Cargo.toml`.
- Add `tokio = { version = "1", features = ["macros", "rt"] }` to the `[dev-dependencies]` section of `crates/agent-sdk/Cargo.toml`.
- No other dependencies should be added or modified in this task.
- The crate must continue to compile cleanly after the change (`cargo check -p agent-sdk` succeeds).
- No source code changes are included in this task -- only `Cargo.toml` is modified.

## Implementation Details

**File to modify:** `crates/agent-sdk/Cargo.toml`

Changes:

1. In the `[dependencies]` section, append the line:
   ```toml
   async-trait = "0.1"
   ```

2. In the `[dev-dependencies]` section, append the line:
   ```toml
   tokio = { version = "1", features = ["macros", "rt"] }
   ```

The resulting file should look like:

```toml
[package]
name = "agent-sdk"
version = "0.1.0"
edition = "2024"

[dependencies]
serde = { version = "1", features = ["derive"] }
schemars = { version = "0.8", features = ["derive"] }
async-trait = "0.1"

[dev-dependencies]
serde_yaml = "0.9"
tokio = { version = "1", features = ["macros", "rt"] }
```

No new types, functions, or interfaces are introduced. This is a dependency-only change that unblocks subsequent tasks.

## Dependencies

- **Blocked by**: Nothing -- this is a Group 1 task with no prerequisites.
- **Blocking**:
  - "Define `MicroAgent` trait" (Group 3) -- the trait definition requires `#[async_trait]` from this crate.
  - "Write object-safety and mock-implementation tests" (Group 4) -- async tests require `#[tokio::test]`.

## Risks & Edge Cases

- **Version compatibility**: `async-trait 0.1` is the only major release line and is widely used; it is compatible with all modern Rust editions including 2024. No compatibility risk.
- **Future deprecation**: Once Rust stabilizes dyn-compatible native async traits, `async-trait` can be removed. This is a known future cleanup task, not a current risk.
- **Tokio feature minimality**: Only `macros` and `rt` features are requested for `tokio` (dev-dependency only). This is the minimal set needed for `#[tokio::test]` with the single-threaded runtime. If multi-threaded test execution is needed later, the `rt-multi-thread` feature can be added at that time.
- **No code uses these crates yet**: Adding unused dependencies will produce no warnings because `async-trait` is a proc macro (only activated via `#[async_trait]`) and `tokio` is a dev-dependency (only relevant in tests). There is no dead-code risk.

## Verification

1. Run `cargo check -p agent-sdk` -- must succeed with no errors.
2. Run `cargo build -p agent-sdk` -- must compile cleanly.
3. Run `cargo test -p agent-sdk` -- existing tests must continue to pass.
4. Confirm `async-trait` appears in `cargo tree -p agent-sdk` output under direct dependencies.
5. Confirm `tokio` appears in `cargo tree -p agent-sdk --edges dev` output under dev-dependencies.
