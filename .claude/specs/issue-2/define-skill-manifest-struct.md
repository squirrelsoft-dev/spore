# Spec: Define `SkillManifest` struct

> From: .claude/tasks/issue-2.md

## Objective

Create `crates/agent-sdk/src/skill_manifest.rs` containing a `SkillManifest` struct that represents a fully deserialized YAML skill file. The struct composes the three sub-types (`ModelConfig`, `Constraints`, `OutputSchema`) alongside scalar and list fields, and must round-trip cleanly through `serde_yaml` serialization/deserialization. Field names must match the canonical YAML example in the README (lines 19-53) exactly.

## Current State

- `crates/agent-sdk/src/lib.rs` contains only a placeholder `add()` function and a trivial test. No real types exist yet.
- `crates/agent-sdk/Cargo.toml` lists no dependencies -- `serde`, `schemars` have not been added yet (prerequisite task from Group 1).
- The three sub-types (`ModelConfig`, `Constraints`, `OutputSchema`) do not exist yet. They are separate tasks in Group 2 and must land before this file can compile.
- No module declarations or re-exports exist in `lib.rs` (that is a downstream task in Group 3).

## Requirements

1. **File location:** `crates/agent-sdk/src/skill_manifest.rs`

2. **Struct definition:** A public struct named `SkillManifest` with the following fields, all public:

   | Field         | Type              | Maps to YAML key   |
   |---------------|-------------------|---------------------|
   | `name`        | `String`          | `name`              |
   | `version`     | `String`          | `version`           |
   | `description` | `String`          | `description`       |
   | `preamble`    | `String`          | `preamble`          |
   | `tools`       | `Vec<String>`     | `tools`             |
   | `model`       | `ModelConfig`     | `model`             |
   | `constraints` | `Constraints`     | `constraints`       |
   | `output`      | `OutputSchema`    | `output`            |

3. **Derive macros:** `Debug`, `Clone`, `Serialize`, `Deserialize`, `JsonSchema`.

4. **Imports required:**
   - `serde::{Serialize, Deserialize}`
   - `schemars::JsonSchema`
   - Crate-local imports for `ModelConfig` (from `crate::model_config`), `Constraints` (from `crate::constraints`), `OutputSchema` (from `crate::output_schema`).

5. **No serde attributes needed:** All field names already match the YAML keys exactly (snake_case matches the YAML convention used in the README). No `#[serde(rename = ...)]` or `#[serde(rename_all = ...)]` annotations are required.

6. **No default values or `Option` wrappers:** Every field in the canonical YAML example is present and required. All fields are non-optional. If a future task introduces optional fields, that is a separate change.

7. **No methods or trait implementations beyond the derives.** This is a pure data struct. Validation logic, constructors, and builder patterns are out of scope.

## Implementation Details

The file should contain:

1. Module-level doc comment explaining the struct's purpose (one or two lines).
2. `use` statements for serde derives, schemars, and the three sub-types.
3. The `SkillManifest` struct definition with a doc comment and the five derive macros.
4. Nothing else -- no tests (those belong in `crates/agent-sdk/tests/skill_manifest_test.rs` per the task breakdown), no helper functions, no constants.

Field ordering in the struct must match the YAML key ordering from the README example: `name`, `version`, `description`, `model`, `preamble`, `tools`, `constraints`, `output`. Note that the YAML example places `model` before `preamble`, so the struct field order should follow that same sequence for readability, even though serde does not require it.

Corrected field order based on the README YAML (lines 19-53):

| Position | YAML key      |
|----------|---------------|
| 1        | `name`        |
| 2        | `version`     |
| 3        | `description` |
| 4        | `model`       |
| 5        | `preamble`    |
| 6        | `tools`       |
| 7        | `constraints` |
| 8        | `output`      |

## Dependencies

**Blocked by (must exist before this file compiles):**
- `serde` and `schemars` dependencies in `Cargo.toml` (Group 1 task)
- `crates/agent-sdk/src/model_config.rs` defining `ModelConfig` (Group 2)
- `crates/agent-sdk/src/constraints.rs` defining `Constraints` (Group 2)
- `crates/agent-sdk/src/output_schema.rs` defining `OutputSchema` (Group 2)

**Blocking (cannot proceed until this file lands):**
- `lib.rs` module declarations and re-exports (Group 3 downstream task)
- Deserialization tests in `crates/agent-sdk/tests/skill_manifest_test.rs` (Group 4)

## Risks & Edge Cases

1. **Field order mismatch with YAML:** The task description lists fields in a different order than the README YAML (`preamble` before `tools` vs `model` before `preamble`). The struct field order should match the YAML document order for maintainability. Serde deserialization is order-independent for structs, so this is a readability concern, not a correctness concern.

2. **`version` as `String` vs semver type:** The task specifies `String`. This is correct for now -- `"1.0.0"` is just a string in the YAML. A future task could introduce a `semver::Version` type, but that would add a dependency and is out of scope.

3. **`preamble` with YAML literal block scalar:** The README example uses `preamble: |` (literal block scalar with trailing newline). `serde_yaml` handles this transparently -- it deserializes into a `String` with embedded newlines. No special handling is needed in the struct definition, but tests should verify that the trailing newline is preserved.

4. **Sub-type import paths:** This file uses `crate::model_config::ModelConfig` etc., which requires that `lib.rs` declares `mod model_config;` etc. Since the `lib.rs` update is a downstream task, this file will not compile in isolation until that task completes. This is expected per the dependency graph.

5. **No `PartialEq` derive:** The task description does not list `PartialEq` in the derives for this struct. The testing task (Group 4) notes that `PartialEq` will need to be added to all four structs for round-trip assertion. This can be added here proactively or deferred to the test task. The conservative choice is to add it here since it has no cost, but the spec follows the task description exactly and omits it. The implementer should be aware of this.

## Verification

1. **Compiles with `cargo check`** once all Group 1 and Group 2 dependencies are in place and `lib.rs` declares the module.
2. **`cargo clippy` produces no warnings** on the new file.
3. **Field names match the README YAML keys exactly** -- verify by visual inspection against README lines 19-53.
4. **All five derive macros are present:** `Debug`, `Clone`, `Serialize`, `Deserialize`, `JsonSchema`.
5. **All fields are `pub`** so they are accessible from outside the module.
6. **No unnecessary imports, no dead code, no commented-out code** per the project rules in `.claude/rules/general.md`.
