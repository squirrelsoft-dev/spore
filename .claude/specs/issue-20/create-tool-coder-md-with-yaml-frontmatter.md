# Spec: Create `skills/tool-coder.md` with YAML frontmatter

> From: .claude/tasks/issue-20.md

## Objective

Create the skill file `skills/tool-coder.md` with valid YAML frontmatter that conforms to the `SkillManifest` schema. This establishes the tool-coder agent's identity, model configuration, tool access, execution constraints, and output schema. The tool-coder is one of the two seed agents (alongside skill-writer) that form the foundation of Spore's self-bootstrapping capability. This task covers only the frontmatter; the preamble body is handled by the subsequent blocking task "Write tool-coder preamble body."

## Current State

Two skill files already exist and serve as reference implementations:

- `skills/skill-writer.md` -- The skill-writer seed agent. Uses `write_file` and `validate_skill` tools, temperature 0.2, max_turns 10, confidence_threshold 0.9, no `escalate_to` field, output schema with `skill_yaml` and `validation_result` keys.
- `skills/orchestrator.md` -- The orchestrator agent. Uses `list_agents` and `route_to_agent` tools, temperature 0.1, max_turns 3, confidence_threshold 0.9, no `escalate_to` field, output schema with `target_agent` and `reasoning` keys.

The `SkillManifest` struct is defined in `crates/agent-sdk/src/skill_manifest.rs` with 8 fields: `name`, `version`, `description`, `model` (`ModelConfig`), `preamble`, `tools` (`Vec<String>`), `constraints` (`Constraints`), and `output` (`OutputSchema`).

The `Constraints` struct (`crates/agent-sdk/src/constraints.rs`) has `escalate_to` as `Option<String>` with `#[serde(default, skip_serializing_if = "Option::is_none")]`, meaning the field can be included or omitted in YAML.

Validation rules in `crates/skill-loader/src/validation.rs` enforce: non-empty required strings, non-empty preamble, confidence_threshold in [0.0, 1.0], max_turns > 0, tools must exist in the tool registry, output format must be one of `json`/`structured_json`/`text`, and escalate_to must not be empty if provided.

## Requirements

1. **Create file `skills/tool-coder.md`** with YAML frontmatter delimited by `---` lines and a minimal placeholder preamble body (to be replaced by the blocking task).

2. **Frontmatter field values must be exactly:**
   - `name: tool-coder`
   - `version: "0.1"` (quoted to prevent YAML float coercion)
   - `description:` A concise one-line summary of the tool-coder agent's purpose (e.g., "Generates, compiles, and validates Rust MCP tool implementations from specifications")
   - `model.provider: anthropic`
   - `model.name: claude-sonnet-4-6`
   - `model.temperature: 0.1`
   - `tools:` list containing `read_file`, `write_file`, `cargo_build`
   - `constraints.max_turns: 15`
   - `constraints.confidence_threshold: 0.85`
   - `constraints.escalate_to: human_reviewer`
   - `constraints.allowed_actions:` list containing `read`, `write`, `execute`
   - `output.format: structured_json`
   - `output.schema:` map with keys `tools_generated` (string), `compilation_result` (string), `implementation_paths` (string)

3. **The preamble (markdown body after closing `---`)** must be non-empty to pass validation. Use a single-line placeholder such as `Placeholder: preamble body to be written in the next task.` This will be fully replaced by the blocking task "Write tool-coder preamble body."

4. **YAML structure must match the existing skill files** in indentation style (2-space indent), field ordering, and list formatting (hyphenated items).

5. **All validation rules must pass** when the file is loaded by `SkillLoader`, with the caveat that tool existence checks depend on the tool registry containing `read_file`, `write_file`, and `cargo_build` (these tools may not exist in the registry yet; validation will pass with `AllToolsExist` stub).

## Implementation Details

### File to create: `skills/tool-coder.md`

The complete file content should be:

