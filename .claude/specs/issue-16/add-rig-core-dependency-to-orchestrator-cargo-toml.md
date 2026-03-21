# Spec: Add `rig-core` dependency to orchestrator `Cargo.toml`

> From: .claude/tasks/issue-16.md

## Objective

Add `rig-core`, `tracing`, and `tokio` (dev) dependencies to the orchestrator crate so that the upcoming `SemanticRouter` implementation can use rig-core's embedding types (`EmbeddingModel` trait, `Embedding` struct, `VectorDistance` trait, `EmbeddingError`) and structured logging via `tracing`. None of these crates are new to the workspace dependency tree -- they are already pulled in by `agent-runtime` and present in the lockfile.

## Current State

`crates/orchestrator/Cargo.toml` currently has the following dependencies:

**[dependencies]:**
- `agent-sdk` (path dep)
- `async-trait = "0.1"`
- `futures = "0.3"`
- `reqwest = { version = "0.13", features = ["json"] }`
- `serde = { version = "1", features = ["derive"] }`
- `serde_json = "1"`
- `serde_yaml = "0.9"`

**[dev-dependencies]:**
- `tokio = { version = "1", features = ["macros", "rt", "net"] }`
- `axum = "0.8"`
- `uuid = { version = "1", features = ["v4"] }`

The `agent-runtime` crate already depends on `rig-core = { version = "0.32", features = ["rmcp"] }` and `tracing = "0.1"`. The workspace `Cargo.toml` does not use `[workspace.dependencies]` -- each crate specifies its own dependency versions directly.

## Requirements

1. Add `rig-core = { version = "0.32" }` to `[dependencies]` in `crates/orchestrator/Cargo.toml`. No extra features are needed (the orchestrator does not need the `rmcp` feature that `agent-runtime` uses).
2. Add `tracing = "0.1"` to `[dependencies]` in `crates/orchestrator/Cargo.toml`.
3. The existing `[dev-dependencies]` entry for `tokio` already has `features = ["macros", "rt", "net"]` which is a superset of the requirement (`["macros", "rt"]`). No change is needed for tokio.
4. No new crates should appear in `Cargo.lock` -- verify that `cargo check -p orchestrator` resolves without downloading new crate sources.
5. The crate must continue to compile cleanly: `cargo check -p orchestrator` must succeed.
6. `cargo clippy -p orchestrator` must produce no new warnings.

## Implementation Details

**File to modify:** `crates/orchestrator/Cargo.toml`

**Changes:**

Add two lines to the `[dependencies]` section:

```toml
rig-core = { version = "0.32" }
tracing = "0.1"
```

Place them in a logical order within the existing dependency list. A reasonable placement is alphabetical, which would put `rig-core` after `reqwest` and `tracing` after `serde_yaml`.

No changes to `[dev-dependencies]` are needed since tokio already has the required features.

No Rust source files are modified in this task -- this is purely a manifest change.

## Dependencies

- **Blocked by:** Nothing (Group 1 task, can be done in parallel with other Group 1 tasks)
- **Blocking:** "Implement `SemanticRouter` struct with two-phase routing" (Group 2) -- the SemanticRouter source code will import types from `rig_core` and `tracing`

## Risks & Edge Cases

- **Version mismatch:** If a different version of `rig-core` were specified, it could pull in a second copy of the crate and bloat the binary. Mitigation: use the exact same major.minor version (`0.32`) as `agent-runtime`.
- **Feature conflicts:** The orchestrator does not enable the `rmcp` feature. This is intentional -- `rmcp` is only needed by `agent-runtime` for MCP transport. Cargo will unify features at the workspace level, so both crates share the same compiled `rig-core` artifact.
- **Lockfile churn:** Since these crates are already in the lockfile, `Cargo.lock` should only gain new dependency edges (orchestrator -> rig-core, orchestrator -> tracing) without adding new crate versions. If the lockfile diff shows new crate downloads, investigate before proceeding.

## Verification

1. Run `cargo check -p orchestrator` -- must succeed with no errors.
2. Run `cargo clippy -p orchestrator` -- must produce no new warnings.
3. Run `cargo test -p orchestrator` -- all existing tests must continue to pass.
4. Inspect the `Cargo.lock` diff and confirm no new crate versions were added (only new dependency edges for the orchestrator package).
5. Confirm `rig-core` types are importable by temporarily adding `use rig_core::embeddings::EmbeddingModel;` to a test or source file (optional manual check, not required to be committed).
