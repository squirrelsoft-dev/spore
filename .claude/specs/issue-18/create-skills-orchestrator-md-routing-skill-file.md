# Spec: Create `skills/orchestrator.md` routing skill file
> From: .claude/tasks/issue-18.md

## Objective

Create a new skill file at `skills/orchestrator.md` that defines the orchestrator agent's routing behavior. The file uses the established markdown-with-frontmatter format (consistent with `echo.md`, `cogs-analyst.md`, `skill-writer.md`) and its YAML frontmatter must deserialize cleanly into `SkillFrontmatter` (which maps to `SkillManifest`). The orchestrator's sole purpose is to analyze incoming requests and route them to the best-matching specialized agent.

## Current State

Three skill files exist under `skills/`:
- `echo.md` -- minimal test skill, no tools, `format: text`
- `cogs-analyst.md` -- domain skill with tools and `escalate_to`
- `skill-writer.md` -- seed factory skill, `format: structured_json`

No `orchestrator.md` file exists yet. The orchestrator crate (`crates/orchestrator/`) currently uses a hardcoded manifest rather than loading from a skill file.

## Requirements

### Frontmatter fields (YAML between `---` delimiters)

| Field | Value |
|---|---|
| `name` | `orchestrator` |
| `version` | `"1.0"` (must be a quoted string) |
| `description` | `Routes incoming requests to the best-matching specialized agent based on intent analysis` |
| `model.provider` | `anthropic` |
| `model.name` | `claude-sonnet-4-6` |
| `model.temperature` | `0.1` |
| `tools` | `[list_agents, route_to_agent]` |
| `constraints.max_turns` | `3` |
| `constraints.confidence_threshold` | `0.9` |
| `constraints.escalate_to` | omitted (defaults to `None` via `#[serde(default)]`) |
| `constraints.allowed_actions` | `[route, discover]` |
| `output.format` | `structured_json` |
| `output.schema` | `target_agent: string`, `reasoning: string` |

### Markdown body (preamble)

- Must not be empty (validation rule in `check_preamble`)
- Must be under 20 lines
- Must contain concise routing-only instructions (no domain logic, no tool usage guidance beyond routing)

### Validation constraints the file must satisfy

These are enforced by `crates/skill-loader/src/validation.rs`:

1. `name`, `version`, `model.provider`, `model.name` must be non-empty strings
2. `preamble` must not be empty (body after closing `---`)
3. `confidence_threshold` must be in `[0.0, 1.0]` -- value `0.9` satisfies this
4. `max_turns` must be `> 0` -- value `3` satisfies this
5. `output.format` must be one of `["json", "structured_json", "text"]` -- value `structured_json` satisfies this
6. `escalate_to` must not be an empty string when provided -- omitted entirely, so `None`, which passes

## Implementation Details

Create a single file `skills/orchestrator.md` with the following structure:

```
---
<YAML frontmatter with all fields from the Requirements table>
---
<Markdown preamble: concise routing instructions, under 20 lines>
```

The preamble should instruct the orchestrator agent to:
- Analyze the user's request to determine intent
- Use `list_agents` to discover available agents and their capabilities
- Select the best-matching agent based on intent analysis
- Use `route_to_agent` to forward the request
- Return structured JSON with `target_agent` (the selected agent name) and `reasoning` (brief explanation of why that agent was chosen)
- If no agent matches with sufficient confidence, indicate this in the response rather than guessing

The preamble must be focused exclusively on routing logic. It must not contain domain-specific instructions for any downstream agent.

### Format conventions (match existing files)

- Two-space indentation for nested YAML
- List items use `- item` format for tools
- Schema fields use `key: type` format (e.g., `target_agent: string`)
- No trailing blank lines after the preamble
- File ends with a newline character

## Dependencies

- `crates/agent-sdk/src/skill_manifest.rs` -- `SkillManifest` struct
- `crates/agent-sdk/src/constraints.rs` -- `Constraints` struct (defines `escalate_to` as `Option<String>` with `#[serde(default)]`)
- `crates/agent-sdk/src/model_config.rs` -- `ModelConfig` struct
- `crates/agent-sdk/src/output_schema.rs` -- `OutputSchema` struct, `ALLOWED_OUTPUT_FORMATS`
- `crates/skill-loader/src/frontmatter.rs` -- `SkillFrontmatter` deserialization, `extract_frontmatter` parser
- `crates/skill-loader/src/validation.rs` -- `validate()` function with all check functions

## Risks & Edge Cases

1. **YAML version quoting**: `version: 1.0` without quotes would be parsed as a float by YAML, not a string. Must be `version: "1.0"` to satisfy `SkillFrontmatter.version: String` deserialization.
2. **Empty preamble**: If the body after the closing `---` is blank or whitespace-only, `check_preamble` will reject it. The preamble must contain substantive routing instructions.
3. **Tool name validity**: `list_agents` and `route_to_agent` must be recognized by the tool registry at runtime. During file-level validation with `AllToolsExist` stub this passes, but integration tests that use a real registry will need these tools registered.
4. **Temperature value**: `0.1` is valid as `f64`. Low temperature is appropriate for deterministic routing decisions.
5. **Schema field types**: The `output.schema` is `HashMap<String, String>`, so values like `string` are just descriptive type labels (not enforced types). This matches the convention in `cogs-analyst.md` and `skill-writer.md`.

## Verification

1. **Parse test**: Run the skill loader against `skills/orchestrator.md` and confirm `extract_frontmatter` returns valid YAML and a non-empty body.
2. **Deserialization test**: Confirm the YAML deserializes into `SkillFrontmatter` without errors.
3. **Validation test**: Confirm `validate()` passes with all checks (required strings, preamble, confidence threshold, max turns, output format, escalate_to).
4. **Field value assertions**: Verify each field matches the specified values:
   - `name == "orchestrator"`
   - `version == "1.0"`
   - `model.provider == "anthropic"`
   - `model.name == "claude-sonnet-4-6"`
   - `model.temperature == 0.1`
   - `tools == ["list_agents", "route_to_agent"]`
   - `constraints.max_turns == 3`
   - `constraints.confidence_threshold == 0.9`
   - `constraints.escalate_to == None`
   - `constraints.allowed_actions == ["route", "discover"]`
   - `output.format == "structured_json"`
   - `output.schema` contains keys `target_agent` and `reasoning` with value `"string"`
5. **Preamble line count**: Confirm body is under 20 lines and non-empty.
6. **Cargo test**: Run `cargo test` to ensure no regressions.

### Blocking

This spec blocks:
- "Add integration test for orchestrator skill"
- "Update orchestrator to load skill file instead of hardcoded manifest"
