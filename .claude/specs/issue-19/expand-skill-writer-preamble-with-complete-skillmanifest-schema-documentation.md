# Spec: Expand skill-writer preamble with complete SkillManifest schema documentation
> From: .claude/tasks/issue-19.md

## Objective

Rewrite the markdown body (preamble) of `skills/skill-writer.md` to transform it from a shallow 18-line stub into a comprehensive skill file format specification. The preamble is the skill-writer agent's sole reference material for generating valid skill files -- it must encode the complete `SkillManifest` schema, all field types and constraints, validation rules, and a detailed generation process. The YAML frontmatter must not be modified.

## Current State

The file `skills/skill-writer.md` has correct YAML frontmatter (lines 1-23) that passes all integration tests. The markdown body (lines 24-41) contains:

- A single introductory sentence (line 24)
- A `## Process` section with 7 numbered steps (lines 26-34) that are high-level and do not reference specific schema fields or validation rules
- A `## Output` section (lines 36-41) describing `skill_yaml` and `validation_result` JSON fields

The preamble is too shallow to serve its purpose. The skill-writer agent is the first seed agent in Spore's self-bootstrapping factory. Its effectiveness depends entirely on how thoroughly it encodes the skill file schema in its preamble, because the LLM will use this text as its only reference for generating valid skill files.

Other skill files in the project (`cogs-analyst.md`, `orchestrator.md`) have domain-appropriate preambles of 15-20 lines. The skill-writer is unique because its domain _is_ the skill file format itself, so its preamble must be substantially longer to document the full schema.

## Requirements

### 1. Expanded introductory paragraph

