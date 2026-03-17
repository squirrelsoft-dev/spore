# Spec: Create `skills/cogs-analyst.md` finance domain skill

> From: .claude/tasks/issue-7.md

## Objective

Create the canonical finance domain agent skill file at `skills/cogs-analyst.md` using markdown-with-frontmatter format. This file serves three purposes: (1) a real skill file loadable by `SkillLoader::load("cogs-analyst")`, (2) the reference example that README.md points to as the canonical illustration of the format, and (3) a schema completeness proof that exercises every field in `SkillManifest` and `SkillFrontmatter`, including the optional `escalate_to` field in its `Some` variant. The markdown body (preamble) must be a substantive, multi-line system prompt with markdown formatting to serve as a real-world example of domain-specific agent instruction.

## Current State

- **`SkillManifest`** (`crates/agent-sdk/src/skill_manifest.rs`) defines eight fields: `name`, `version`, `description`, `model` (ModelConfig), `preamble`, `tools` (Vec), `constraints` (Constraints), `output` (OutputSchema).
- **`SkillFrontmatter`** (`crates/skill-loader/src/frontmatter.rs`) mirrors `SkillManifest` but excludes `preamble`. The loader extracts YAML frontmatter into `SkillFrontmatter`, takes the markdown body as the `preamble`, and combines them into a `SkillManifest`.
- **`Constraints`** (`crates/agent-sdk/src/constraints.rs`) has `escalate_to: Option<String>` with `#[serde(default, skip_serializing_if = "Option::is_none")]`. When present in YAML, it deserializes to `Some(value)`. When omitted, it defaults to `None`.
- **`OutputSchema`** (`crates/agent-sdk/src/output_schema.rs`) defines `ALLOWED_OUTPUT_FORMATS: ["json", "structured_json", "text"]`. The `schema` field is `HashMap<String, String>` — values are opaque strings, not validated type names.
- **`SkillLoader`** (`crates/skill-loader/src/lib.rs`) is fully implemented. `load(skill_name)` constructs path as `{skill_dir}/{skill_name}.md`, reads the file, extracts frontmatter via `extract_frontmatter()`, deserializes YAML into `SkillFrontmatter`, trims the body for `preamble`, and runs `validate()`.
- **Validation** (`crates/skill-loader/src/validation.rs`) checks: required strings non-empty, preamble non-empty, confidence_threshold in [0.0, 1.0], max_turns > 0, tools exist (via `ToolExists` trait), output format in `ALLOWED_OUTPUT_FORMATS`, and `escalate_to` not empty when provided.
- **`skills/`** directory exists and contains only `.gitkeep`.
- **README.md** (lines 19-53) contains a pure YAML example for `cogs-analyst` with matching field values.

## Requirements

1. Create file `skills/cogs-analyst.md` in markdown-with-frontmatter format (YAML between `---` delimiters, markdown body after).

2. YAML frontmatter must contain exactly these fields (all `SkillFrontmatter` fields):
   - `name: cogs-analyst`
   - `version: "1.0.0"` (quoted to prevent YAML float/version interpretation)
   - `description: Handles COGS-related finance queries`
   - `model:` block with `provider: anthropic`, `name: claude-sonnet-4-6`, `temperature: 0.1`
   - `tools:` list with exactly `get_account_groups`, `execute_sql`, `query_store_lookup`
   - `constraints:` block with `max_turns: 5`, `confidence_threshold: 0.75`, `escalate_to: general-finance-agent`, `allowed_actions: [read, query]`
   - `output:` block with `format: structured_json`, `schema:` containing `sql: string`, `explanation: string`, `confidence: float`, `source: string`

3. The markdown body (preamble) must be multi-line, contain markdown headings and guidelines, describe the COGS finance analyst role, and include the rule "Never speculate. If confidence is below threshold, escalate."

4. The file must parse successfully through `SkillLoader::load("cogs-analyst")` when using `AllToolsExist`.

5. The `preamble` extracted from the markdown body must not be empty.

6. The output format `structured_json` must be one of the allowed formats in `ALLOWED_OUTPUT_FORMATS`.

## Implementation Details

### File to create

**`skills/cogs-analyst.md`**