```yaml
---
name: tool-coder
version: "0.1"
description: Generates, compiles, and validates Rust MCP tool implementations from specifications
model:
  provider: anthropic
  name: claude-sonnet-4-6
  temperature: 0.1
tools:
  - read_file
  - write_file
  - cargo_build
constraints:
  max_turns: 15
  confidence_threshold: 0.85
  escalate_to: human_reviewer
  allowed_actions:
    - read
    - write
    - execute
output:
  format: structured_json
  schema:
    tools_generated: string
    compilation_result: string
    implementation_paths: string
---
Placeholder: preamble body to be written in the next task.
```

### Key differences from existing skill files

- **`escalate_to` is present.** Neither `skill-writer.md` nor `orchestrator.md` uses `escalate_to`. The tool-coder is the first skill to include it. The value `human_reviewer` must be a non-empty string (validated by `check_escalate_to`).
- **Three tools** instead of two. The `cargo_build` tool implies the agent can trigger compilation, which is a new capability compared to the other seed agents.
- **`allowed_actions` includes `execute`.** The other skills use `read`/`write` or `route`/`discover`. The `execute` action reflects the tool-coder's ability to run build commands.
- **`max_turns: 15`** is higher than the other skills (10 for skill-writer, 3 for orchestrator), reflecting the iterative compile-fix cycle expected during tool implementation.
- **`confidence_threshold: 0.85`** is slightly lower than the 0.9 used by the other skills, allowing the agent more latitude before escalating.

### No code changes required

This task creates only a markdown skill file. No Rust code is added or modified.

## Dependencies

- **Blocked by:** None
- **Blocking:** "Write tool-coder preamble body" -- that task will replace the placeholder preamble with the full behavioral instructions for the tool-coder agent.

## Risks & Edge Cases

1. **YAML float coercion on `version`.** If `version` is written as `0.1` without quotes, YAML parsers will interpret it as a float (0.1), and serde deserialization into `String` will fail. Mitigation: the spec explicitly requires `"0.1"` with quotes, and the existing skill files demonstrate this pattern.

2. **`escalate_to: human_reviewer` references a non-existent agent.** The current `check_escalate_to` validation only verifies the string is non-empty; it does not verify the target agent exists (see the TODO comment in `validation.rs` line 94). This means the file will pass validation today, but a future cross-agent validation pass could flag it. Mitigation: the `human_reviewer` target is intentional as an escape hatch and should be registered when the escalation system is built.

3. **Tools `read_file`, `write_file`, `cargo_build` may not be registered yet.** The tool registry validation (`check_tools_exist`) will fail if these tool names are not in the registry. Mitigation: during development and testing, the `AllToolsExist` stub can be used. The tool names should be registered in the tool registry as part of the tool-coder implementation work.

4. **Placeholder preamble is intentionally minimal.** The preamble satisfies the non-empty validation rule but provides no behavioral guidance. The agent should not be deployed with this placeholder; the blocking task must be completed first.

5. **No standalone `---` lines in preamble.** The placeholder text contains no triple-dash lines, so there is no risk of frontmatter delimiter collision.

## Verification

1. **File exists** at `skills/tool-coder.md` with correct path and filename.
2. **YAML frontmatter parses successfully** -- run `cargo test -p skill-loader` to confirm the frontmatter can be deserialized into `SkillFrontmatter` (requires a test that loads the file, or manual verification with `serde_yaml`).
3. **All required fields are present and non-empty** -- `name`, `version`, `model.provider`, `model.name` are all non-empty strings.
4. **`version` is a string, not a float** -- confirm the YAML value is quoted (`"0.1"`).
5. **`confidence_threshold` is in [0.0, 1.0]** -- 0.85 satisfies this.
6. **`max_turns` is greater than 0** -- 15 satisfies this.
7. **`output.format` is a valid value** -- `structured_json` is in `ALLOWED_OUTPUT_FORMATS`.
8. **`escalate_to` is non-empty** -- `human_reviewer` satisfies this.
9. **Preamble is non-empty** -- the placeholder line satisfies this.
10. **`cargo build` succeeds** -- no Rust code is changed, so existing compilation should be unaffected.
11. **Field ordering and indentation** visually matches the style of `skills/skill-writer.md` and `skills/orchestrator.md`.
