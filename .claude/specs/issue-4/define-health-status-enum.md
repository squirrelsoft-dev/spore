# Spec: Define `HealthStatus` enum

> From: .claude/tasks/issue-4.md

## Objective

Create a `HealthStatus` enum in `crates/agent-sdk/src/health_status.rs` that represents the three possible health states of a micro-agent: healthy, degraded (with a reason), or unhealthy (with a reason). This type will be returned by `MicroAgent::health()` (defined in issue #3) to allow orchestrators and monitoring systems to query agent readiness.

## Current State

- `crates/agent-sdk/src/lib.rs` declares four modules (`constraints`, `model_config`, `output_schema`, `skill_manifest`) and re-exports their primary types.
- All existing types follow a consistent pattern: one type per file, `use serde::{Deserialize, Serialize};` and `use schemars::JsonSchema;` at the top, a single `#[derive(...)]` line, public type with public fields.
- The crate's `Cargo.toml` already has `serde` (with `derive` feature) and `schemars` (with `derive` feature) as dependencies, which are the only dependencies this type requires.
- No `health_status.rs` file exists yet.

## Requirements

1. Create the file `crates/agent-sdk/src/health_status.rs`.
2. Define a public enum `HealthStatus` with exactly three variants:
   - `Healthy` -- unit variant, no payload. The agent is operating normally.
   - `Degraded(String)` -- tuple variant carrying a human-readable reason. The agent is operational but experiencing issues (e.g., elevated latency, degraded accuracy, partial feature unavailability).
   - `Unhealthy(String)` -- tuple variant carrying a human-readable reason. The agent cannot serve requests (e.g., missing API key, downstream dependency down, corrupted state).
3. Derive the following traits on `HealthStatus`: `Debug`, `Clone`, `PartialEq`, `Serialize`, `Deserialize`, `JsonSchema`.
4. No methods, impl blocks, or trait implementations beyond the derived traits are needed.
5. The file must include the necessary `use` imports for the derived traits.

## Implementation Details

- **File path:** `crates/agent-sdk/src/health_status.rs`
- **Type visibility:** `pub enum HealthStatus`
- **Derive line:** `#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]`
- **Imports needed:**
  - `use serde::{Serialize, Deserialize};`
  - `use schemars::JsonSchema;`
- **No `#[serde(...)]` attributes required** -- the default externally-tagged serde representation for enums is appropriate. A `Healthy` value serializes as `"Healthy"`, while `Degraded("slow")` serializes as `{"Degraded": "slow"}`, which is clear and idiomatic for JSON consumers.
- **No `Default` implementation** -- there is no universally correct default health state; callers should explicitly choose a variant.
- This module will later be declared in `lib.rs` via `mod health_status;` and re-exported via `pub use health_status::HealthStatus;`, but that wiring is handled by the separate "Update `lib.rs` module declarations and re-exports" task.

### Expected serialized forms (JSON)

```json
"Healthy"
```

```json
{"Degraded": "elevated latency to model provider"}
```

```json
{"Unhealthy": "missing API key for provider"}
```

## Dependencies

- **Blocked by:** "Add `uuid` and `serde_json` dependencies to `agent-sdk/Cargo.toml`" (Group 1). While `HealthStatus` itself does not use `uuid` or `serde_json`, it is listed in Group 2 which depends on Group 1 completing first per the task breakdown ordering. In practice, this type only requires `serde` and `schemars`, which are already present, so it could be implemented immediately.
- **Blocking:** "Update `lib.rs` module declarations and re-exports" (Group 4). The `lib.rs` wiring task needs this file to exist before it can add the `mod` and `pub use` declarations.

## Risks & Edge Cases

1. **Serde enum representation:** The default externally-tagged representation is the right choice here. If a different representation (e.g., adjacently tagged with `#[serde(tag = "status", content = "reason")]`) were needed for wire compatibility with other systems, that would be a breaking change. For now, the default is sufficient and the most idiomatic.
2. **Reason string conventions:** There is no validation or constraint on the `String` payloads in `Degraded` and `Unhealthy`. Callers are expected to provide concise, human-readable descriptions. If structured error information is needed in the future, the payload type could be changed to a struct, but that would be a breaking change. The current design is intentionally simple.
3. **`PartialEq` on `String` payloads:** Equality comparison includes the reason string, so `Degraded("reason A") != Degraded("reason B")`. This is the correct behavior -- two degraded states with different reasons are semantically distinct.
4. **No `Eq` derive:** `HealthStatus` could also derive `Eq` (since `String` implements `Eq`), but the task description specifies `PartialEq` only. `Eq` can be added later if needed without breaking changes.

## Verification

After implementation (and after the Group 1 dependency task is complete), run:

```bash
cargo check -p agent-sdk
cargo clippy -p agent-sdk
```

Both commands must pass with no errors and no warnings. Full `cargo test` validation (including serialization round-trip tests for all `HealthStatus` variants) is covered by the separate "Write serialization and construction tests" task in Group 5.