The file has two sections separated by frontmatter delimiters:

**Section 1: YAML frontmatter** — The file must start with `---` on line 1. The YAML block includes all fields from `SkillFrontmatter`. Field ordering should follow the struct definition order: `name`, `version`, `description`, `model`, `tools`, `constraints`, `output`. The closing `---` must be on its own line.

Key formatting decisions:
- `version` must be quoted as `"1.0.0"` to ensure String deserialization.
- `temperature: 0.1` deserializes as `f64`.
- `confidence_threshold: 0.75` deserializes as `f64`.
- `escalate_to: general-finance-agent` deserializes to `Some("general-finance-agent".to_string())`.
- `schema` under `output` maps string keys to string values. The value `float` is an opaque string stored as `"float"` in `HashMap<String, String>`.
- The `preamble` field must NOT appear in the frontmatter. It comes from the markdown body.

**Section 2: Markdown body (preamble)** — Everything after the closing `---` delimiter, trimmed by the loader. Suggested structure:

1. Opening paragraph establishing the agent identity as a finance analyst specializing in Cost of Goods Sold.
2. `## Guidelines` heading with behavioral rules as bullet points (never speculate, escalate policy, cite sources, confidence requirements).
3. `## Tool Usage` heading describing when to use each of the three tools.
4. `## Output Format` heading reminding the agent to produce structured JSON with the four schema fields.

### Integration points

- **`SkillLoader::load("cogs-analyst")`** will load this file, parse frontmatter, set preamble from body, and validate.
- **README.md update** (Group 2 task) will reference or mirror this file's content.
- **Integration test** (Group 2 task) will load this file with `AllToolsExist` and assert field values.

## Dependencies

- **Blocked by:** Nothing. Standalone file creation.
- **Blocking:** "Update README.md to show markdown-with-frontmatter format" — the README update must match this file's content.

## Risks & Edge Cases

1. **YAML version string interpretation:** `version: 1.0.0` without quotes may be rejected or misinterpreted. Must be quoted as `"1.0.0"`.

2. **Frontmatter delimiter positioning:** The `extract_frontmatter()` function trims leading whitespace and BOM before checking for the opening delimiter, but the file should start cleanly with `---`.

3. **Preamble whitespace handling:** The loader calls `body.trim().to_string()`. Leading/trailing blank lines in the body will be stripped.

4. **Schema value `float` vs `number`:** The task description specifies `confidence: float`. Since `OutputSchema.schema` is `HashMap<String, String>`, the value `"float"` is stored as-is.

5. **Horizontal rules in preamble:** Avoid `---` horizontal rules in the preamble body to prevent confusion; use `***` or `___` instead if needed.

6. **Tool names are stubs:** `get_account_groups`, `execute_sql`, `query_store_lookup` do not exist in any registry. Integration tests must use `AllToolsExist`.

## Verification

1. Confirm `skills/cogs-analyst.md` exists and is non-empty.
2. Confirm the file starts with `---` on line 1 and has a second `---` delimiter closing the frontmatter.
3. Run `cargo test` to confirm no existing tests are broken.
4. Run `cargo clippy` to confirm no lint warnings.
5. Verify frontmatter YAML deserializes correctly into `SkillFrontmatter` with all expected field values:
   - `name` == `"cogs-analyst"`, `version` == `"1.0.0"`, `description` == `"Handles COGS-related finance queries"`
   - `model.provider` == `"anthropic"`, `model.name` == `"claude-sonnet-4-6"`, `model.temperature` == `0.1`
   - `tools` == `["get_account_groups", "execute_sql", "query_store_lookup"]`
   - `constraints.max_turns` == `5`, `constraints.confidence_threshold` == `0.75`, `constraints.escalate_to` == `Some("general-finance-agent")`, `constraints.allowed_actions` == `["read", "query"]`
   - `output.format` == `"structured_json"`, `output.schema` has keys `sql`/`explanation`/`confidence`/`source`
6. Confirm the markdown body contains markdown headings and the behavioral rule about escalating below confidence threshold.
7. Confirm the full file loads via `SkillLoader::load("cogs-analyst")` with `AllToolsExist`.
