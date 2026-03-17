# Spec: Create `skills/skill-writer.md` bootstrap seed agent stub

> From: .claude/tasks/issue-7.md

## Objective

Create the seed agent skill file for the self-bootstrapping factory milestone. The skill-writer is the first of two seed agents described in the README that together enable the platform to extend itself: given a plain-language capability description, it produces a validated skill file in markdown-with-frontmatter format. This task creates `skills/skill-writer.md` containing the full skill definition with YAML frontmatter and a multi-step markdown preamble.

## Current State

- **`SkillManifest`** (`crates/agent-sdk/src/skill_manifest.rs`) defines eight fields: `name`, `version`, `description`, `model` (ModelConfig), `preamble`, `tools` (Vec), `constraints` (Constraints), `output` (OutputSchema).
- **`ModelConfig`** (`crates/agent-sdk/src/model_config.rs`): `provider: String`, `name: String`, `temperature: f64`.
- **`Constraints`** (`crates/agent-sdk/src/constraints.rs`): `max_turns: u32`, `confidence_threshold: f64`, `escalate_to: Option<String>` (changed in issue #6 with `#[serde(default, skip_serializing_if = "Option::is_none")]`), `allowed_actions: Vec<String>`.
- **`OutputSchema`** (`crates/agent-sdk/src/output_schema.rs`): `format: String`, `schema: HashMap<String, String>`. Allowed formats: `["json", "structured_json", "text"]`.
- **`SkillFrontmatter`** (`crates/skill-loader/src/frontmatter.rs`): Contains all `SkillManifest` fields except `preamble`.
- **`SkillLoader`** (`crates/skill-loader/src/lib.rs`): `load()` constructs path as `{skill_dir}/{skill_name}.md`, extracts frontmatter, deserializes into `SkillFrontmatter`, uses `body.trim().to_string()` as preamble, and runs validation.
- **Validation** (`crates/skill-loader/src/validation.rs`): Checks required strings non-empty, preamble non-empty, confidence_threshold in [0.0, 1.0], max_turns > 0, tools exist (via `ToolExists` trait), output format in allowed list, escalate_to not empty when provided.
- The `skills/` directory exists with only `.gitkeep`.
- The README (line 98) references `skill-writer.yaml` — a sibling task will update this to `skill-writer.md`.
- Validation integration tests confirm omitting `escalate_to` defaults to `None` and passes validation.

## Requirements

- Create file `skills/skill-writer.md` in markdown-with-frontmatter format.
- YAML frontmatter must contain all `SkillFrontmatter` fields:

  | Field | Value | Notes |
  |---|---|---|
  | `name` | `skill-writer` | Must match filename stem |
  | `version` | `"0.1"` | Quoted to avoid YAML float interpretation |
  | `description` | `Produces validated skill files from plain-language descriptions` | |
  | `model.provider` | `anthropic` | |
  | `model.name` | `claude-sonnet-4-6` | |
  | `model.temperature` | `0.2` | Low temperature for deterministic generation |
  | `tools` | `[write_file, validate_skill]` | Stub names, not in registry yet |
  | `constraints.max_turns` | `10` | Higher than domain agents; generation may iterate |
  | `constraints.confidence_threshold` | `0.9` | High bar for generated skill quality |
  | `constraints.escalate_to` | *(omitted)* | Defaults to `None` via `#[serde(default)]` |
  | `constraints.allowed_actions` | `[read, write]` | |
  | `output.format` | `structured_json` | One of three allowed formats |
  | `output.schema.skill_yaml` | `string` | Generated skill file content |
  | `output.schema.validation_result` | `string` | Validation outcome description |

- Markdown body (preamble) must contain:
  1. Introductory paragraph explaining the agent's purpose as the first seed agent in Spore's self-bootstrapping factory
  2. A `## Process` section with numbered steps the agent follows
  3. A `## Output` section describing the structured JSON output format
- `escalate_to` must be **omitted entirely** (not set to empty string)
- Tool names `write_file` and `validate_skill` are intentionally stubs — they pass with `AllToolsExist` but will fail real tool-existence validation until issue #8

## Implementation Details

### File to create

**`skills/skill-writer.md`**

Structure:
```
---
<YAML frontmatter>
---

<Markdown body with numbered process steps>
```

### Preamble content guidance

The markdown body should contain:
1. An introductory paragraph about the agent taking plain-language capability descriptions and producing validated skill files
2. `## Process` section with numbered steps: analyze input, determine model config, identify tools, generate frontmatter, write preamble body, validate against schema, return results
3. `## Output` section describing `skill_yaml` and `validation_result` fields

### Integration points

- **`SkillLoader::load("skill-writer")`** will load this file.
- **Tool-registry (issue #8):** Will eventually provide `write_file` and `validate_skill` implementations.
- **README** (issue #7 Group 2): Reference on line 98 will be updated from `.yaml` to `.md`.

## Dependencies

- **Blocked by:** None (non-blocking)
- **Blocking:** None directly. Group 2 integration test will validate this file.

## Risks & Edge Cases

1. **Stub tool names will fail validation with real tool checker:** Intentional. Integration test must use `AllToolsExist`.
2. **YAML version quoting:** `version: "0.1"` must be quoted. Without quotes, YAML treats `0.1` as float, failing String deserialization.
3. **`escalate_to` must be omitted, not empty:** `Some("")` fails `check_escalate_to` validation. Omitting results in `None`, which passes.
4. **Frontmatter does not round-trip as `SkillManifest`:** Expected — `SkillLoader` assembles the full manifest. Do not add `preamble` to frontmatter.
5. **Preamble body trimming:** Loader applies `body.trim()`. Leading/trailing whitespace stripped.
6. **Avoid `---` alone on a line in preamble body** to prevent confusion with frontmatter delimiters.

## Verification

1. Confirm `skills/skill-writer.md` exists and starts with `---`.
2. YAML frontmatter parses into `SkillFrontmatter` with all expected values:
   - `name` == `"skill-writer"`, `version` == `"0.1"`, `description` matches
   - `model.provider` == `"anthropic"`, `model.name` == `"claude-sonnet-4-6"`, `model.temperature` == `0.2`
   - `tools` == `["write_file", "validate_skill"]`
   - `constraints.max_turns` == `10`, `constraints.confidence_threshold` == `0.9`, `constraints.escalate_to` == `None`, `constraints.allowed_actions` == `["read", "write"]`
   - `output.format` == `"structured_json"`, `output.schema` has `skill_yaml` and `validation_result`
3. Markdown body is multi-line with numbered process steps and is non-empty.
4. `SkillLoader::load("skill-writer")` succeeds with `AllToolsExist`.
5. `cargo test` and `cargo clippy` pass.