Replace the current single-sentence introduction with a paragraph that:
- Establishes the skill-writer as the first seed agent in Spore's self-bootstrapping factory
- Mentions its partnership with the tool-coder agent (issue #20)
- Explains that given a plain-language description, it produces a validated skill file in markdown-with-frontmatter format
- Sets the expectation that the agent must follow the format specification defined below

### 2. `## Skill File Format Specification` section

Document the complete file structure and every `SkillManifest` field. This section must cover:

**File structure overview:**
- Skill files are markdown files with YAML frontmatter delimited by `---` lines
- The YAML frontmatter contains all fields except `preamble`
- The markdown body after the closing `---` becomes the `preamble` field
- Together they map to the `SkillManifest` struct (8 fields total)

**Top-level fields** (from `SkillManifest` in `crates/agent-sdk/src/skill_manifest.rs`):

| Field | Type | Description |
|---|---|---|
| `name` | `String` | Unique skill identifier, used as the filename stem |
| `version` | `String` | Semantic version; must be quoted in YAML to prevent float coercion |
| `description` | `String` | One-line summary of the skill's purpose |
| `model` | `ModelConfig` | LLM configuration (see sub-fields below) |
| `tools` | `Vec<String>` | List of tool names the skill requires |
| `constraints` | `Constraints` | Execution guardrails (see sub-fields below) |
| `output` | `OutputSchema` | Response format specification (see sub-fields below) |
| `preamble` | `String` | Behavioral instructions (from markdown body, not YAML) |

**`ModelConfig` sub-fields** (from `crates/agent-sdk/src/model_config.rs`):

| Field | Type | Notes |
|---|---|---|
| `model.provider` | `String` | e.g., `anthropic` |
| `model.name` | `String` | e.g., `claude-sonnet-4-6` |
| `model.temperature` | `f64` | Controls randomness; lower values for deterministic tasks |

**`Constraints` sub-fields** (from `crates/agent-sdk/src/constraints.rs`):

| Field | Type | Notes |
|---|---|---|
| `constraints.max_turns` | `u32` | Maximum agent interaction turns; must be > 0 |
| `constraints.confidence_threshold` | `f64` | Minimum confidence to proceed; must be in [0.0, 1.0] |
| `constraints.escalate_to` | `Option<String>` | Agent to escalate to when confidence is insufficient; omit entirely when not needed (uses `#[serde(default)]`); must not be an empty string if provided |
| `constraints.allowed_actions` | `Vec<String>` | Permitted action types (e.g., `read`, `write`, `query`, `route`, `discover`) |

**`OutputSchema` sub-fields** (from `crates/agent-sdk/src/output_schema.rs`):

| Field | Type | Notes |
|---|---|---|
| `output.format` | `String` | Must be one of: `json`, `structured_json`, `text` (defined in `ALLOWED_OUTPUT_FORMATS`) |
| `output.schema` | `HashMap<String, String>` | Key-value pairs where keys are field names and values are descriptive type labels (e.g., `string`, `float`) |

### 3. `## Validation Rules` section

Document all rules enforced by `crates/skill-loader/src/validation.rs`:

1. **Required non-empty strings**: `name`, `version`, `model.provider`, and `model.name` must not be empty or whitespace-only
2. **Non-empty preamble**: The markdown body after the closing `---` must not be empty or whitespace-only
3. **Confidence threshold range**: `confidence_threshold` must be in `[0.0, 1.0]` inclusive
4. **Max turns positive**: `max_turns` must be greater than 0
5. **Tool existence**: Every tool name in the `tools` list must exist in the tool registry
6. **Output format validity**: `output.format` must be one of `"json"`, `"structured_json"`, or `"text"`
7. **Escalate-to non-empty when present**: If `escalate_to` is provided, it must not be an empty or whitespace-only string; omit the field entirely to default to `None`
8. **Version quoting**: `version` must be quoted in YAML (e.g., `"1.0"`) to prevent YAML interpreting it as a float
9. **No standalone `---` in preamble body**: The frontmatter parser uses `---` as a delimiter; a standalone `---` line in the markdown body could be misinterpreted as a frontmatter boundary

### 4. Expanded `## Process` section

Replace the current 7 high-level steps with more detailed steps that reference the format specification and validation rules. The steps should cover:

1. Analyze the input description to identify core capability, required tools, and domain constraints
2. Determine model configuration based on task complexity (provider, model name, temperature)
3. Identify required tools and verify they exist in the tool registry
4. Select appropriate constraints (max_turns, confidence_threshold, escalate_to, allowed_actions)
5. Choose the output format and define the output schema fields
6. Generate the complete YAML frontmatter conforming to the Skill File Format Specification above
7. Write the markdown preamble body with clear behavioral guidelines, ensuring it is non-empty and does not contain standalone `---` lines
8. Validate the complete skill file against all Validation Rules listed above
9. Return the generated skill file and validation results

### 5. Keep existing `## Output` section

Preserve the existing output section content describing `skill_yaml` and `validation_result` structured JSON fields. Minor wording improvements are acceptable but the semantic content must not change.

### Do NOT modify YAML frontmatter

Lines 1-23 (the `---`-delimited YAML block) must remain exactly as they are. The frontmatter is already correct and passes all integration tests.

## Implementation Details

### Section ordering in the final file

The markdown body should be structured as:

1. Introductory paragraph(s) -- no heading
2. `## Skill File Format Specification`
3. `## Validation Rules`
4. `## Process`
5. `## Output`

### Formatting conventions

- Use markdown tables for field documentation where appropriate (matching patterns in the codebase)
- Use numbered lists for sequential steps and ordered rules
- Use backtick formatting for field names, types, and values (e.g., `confidence_threshold`, `f64`, `"structured_json"`)
- Use `----` (four dashes) instead of `---` (three dashes) if horizontal rules are needed in the preamble, to avoid conflicting with the frontmatter delimiter parser
- Do not use HTML or complex markdown constructs; keep it readable as plain text since this becomes an LLM prompt

### Content accuracy

All field names, types, default behaviors, and validation rules must exactly match the Rust source code:

- `SkillManifest`: `crates/agent-sdk/src/skill_manifest.rs`
- `ModelConfig`: `crates/agent-sdk/src/model_config.rs`
- `Constraints`: `crates/agent-sdk/src/constraints.rs` (note `#[serde(default, skip_serializing_if = "Option::is_none")]` on `escalate_to`)
- `OutputSchema`: `crates/agent-sdk/src/output_schema.rs` (note `ALLOWED_OUTPUT_FORMATS` constant)
- Validation logic: `crates/skill-loader/src/validation.rs` (7 check functions)

### Key phrases the preamble must contain

The downstream integration test (Group 2 task) will assert keyword presence. The preamble should naturally contain these terms:

- "SkillManifest" or "skill file format"
- "confidence_threshold"
- "ModelConfig" or "model"
- "OutputSchema" or "output format"
- "validation"

These should appear organically within the specification text, not as forced insertions.

## Dependencies

- `crates/agent-sdk/src/skill_manifest.rs` -- `SkillManifest` struct definition (8 fields)
- `crates/agent-sdk/src/model_config.rs` -- `ModelConfig` struct definition (3 fields)
- `crates/agent-sdk/src/constraints.rs` -- `Constraints` struct definition (4 fields, `escalate_to` is `Option<String>` with serde defaults)
- `crates/agent-sdk/src/output_schema.rs` -- `OutputSchema` struct definition (2 fields) and `ALLOWED_OUTPUT_FORMATS` constant
- `crates/skill-loader/src/validation.rs` -- `validate()` function and 7 individual check functions
- `crates/skill-loader/src/frontmatter.rs` -- `SkillFrontmatter` struct, `extract_frontmatter` parser (uses `---` as delimiter)
- `skills/cogs-analyst.md`, `skills/orchestrator.md` -- preamble style reference (though skill-writer preamble will be longer due to its schema-documentation purpose)

## Risks & Edge Cases

1. **Standalone `---` in preamble**: The frontmatter parser in `frontmatter.rs` scans for `---` to find the closing delimiter. If the preamble body contains a line that is exactly `---` (trimmed), it could cause parse failures or incorrect splitting. Avoid `---` on its own line; use `----` (four dashes) for horizontal rules if needed.

2. **Preamble length vs. LLM context**: The expanded preamble will be substantially longer than other skill preambles (likely 80-120 lines vs. 15-20 lines for other skills). This is intentional and necessary -- the skill-writer's domain is the format itself. However, the preamble should remain concise and avoid unnecessary repetition to minimize token usage.

3. **YAML frontmatter regression**: Any accidental modification to the YAML frontmatter (lines 1-23) could break the 4 existing integration tests. The implementation must leave the frontmatter byte-identical.

4. **Version quoting documentation**: The spec documents that `version` must be quoted in YAML. This is a critical rule because YAML parses `1.0` as a float, which would fail `String` deserialization. The preamble must call this out clearly.

5. **Tool stub names**: The frontmatter declares `write_file` and `validate_skill` as tools. These do not exist in the tool registry yet (they will be created by issue #20 tool-coder or issue #8). The preamble should not reference these specific tool names as examples of valid tools, since they are specific to this skill. The format spec should document the `tools` field generically.

6. **`escalate_to` omission semantics**: The `Constraints` struct uses `#[serde(default, skip_serializing_if = "Option::is_none")]`, meaning the field can be entirely omitted from YAML to get `None`. This is different from providing `escalate_to: ""` (which would fail validation). The preamble must document this distinction clearly.

7. **`output.schema` is descriptive only**: The `HashMap<String, String>` values like `"string"` and `"float"` are descriptive labels, not enforced types. The preamble should document this to prevent confusion.

## Verification

1. **Frontmatter unchanged**: Diff the YAML frontmatter (lines 1-23) before and after the change to confirm it is byte-identical.

2. **Preamble non-empty**: Confirm the markdown body after the closing `---` is non-empty and contains substantive content.

3. **No standalone `---` lines**: Verify the preamble body does not contain any line that is exactly `---` (trimmed), which would conflict with the frontmatter parser.

4. **Key phrase presence**: Verify the preamble contains the following terms (case-insensitive or natural occurrences):
   - "SkillManifest" or "skill file format"
   - "confidence_threshold"
   - "ModelConfig" or "model"
   - "OutputSchema" or "output format"
   - "validation"

5. **Schema completeness**: Verify all 8 `SkillManifest` fields are documented, all 3 `ModelConfig` fields, all 4 `Constraints` fields, and all 2 `OutputSchema` fields.

6. **Validation rules completeness**: Verify all 7 check functions from `validation.rs` are documented as rules, plus the YAML quoting and `---` delimiter rules.

7. **Section structure**: Verify the preamble contains sections titled `## Skill File Format Specification`, `## Validation Rules`, `## Process`, and `## Output`.

8. **Cargo test**: Run `cargo test` to confirm all existing integration tests still pass (frontmatter parsing, deserialization, validation, field value assertions).

9. **Cargo clippy / cargo check**: Run to confirm no build or lint regressions.

### Blocking

This spec blocks:
- "Strengthen integration test for skill-writer preamble content" (Group 2 task)
