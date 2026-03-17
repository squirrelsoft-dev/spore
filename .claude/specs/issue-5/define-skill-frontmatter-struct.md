# Spec: Define SkillFrontmatter struct

> From: .claude/tasks/issue-5.md

## Objective

Create a crate-private `SkillFrontmatter` struct that serves as the deserialization target for YAML frontmatter embedded in skill markdown files. Skill files store their metadata as YAML frontmatter (between `---` delimiters) and the preamble as the markdown body below. Since the YAML block does not contain a `preamble` field, a separate struct without that field is needed to cleanly deserialize frontmatter before the loader combines it with the extracted body to produce a full `SkillManifest`.

## Current State

- `agent-sdk` defines and re-exports the following public types used as field types:
  - `SkillManifest` (`crates/agent-sdk/src/skill_manifest.rs`) -- has fields: `name`, `version`, `description`, `model`, `preamble`, `tools`, `constraints`, `output`.
  - `ModelConfig` (`crates/agent-sdk/src/model_config.rs`) -- fields: `provider: String`, `name: String`, `temperature: f64`.
  - `Constraints` (`crates/agent-sdk/src/constraints.rs`) -- fields: `max_turns: u32`, `confidence_threshold: f64`, `escalate_to: String`, `allowed_actions: Vec<String>`.
  - `OutputSchema` (`crates/agent-sdk/src/output_schema.rs`) -- fields: `format: String`, `schema: HashMap<String, String>`.
  - All four types derive `Deserialize` (among other traits) and are re-exported from `agent-sdk`'s `lib.rs`.
- `skill-loader` crate exists at `crates/skill-loader/` with a placeholder `lib.rs`. The file `crates/skill-loader/src/frontmatter.rs` does not yet exist.
- `skill-loader/Cargo.toml` currently has no dependencies. The sibling task "Add dependencies to skill-loader Cargo.toml" will add `serde`, `serde_yaml`, and `agent-sdk` as dependencies before this task's code can compile.

## Requirements

- Create file `crates/skill-loader/src/frontmatter.rs`.
- Define a struct named `SkillFrontmatter` that is **not** `pub` (i.e., `pub(crate)` at most, visible only within the `skill-loader` crate).
- The struct must have exactly these fields, matching `SkillManifest` minus `preamble`:
  - `name: String`
  - `version: String`
  - `description: String`
  - `model: ModelConfig`
  - `tools: Vec<String>`
  - `constraints: Constraints`
  - `output: OutputSchema`
- All fields must be `pub(crate)` so that `lib.rs` (the `SkillLoader`) can read them when constructing a `SkillManifest`.
- Derive only `serde::Deserialize` on the struct. Do not derive `Serialize`, `Debug`, `Clone`, `JsonSchema`, or any other traits unless required for compilation.
- Import `ModelConfig`, `Constraints`, and `OutputSchema` from `agent_sdk` (the crate dependency).
- Import `Deserialize` from `serde`.
- The file must declare the module with `mod frontmatter;` in `lib.rs` (this wiring is done by the "Implement SkillLoader struct and load method" task, not this task, but keep it in mind for integration).
- No functions, impls, or tests are required in this task. The `extract_frontmatter` function and tests are separate tasks.

## Implementation Details

### File to create

**`crates/skill-loader/src/frontmatter.rs`**

- Add `use serde::Deserialize;` for the derive macro.
- Add `use agent_sdk::{ModelConfig, Constraints, OutputSchema};` to import the shared types.
- Define the struct:
  ```rust
  #[derive(Deserialize)]
  pub(crate) struct SkillFrontmatter {
      pub(crate) name: String,
      pub(crate) version: String,
      pub(crate) description: String,
      pub(crate) model: ModelConfig,
      pub(crate) tools: Vec<String>,
      pub(crate) constraints: Constraints,
      pub(crate) output: OutputSchema,
  }
  ```
- No other code is needed in this file for this task. Later tasks will add `extract_frontmatter` and tests to the same file.

### Integration points

- The downstream `SkillLoader::load` method (Group 3 task) will deserialize YAML into `SkillFrontmatter` via `serde_yaml::from_str::<SkillFrontmatter>(yaml_str)`, then construct a `SkillManifest` by copying all fields and setting `preamble` to the extracted markdown body.
- The `extract_frontmatter` function (sibling Group 2 task) will also be placed in this file but is out of scope for this task.

## Dependencies

- **Blocked by:** "Add dependencies to skill-loader Cargo.toml" -- the file will not compile until `serde`, `serde_yaml`, and `agent-sdk` are added as dependencies.
- **Blocking:** "Implement SkillLoader struct and load method" -- the loader needs `SkillFrontmatter` to deserialize frontmatter.

## Risks & Edge Cases

- **Field drift:** If `SkillManifest` gains or loses fields (other than `preamble`), `SkillFrontmatter` must be updated in lockstep. There is no compile-time enforcement that the two structs stay in sync. Mitigation: the integration tests in Group 4 will round-trip a full skill file through `SkillLoader::load` and assert all fields, catching drift.
- **Strict deserialization:** By default, `serde_yaml` will reject unknown keys. If skill files later include extra YAML keys (e.g., comments metadata), deserialization will fail. Mitigation: this is acceptable for now; if needed later, `#[serde(deny_unknown_fields)]` or `#[serde(flatten)]` can be added deliberately.
- **Visibility scope:** The struct is `pub(crate)`, not `pub`. If external crates ever need to deserialize frontmatter directly, the visibility would need to change. For now, keeping it private to the crate is intentional -- only `SkillLoader` should use it.

## Verification

- After the sibling dependency task completes, run `cargo check -p skill-loader` (after adding `mod frontmatter;` to `lib.rs`) and confirm no compilation errors.
- Confirm the struct is not visible outside the crate: adding `use skill_loader::frontmatter::SkillFrontmatter;` in an external test should fail to compile.
- Confirm the struct deserializes valid YAML correctly by writing a unit test (covered by Group 4, but can be spot-checked manually):
  ```rust
  let yaml = r#"
  name: test-skill
  version: "1.0"
  description: A test
  model:
    provider: openai
    name: gpt-4
    temperature: 0.7
  tools:
    - search
  constraints:
    max_turns: 5
    confidence_threshold: 0.8
    escalate_to: human
    allowed_actions:
      - read
  output:
    format: json
    schema:
      result: string
  "#;
  let fm: SkillFrontmatter = serde_yaml::from_str(yaml).unwrap();
  assert_eq!(fm.name, "test-skill");
  ```
- Run `cargo clippy -p skill-loader` and confirm no warnings.
