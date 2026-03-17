# Spec: Add axum dependency to agent-runtime

> From: .claude/tasks/issue-12.md

## Objective

Add the `axum` web framework as a dependency to the `agent-runtime` crate so that downstream tasks ("Create HTTP handler module", "Wire router into main.rs") can build HTTP endpoints for the agent runtime. This is a dependency-only change with no code modifications.

## Current State

`crates/agent-runtime/Cargo.toml` currently declares these dependencies:

- `rig-core` 0.32 (with `rmcp` feature)
- `rmcp` 0.16 (with `client` and `transport-async-rw` features)
- `tool-registry`, `agent-sdk`, `skill-loader` (workspace path deps)
- `serde_json` 1
- `tokio` 1 (with `features = ["full"]`, which includes `net` and `rt-multi-thread`)
- `tracing` 0.1, `tracing-subscriber` 0.3
- `futures` 0.3

The existing dependency tree already includes `hyper` 1.8, `hyper-util` 0.1, `tower` 0.5, `tower-service` 0.3, and `tower-http` 0.6 (all pulled in transitively via `rig-core`). Axum 0.8 depends on these same crates, so adding it will not introduce duplicate versions of its core transitive dependencies.

## Requirements

- Add `axum = "0.8"` to the `[dependencies]` section of `crates/agent-runtime/Cargo.toml`.
- No other new dependencies are added.
- The crate continues to compile successfully after the change (`cargo check -p agent-runtime`).
- The full workspace continues to build (`cargo build`).
- All existing tests continue to pass (`cargo test`).

## Implementation Details

- **File to modify**: `crates/agent-runtime/Cargo.toml`
  - Add the line `axum = "0.8"` under the `[dependencies]` section, placed alphabetically (i.e., before the `futures` line).
- No source files are created or modified in this task.
- No feature flags are needed on axum for the initial addition; downstream tasks may add features (e.g., `json`, `macros`) if required.
- No community skills were found for this task (`npx` was unavailable in the environment; the change is straightforward enough to not require one).

## Dependencies

- Blocked by: none
- Blocking: "Create HTTP handler module", "Wire router into main.rs"

## Risks & Edge Cases

- **Version compatibility**: Axum 0.8 requires `hyper` 1.x and `tower` 0.4+. The existing tree has `hyper` 1.8 and `tower` 0.5, both compatible. If `rig-core` later pins an incompatible hyper version, a `cargo update` or version constraint adjustment would be needed.
- **Feature bloat**: Axum's default features are lightweight, but if binary size becomes a concern, `default-features = false` can be used with explicit feature selection in a follow-up.
- **No runtime impact**: This task only adds a dependency line. Until downstream tasks import and use axum, there is zero behavioral change to the crate.

## Verification

1. Run `cargo check -p agent-runtime` -- must succeed with no errors.
2. Run `cargo build` -- full workspace must compile.
3. Run `cargo test` -- all existing tests must pass.
4. Inspect `Cargo.lock` to confirm `axum 0.8.x` appears and that `hyper`, `tower`, and `tower-service` are not duplicated at incompatible versions.
