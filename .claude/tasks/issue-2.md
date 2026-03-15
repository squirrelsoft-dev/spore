# Task Breakdown: Define SkillManifest and related config types

> Add the core data types to the `agent-sdk` crate that represent a deserialized YAML skill file, including `SkillManifest`, `Constraints`, `ModelConfig`, and `OutputSchema`.

## Group 1 — Add dependencies to agent-sdk

_Tasks in this group must be done first, before any type definitions._

- [x] **Add `serde`, `schemars` dependencies to `agent-sdk/Cargo.toml`** `[S]`
      Add `serde` (with `derive` feature) and `schemars` as dependencies in `crates/agent-sdk/Cargo.toml`. These are required for `Serialize`, `Deserialize`, and `JsonSchema` derive macros on all types. Also add `serde_yaml` as a `[dev-dependencies]` entry for testing deserialization of the README's canonical YAML example. This is the only task that adds new dependencies, justified by the issue requirements and the README tech stack (line 111).
      Files: `crates/agent-sdk/Cargo.toml`
      Blocking: all tasks in Group 2 and Group 3

## Group 2 — Define the four config types

_Tasks in this group can be done in parallel. Each type lives in its own module file for single-responsibility._

- [x] **Define `ModelConfig` struct** `[S]`
      Create `crates/agent-sdk/src/model_config.rs` with a `ModelConfig` struct containing fields: `provider: String`, `name: String`, `temperature: f64`. Derive `Debug`, `Clone`, `Serialize`, `Deserialize`, `JsonSchema`. The field names must match the YAML keys from the README example (lines 24-27).
      Files: `crates/agent-sdk/src/model_config.rs`
      Blocking: "Define `SkillManifest` struct"

- [x] **Define `Constraints` struct** `[S]`
      Create `crates/agent-sdk/src/constraints.rs` with a `Constraints` struct containing fields: `max_turns: u32`, `confidence_threshold: f64`, `escalate_to: String`, `allowed_actions: Vec<String>`. Derive `Debug`, `Clone`, `Serialize`, `Deserialize`, `JsonSchema`.
      Files: `crates/agent-sdk/src/constraints.rs`
      Blocking: "Define `SkillManifest` struct"

- [x] **Define `OutputSchema` struct** `[S]`
      Create `crates/agent-sdk/src/output_schema.rs` with an `OutputSchema` struct containing fields: `format: String`, `schema: HashMap<String, String>`. Derive `Debug`, `Clone`, `Serialize`, `Deserialize`, `JsonSchema`. Import `std::collections::HashMap`.
      Files: `crates/agent-sdk/src/output_schema.rs`
      Blocking: "Define `SkillManifest` struct"

## Group 3 — Define SkillManifest and wire up module tree

_Depends on: Group 2_

- [x] **Define `SkillManifest` struct** `[S]`
      Create `crates/agent-sdk/src/skill_manifest.rs` with a `SkillManifest` struct that composes the three sub-types: `name: String`, `version: String`, `description: String`, `preamble: String`, `tools: Vec<String>`, `model: ModelConfig`, `constraints: Constraints`, `output: OutputSchema`. Derive `Debug`, `Clone`, `Serialize`, `Deserialize`, `JsonSchema`.
      Files: `crates/agent-sdk/src/skill_manifest.rs`
      Blocked by: ModelConfig, Constraints, OutputSchema
      Blocking: "Update lib.rs module declarations and re-exports"

- [x] **Update `lib.rs` module declarations and re-exports** `[S]`
      Replace the placeholder `add()` function and its test in `crates/agent-sdk/src/lib.rs` with `mod` declarations for `model_config`, `constraints`, `output_schema`, and `skill_manifest`. Add `pub use` re-exports so consumers can write `use agent_sdk::SkillManifest` etc.
      Files: `crates/agent-sdk/src/lib.rs`
      Blocked by: SkillManifest struct
      Blocking: "Write deserialization tests"

## Group 4 — Tests and verification

_Depends on: Group 3_

- [x] **Write deserialization tests** `[M]`
      Add tests that: (1) deserialize the exact YAML example from the README (lines 19-53) into a `SkillManifest` and assert all field values; (2) serialize a `SkillManifest` to YAML and deserialize it back, asserting equality (requires `PartialEq` derive on all types); (3) test edge cases for empty tools list and empty schema map. Use `serde_yaml` (dev-dependency). Add `PartialEq` derive to all four structs.
      Files: `crates/agent-sdk/tests/skill_manifest_test.rs`
      Blocked by: lib.rs re-exports
      Blocking: None

- [x] **Run `cargo check`, `cargo clippy`, `cargo test` to verify** `[S]`
      Run the full verification suite per CLAUDE.md project commands. Ensure no clippy warnings, all tests pass, and the crate compiles cleanly.
      Files: (none — command-line only)
      Blocked by: deserialization tests
