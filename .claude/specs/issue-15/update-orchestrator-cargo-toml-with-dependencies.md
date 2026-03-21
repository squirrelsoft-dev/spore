# Spec: Update orchestrator Cargo.toml with dependencies

> From: .claude/tasks/issue-15.md

## Objective
Add all required dependencies to `crates/orchestrator/Cargo.toml` so that subsequent tasks (Groups 2-4) can compile against them. The orchestrator crate currently has an empty `[dependencies]` section. All dependencies being added already exist in `Cargo.lock` as transitive dependencies of other workspace crates, so this change introduces zero new crates to the dependency tree.

## Current State

**`crates/orchestrator/Cargo.toml`** is a minimal stub:
```toml
[package]
name = "orchestrator"
version = "0.1.0"
edition = "2024"

[dependencies]
```
There is no `[dev-dependencies]` section, and the `[dependencies]` section is empty.

**Workspace lockfile (`Cargo.lock`)** already contains every crate being added:
| Crate | Locked version | Pulled in by |
|---|---|---|
| `async-trait` | 0.1.89 | `agent-sdk` |
| `serde` | 1.0.228 | `agent-sdk`, `agent-runtime`, others |
| `serde_json` | 1.0.149 | `agent-sdk`, `agent-runtime`, others |
| `serde_yaml` | 0.9.34 | `agent-sdk` (dev-dep) |
| `tokio` | 1.50.0 | `agent-runtime`, others |
| `reqwest` | 0.13.2 | `rig-core` (transitive) |
| `axum` | 0.8.8 | `agent-runtime` |

**`crates/agent-runtime/Cargo.toml`** serves as the reference for dependency declaration patterns in this workspace. It uses:
- Path dependencies for workspace crates: `agent-sdk = { path = "../agent-sdk" }`
- Version-only for simple crates: `serde_json = "1"`
- Version with features for crates needing feature flags: `serde = { version = "1", features = ["derive"] }`, `tokio = { version = "1", features = ["full"] }`

**`crates/agent-sdk/Cargo.toml`** also serves as a reference, using the same patterns and already declaring `async-trait = "0.1"`, `serde_yaml = "0.9"` (as dev-dep), and `tokio` (as dev-dep).

## Requirements

1. Add the following entries under `[dependencies]` in `crates/orchestrator/Cargo.toml`:
   - `agent-sdk = { path = "../agent-sdk" }` -- provides `MicroAgent`, `AgentRequest`, `AgentResponse`, `AgentError`, `HealthStatus`, `SkillManifest`
   - `reqwest = { version = "0.13", features = ["json"] }` -- HTTP client for calling downstream agent endpoints
   - `tokio = { version = "1", features = ["full"] }` -- async runtime (needed for async methods and `join_all` in health checks)
   - `serde = { version = "1", features = ["derive"] }` -- serialization/deserialization with derive macros
   - `serde_json = "1"` -- JSON handling for request/response bodies
   - `serde_yaml = "0.9"` -- YAML config file parsing
   - `async-trait = "0.1"` -- async trait support for `MicroAgent` impl

2. Add a `[dev-dependencies]` section with:
   - `tokio = { version = "1", features = ["macros", "rt"] }` -- test runtime with `#[tokio::test]` macro support
   - `axum = "0.8"` -- used to spin up mock HTTP servers in integration tests

3. The `[package]` section must remain unchanged (`name = "orchestrator"`, `version = "0.1.0"`, `edition = "2024"`).

4. After modification, `cargo check -p orchestrator` must succeed without errors.

5. `Cargo.lock` must not gain any new top-level crate entries (all dependencies are already present as transitive deps).

6. `cargo clippy -p orchestrator` must produce no warnings.

7. All existing workspace tests (`cargo test`) must continue to pass with no regressions.

## Implementation Details

**File to modify:** `crates/orchestrator/Cargo.toml`

The final file content should be:

```toml
[package]
name = "orchestrator"
version = "0.1.0"
edition = "2024"

[dependencies]
agent-sdk = { path = "../agent-sdk" }
reqwest = { version = "0.13", features = ["json"] }
tokio = { version = "1", features = ["full"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
serde_yaml = "0.9"
async-trait = "0.1"

[dev-dependencies]
tokio = { version = "1", features = ["macros", "rt"] }
axum = "0.8"
```

**Ordering convention:** Follow the pattern from `agent-runtime/Cargo.toml` -- path dependencies first, then external crates. This is a soft convention; the key requirement is that all entries are present and correct.

**No other files need modification.** The `Cargo.lock` will be automatically updated by Cargo to reflect the new direct dependency edges, but since all crates are already resolved in the lockfile, the lock entries themselves should not change (only the dependency graph metadata within the lock may shift minimally).

## Dependencies

- **Blocked by:** Nothing -- this is a Group 1 task that can be done immediately in parallel with the other Group 1 tasks.
- **Blocking:** All Group 2 and Group 3 tasks:
  - "Implement AgentEndpoint struct" (needs `reqwest`, `serde`, `agent-sdk`)
  - "Define registry config format and loader" (needs `serde`, `serde_yaml`)
  - "Implement Orchestrator struct with dispatch logic" (needs all deps)
  - "Implement MicroAgent for Orchestrator" (needs `async-trait`, `agent-sdk`)
  - All Group 5 test tasks (need `axum` and `tokio` dev-deps)

## Risks & Edge Cases

1. **`serde_yaml` deprecation warning:** The locked version is `0.9.34+deprecated`. The `serde_yaml` crate has been deprecated in favor of alternatives, but it is already used by `agent-sdk` as a dev-dependency, so using the same version here is consistent. If the workspace migrates away from `serde_yaml` in the future, the orchestrator crate will need to follow suit.

2. **`reqwest` version compatibility:** The spec requests `version = "0.13"` which will resolve to `0.13.2` (already locked). If the workspace upgrades `reqwest` via `rig-core`, the semver range `"0.13"` will accept any `0.13.x` patch. No risk here.

3. **Duplicate `tokio` in `[dependencies]` and `[dev-dependencies]`:** Cargo handles this correctly -- the `[dev-dependencies]` entry adds the `macros` and `rt` features only for test/bench targets, while the `[dependencies]` entry with `features = ["full"]` applies to the library. Since `"full"` already includes `"macros"` and `"rt"`, the dev-dependency entry is technically redundant but is included explicitly for clarity and to match the `agent-sdk` pattern.

4. **Edition 2024 compatibility:** The crate uses `edition = "2024"`. All listed dependencies support Rust edition 2024. No compatibility issues expected.

## Verification

1. Run `cargo check -p orchestrator` -- must succeed with exit code 0.
2. Run `cargo clippy -p orchestrator` -- must produce no warnings or errors.
3. Run `cargo test` (full workspace) -- must pass with no regressions in existing crates.
4. Inspect `Cargo.lock` diff to confirm no entirely new crate entries were added (only dependency-graph edges may change).
5. Verify the final `Cargo.toml` contains all seven `[dependencies]` entries and both `[dev-dependencies]` entries listed in the Requirements section.
