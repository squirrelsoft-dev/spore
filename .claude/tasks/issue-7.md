# Task Breakdown: Create example skill files

> Create three example skill files in `skills/` using markdown-with-frontmatter format to serve as documentation, test fixtures, and schema coverage, then update the README to match.

## Group 1 — Create skill files

_Tasks in this group can be done in parallel._

- [x] **Create `skills/cogs-analyst.md` finance domain skill** `[S]`
      Create the canonical finance domain agent skill file. YAML frontmatter should include: `name: cogs-analyst`, `version: "1.0.0"`, `description: Handles COGS-related finance queries`, model config (`provider: anthropic`, `name: claude-sonnet-4-6`, `temperature: 0.1`), three tools (`get_account_groups`, `execute_sql`, `query_store_lookup`), constraints (`max_turns: 5`, `confidence_threshold: 0.75`, `escalate_to: general-finance-agent`, `allowed_actions: [read, query]`), output (`format: structured_json`, schema with fields `sql: string`, `explanation: string`, `confidence: float`, `source: string`). The markdown body is a multi-line preamble describing the finance analyst role, with markdown headings and guidelines. This exercises every field in `SkillManifest` and `SkillFrontmatter`. The filename must be `cogs-analyst.md` to match the `name` field for `SkillLoader.load("cogs-analyst")`.
      Files: `skills/cogs-analyst.md`
      Non-blocking

- [x] **Create `skills/echo.md` minimal testing skill** `[S]`
      Create a minimal skill for runtime isolation testing. Frontmatter: `name: echo`, `version: "1.0"`, `description: Echoes input back for testing`, model config (`provider: anthropic`, `name: claude-haiku-4-5-20251001`, `temperature: 0.0`), `tools: []`, constraints (`max_turns: 1`, `confidence_threshold: 1.0`, `allowed_actions: []` — omit `escalate_to` since it is `Option<String>` and defaults to `None`), output (`format: text`, `schema: {}`). The markdown body is a single-line preamble: "Echo back the input exactly as received. Do not modify, summarize, or interpret." This exercises: empty tools list, empty schema, empty allowed_actions, omitted optional field (`escalate_to`), boundary values for confidence_threshold (1.0) and max_turns (1).
      Files: `skills/echo.md`
      Non-blocking

- [x] **Create `skills/skill-writer.md` bootstrap seed agent stub** `[S]`
      Create the seed agent skill for the bootstrap milestone. Frontmatter: `name: skill-writer`, `version: "0.1"`, `description: Produces validated skill files from plain-language descriptions`, model config (`provider: anthropic`, `name: claude-sonnet-4-6`, `temperature: 0.2`), tools (`write_file`, `validate_skill` — stubs), constraints (`max_turns: 10`, `confidence_threshold: 0.9`, `allowed_actions: [read, write]` — omit `escalate_to`), output (`format: structured_json`, schema: `skill_yaml: string`, `validation_result: string`). The markdown body is a multi-step preamble with numbered process steps using markdown formatting. Tool names are stubs that will fail tool-existence validation until tool-registry (issue #8) is populated, which is expected.
      Files: `skills/skill-writer.md`
      Non-blocking

## Group 2 — Update documentation and verify

_Depends on: Group 1_

- [x] **Update README.md to show markdown-with-frontmatter format** `[S]`
      Replace the pure YAML skill file example in `README.md` (lines 19-53) with markdown-with-frontmatter format matching `skills/cogs-analyst.md`. Change the fenced code block language from `yaml` to `markdown`. Update line 17 text from "A YAML document declaring everything the agent needs" to reflect the new format. Update the architecture tree comment on line 65 from `# Skill file definitions (YAML)` to `# Skill file definitions (markdown)`. Update Self-Bootstrapping Factory table entries: `skill-writer.yaml` to `skill-writer.md` (line 98), `tool-coder.yaml` to `tool-coder.md` (line 99). Scan for any other `.yaml` skill file references and update.
      Files: `README.md`
      Blocked by: "Create `skills/cogs-analyst.md` finance domain skill"
      Blocking: None

- [x] **Add integration test loading skill files from `skills/` directory** `[S]`
      Add a test (in `crates/skill-loader/tests/`) that loads the actual skill files from the `skills/` directory using the `SkillLoader` with `AllToolsExist` as the tool checker (since tool names are stubs). Verify that `cogs-analyst.md` and `echo.md` load successfully and their parsed `SkillManifest` fields match expected values. The `skill-writer.md` will also load successfully with `AllToolsExist`. This serves as a regression test ensuring the example files stay valid as the schema evolves. Use a relative path from the workspace root or `env!("CARGO_MANIFEST_DIR")` to locate the `skills/` directory.
      Files: `crates/skill-loader/tests/example_skills_test.rs`
      Blocked by: All Group 1 tasks
      Blocking: None

- [x] **Remove `skills/.gitkeep`** `[S]`
      Remove `skills/.gitkeep` since the directory will no longer be empty after adding the three skill files. Leave `tools/.gitkeep` as-is (tools directory is still empty).
      Files: `skills/.gitkeep`
      Blocked by: All Group 1 tasks
      Blocking: None

## Group 3 — Final verification

_Depends on: Group 2_

- [x] **Run full test suite and verify** `[S]`
      Run `cargo test` to ensure all existing tests pass and the new integration test passes. Run `cargo clippy` for lint checks. Verify each skill file loads through the `SkillLoader` without errors (covered by the integration test).
      Blocked by: All previous tasks
      Blocking: None

---

## Notes for implementers

1. **`escalate_to` is `Option<String>`**: This was already changed in issue #6. The echo and skill-writer skills should simply omit `escalate_to` from their frontmatter, and it will default to `None` via `#[serde(default)]`.

2. **Filename must match `name` field**: The `SkillLoader.load(skill_name)` method constructs the path as `{skill_dir}/{skill_name}.md`. So `cogs-analyst.md` must have `name: cogs-analyst` in its frontmatter.

3. **Validation constraints**: All skill files must use output formats from `ALLOWED_OUTPUT_FORMATS: ["json", "structured_json", "text"]`. The `confidence_threshold` must be in `[0.0, 1.0]`, and `max_turns` must be `> 0`. The preamble (markdown body) must not be empty.

4. **Stub tool names**: Tool names like `get_account_groups`, `execute_sql`, `write_file`, `validate_skill` are stubs. They will fail tool-existence validation with a real `ToolExists` implementation. The integration test should use `AllToolsExist` to bypass this check.

5. **Only remove `skills/.gitkeep`**: The `tools/.gitkeep` must remain since the `tools/` directory is still empty.

## Critical files for implementation

- `skills/` — Target directory for the three new `.md` skill files (currently contains only `.gitkeep`)
- `crates/agent-sdk/src/constraints.rs` — Defines `Constraints` struct with `escalate_to: Option<String>`, dictates valid frontmatter fields
- `crates/skill-loader/src/frontmatter.rs` — Defines `SkillFrontmatter` struct that the YAML frontmatter must match, and the extraction logic
- `crates/skill-loader/tests/skill_loader_test.rs` — Pattern to follow for the new integration test (shows how to use `SkillLoader` with `AllToolsExist`)
- `README.md` — Contains the canonical YAML example (lines 19-53) and `.yaml` references that must be updated to markdown-with-frontmatter format
