# Spec: Define `ModelConfig` struct

> From: .claude/tasks/issue-2.md

## Objective

Create a `ModelConfig` struct in `crates/agent-sdk/src/model_config.rs` that represents the `model:` section of a YAML skill file. This struct will be composed into `SkillManifest` and must round-trip through serde serialization/deserialization with field names matching the canonical YAML keys exactly.

## Current State

- `crates/agent-sdk/src/lib.rs` contains only a placeholder `add()` function and a trivial test. No domain types exist yet.
- `crates/agent-sdk/Cargo.toml` lists no dependencies -- `serde`, `schemars`, and their derive features have not been added yet.
- No `model_config.rs` file exists in the crate.

## Requirements

1. Create the file `crates/agent-sdk/src/model_config.rs`.
2. Define a public struct `ModelConfig` with exactly three public fields:
   - `provider: String` -- the model provider (e.g., `"anthropic"`). Maps to YAML key `provider`.
   - `name: String` -- the model identifier (e.g., `"claude-sonnet-4-6"`). Maps to YAML key `name`.
   - `temperature: f64` -- sampling temperature (e.g., `0.1`). Maps to YAML key `temperature`.
3. Derive the following traits on `ModelConfig`: `Debug`, `Clone`, `Serialize`, `Deserialize`, `JsonSchema`.
4. Field names in the struct must match the YAML keys from the README example (lines 24-27) verbatim, so no `#[serde(rename)]` attributes should be needed.
5. The file must include the necessary `use` imports for the derived traits (`serde::Serialize`, `serde::Deserialize`, `schemars::JsonSchema`).

## Implementation Details

- **File path:** `crates/agent-sdk/src/model_config.rs`
- **Struct visibility:** `pub struct ModelConfig`
- **Field visibility:** All fields `pub`
- **Derive line:** `#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]`
- **Imports needed:**
  - `use serde::{Serialize, Deserialize};`
  - `use schemars::JsonSchema;`
- **No `#[serde(...)]` attributes required** -- the Rust field names (`provider`, `name`, `temperature`) already match the YAML keys exactly.
- **No default values or `Option` wrappers** -- all three fields are required per the canonical YAML example. A skill file missing any of these fields should fail deserialization.
- This module will later be declared in `lib.rs` via `pub mod model_config;` and re-exported via `pub use model_config::ModelConfig;`, but that wiring is handled by a separate task.

### Reference YAML (README lines 24-27)

```yaml
model:
  provider: anthropic
  name: claude-sonnet-4-6
  temperature: 0.1
```

## Dependencies

- **Blocked by:** "Add `serde`, `schemars` dependencies to `agent-sdk/Cargo.toml`" (Group 1). The derive macros will not compile without those crate dependencies being present in `Cargo.toml`.
- **Blocks:** "Define `SkillManifest` struct" (Group 3). `SkillManifest` composes `ModelConfig` as its `model` field.

## Risks & Edge Cases

1. **`temperature` precision:** `f64` faithfully represents `0.1` for practical purposes, but floating-point equality comparisons in tests should use approximate matching or accept the serde round-trip value. This is primarily a concern for the test task, not this struct definition.
2. **Future extensibility:** Additional model parameters (e.g., `max_tokens`, `top_p`) may be added later. If forward-compatible parsing is desired, a `#[serde(deny_unknown_fields)]` attribute should NOT be added now. Conversely, if strict validation is preferred, it can be added in a follow-up. For this task, omit it -- keep the struct simple and defer that decision.
3. **`name` field ambiguity:** The field `name` could collide conceptually with `SkillManifest::name`. This is fine because `ModelConfig` is a separate type and the field clearly refers to the model name within its context.
4. **Dependency not yet added:** If this task is attempted before the Group 1 dependency task completes, `cargo check` will fail on the missing `serde` and `schemars` crates. The implementation must not proceed until `Cargo.toml` is updated.

## Verification

After implementation (and after the dependency task is complete), run:

```bash
cargo check -p agent-sdk
cargo clippy -p agent-sdk
```

Both commands must pass with no errors and no warnings. Full `cargo test` validation is covered by the separate deserialization test task in Group 4.
