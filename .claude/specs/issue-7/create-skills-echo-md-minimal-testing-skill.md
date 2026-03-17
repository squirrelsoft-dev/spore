# Spec: Create `skills/echo.md` minimal testing skill

> From: .claude/tasks/issue-7.md

## Objective

Create a minimal skill file at `skills/echo.md` that exercises the boundary conditions and edge cases of the `SkillManifest` schema. Specifically, this file tests: an empty tools list, an empty output schema map, an empty `allowed_actions` list, omission of the optional `escalate_to` field (which defaults to `None`), the boundary value `1.0` for `confidence_threshold`, and the minimum valid `max_turns` value of `1`. The file serves as both a test fixture for runtime isolation testing and as documentation of the minimal valid skill file format.

## Current State

- The `skills/` directory exists but contains only `.gitkeep`.
- `SkillManifest` defines eight fields: `name`, `version`, `description`, `model` (ModelConfig), `preamble`, `tools` (Vec), `constraints` (Constraints), `output` (OutputSchema).
- `Constraints` defines `escalate_to` as `Option<String>` with `#[serde(default, skip_serializing_if = "Option::is_none")]` (changed in issue #6). Omitting `escalate_to` from YAML correctly defaults it to `None`.
- `OutputSchema` defines `schema` as `HashMap<String, String>`, which accepts an empty map (`{}`).
- `ALLOWED_OUTPUT_FORMATS` is `["json", "structured_json", "text"]`. The `text` format is used here.
- The `SkillLoader::load()` method constructs the path as `{skill_dir}/{skill_name}.md`, so `echo.md` must match `name: echo`.
- `SkillFrontmatter` mirrors `SkillManifest` but excludes `preamble` — preamble comes from the markdown body.
- Validation checks: `confidence_threshold` in `[0.0, 1.0]`, `max_turns > 0`, `escalate_to` when `Some` must not be empty string.
- Existing tests already cover `deserialize_empty_tools_list`, `deserialize_empty_schema_map`, and `deserialize_missing_escalate_to` as isolated fragments, but no real skill file exercises all minimal/boundary values simultaneously.

## Requirements

1. **File location:** `skills/echo.md`

2. **YAML frontmatter fields** (all fields from `SkillFrontmatter`):

   | Field | Value | Rationale |
   |---|---|---|
   | `name` | `echo` | Must match filename |
   | `version` | `"1.0"` (quoted) | Prevents YAML float coercion |
   | `description` | `Echoes input back for testing` | From task description |
   | `model.provider` | `anthropic` | |
   | `model.name` | `claude-haiku-4-5-20251001` | |
   | `model.temperature` | `0.0` | Deterministic output |
   | `tools` | `[]` (empty) | Tests empty Vec |
   | `constraints.max_turns` | `1` | Boundary: minimum valid (> 0) |
   | `constraints.confidence_threshold` | `1.0` | Boundary: maximum valid |
   | `constraints.escalate_to` | **omitted entirely** | Tests Option defaulting to None |
   | `constraints.allowed_actions` | `[]` (empty) | Tests empty Vec |
   | `output.format` | `text` | Valid allowed format |
   | `output.schema` | `{}` (empty) | Tests empty HashMap |

3. **Markdown body (preamble):** A single line:
   `Echo back the input exactly as received. Do not modify, summarize, or interpret.`

4. **`escalate_to` must be omitted, not set to empty string.** Setting to `""` would result in `Some("")`, which fails validation.

5. **`version` must be quoted as `"1.0"`** to prevent YAML float interpretation.

## Implementation Details

### File to create

**`skills/echo.md`** — The complete file content:

```
---
name: echo
version: "1.0"
description: Echoes input back for testing
model:
  provider: anthropic
  name: claude-haiku-4-5-20251001
  temperature: 0.0
tools: []
constraints:
  max_turns: 1
  confidence_threshold: 1.0
  allowed_actions: []
output:
  format: text
  schema: {}
---
Echo back the input exactly as received. Do not modify, summarize, or interpret.
```

### Key design decisions

- **No `preamble` in frontmatter:** Populated from the markdown body by `SkillLoader::load()`.
- **`escalate_to` omitted, not empty:** Omitting defaults to `None`. `Some("")` would fail validation.
- **`confidence_threshold: 1.0`:** Upper boundary of `[0.0, 1.0]` range (inclusive via `(0.0..=1.0).contains(&value)`).
- **`max_turns: 1`:** Minimum valid value (`max_turns > 0`).

### Validation pass-through

All validation checks pass:
- `check_required_strings`: all non-empty
- `check_preamble`: single non-empty line
- `check_confidence_threshold`: `1.0` in `[0.0, 1.0]`
- `check_max_turns`: `1 > 0`
- `check_tools_exist`: empty list, nothing to check
- `check_output_format`: `"text"` in allowed formats
- `check_escalate_to`: `None` — branch not entered

## Dependencies

- **Blocked by:** None (non-blocking)
- **Blocking:** None directly. Group 2 integration test and `.gitkeep` removal depend on all Group 1 files.

## Risks & Edge Cases

1. **YAML float coercion for `version`:** `version: 1.0` without quotes interpreted as float. Must quote as `"1.0"`.
2. **`temperature: 0.0` vs `0`:** Both valid for f64. Use `0.0` for clarity.
3. **Trailing newline in preamble:** Loader trims body via `body.trim().to_string()`. Handled correctly.
4. **Frontmatter alone won't deserialize to `SkillManifest`:** Must go through `SkillFrontmatter` + body assembly.
5. **Empty `allowed_actions`:** Currently valid. Future runtime may interpret as "no actions permitted" — acceptable for echo.

## Verification

1. File exists at `skills/echo.md` with exact content specified above.
2. YAML frontmatter parses as valid YAML.
3. All field values match requirements table.
4. Markdown body is exactly: `Echo back the input exactly as received. Do not modify, summarize, or interpret.`
5. `SkillLoader::load("echo")` succeeds with `AllToolsExist`, producing `SkillManifest` with `escalate_to == None`.
6. All validation checks pass without `ValidationError`.
7. `cargo test` and `cargo clippy` pass.
