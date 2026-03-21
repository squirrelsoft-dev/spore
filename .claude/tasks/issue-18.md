# Task Breakdown: Create orchestrator skill file

> Create `skills/orchestrator.md` -- the orchestrator's skill file that configures it as a pure routing agent with no domain knowledge, proving the system's homogeneity principle.

## Group 1 — Create skill file

_Tasks in this group can be done in parallel._

- [x] **Create `skills/orchestrator.md` routing skill file** `[S]`
      Create the orchestrator skill file using the established markdown-with-frontmatter format (same as `echo.md`, `cogs-analyst.md`, `skill-writer.md`). The YAML frontmatter must deserialize into `SkillFrontmatter` (which maps to `SkillManifest`). Frontmatter fields: `name: orchestrator`, `version: "1.0"`, `description: Routes incoming requests to the best-matching specialized agent based on intent analysis`, model config (`provider: anthropic`, `name: claude-sonnet-4-6`, `temperature: 0.1`), `tools: [list_agents, route_to_agent]`, constraints (`max_turns: 3`, `confidence_threshold: 0.9`, omit `escalate_to` so it defaults to `None` -- this is the top-level router with no escalation target), `allowed_actions: [route, discover]`, output (`format: structured_json`, schema: `target_agent: string`, `reasoning: string`). The markdown body (preamble) must be concise routing-only instructions: define the agent as a request router, specify rules (never answer domain questions directly, analyze intent, match against available agents, report no-match if confidence insufficient, never speculate). Keep the preamble under 20 lines, focused purely on routing behavior with no domain knowledge. Key validation constraints: `version` must be quoted (`"1.0"`), `confidence_threshold` in [0.0, 1.0], `max_turns > 0`, `format` must be one of `["json", "structured_json", "text"]`, preamble must not be empty.
      Files: `skills/orchestrator.md`
      Blocking: "Add integration test for orchestrator skill", "Update orchestrator to load skill file instead of hardcoded manifest"

## Group 2 — Integration test and orchestrator wiring

_Depends on: Group 1._

- [x] **Add integration test for orchestrator skill** `[S]`
      Add a test case in `crates/skill-loader/tests/example_skills_test.rs` following the exact pattern of `load_echo_skill` and `load_cogs_analyst_skill`. The test function should be named `load_orchestrator_skill`. It should use `skills_dir()` and `make_loader()` helpers already defined in that file. Assert all parsed `SkillManifest` fields match expected values: `name == "orchestrator"`, `version == "1.0"`, `description` matches, `model.provider == "anthropic"`, `model.name == "claude-sonnet-4-6"`, `temperature ~= 0.1`, `tools == ["list_agents", "route_to_agent"]`, `max_turns == 3`, `confidence_threshold ~= 0.9`, `escalate_to == None`, `allowed_actions == ["route", "discover"]`, `output.format == "structured_json"`, `output.schema` has keys `target_agent` and `reasoning` both with value `"string"`. Verify preamble is non-empty and contains key phrases like "router" or "route". Uses `AllToolsExist` since `list_agents` and `route_to_agent` are stub tool names.
      Files: `crates/skill-loader/tests/example_skills_test.rs`
      Blocked by: "Create `skills/orchestrator.md` routing skill file"
      Non-blocking

- [x] **Update orchestrator to load skill file instead of hardcoded manifest** `[M]`
      Replace the `build_default_manifest()` function in `crates/orchestrator/src/orchestrator.rs` which hardcodes a placeholder `SkillManifest` with empty preamble, `provider: "none"`, and zero tools. Instead, update `from_config` and `from_config_with_model` to accept a `SkillManifest` loaded from the skill file (either passed in as a parameter, or by accepting the skills directory path and loading `orchestrator.md` via `SkillLoader`). This demonstrates the key design principle: the orchestrator is just runtime + skill file, identical to every other agent. The `build_default_manifest` function can be kept as a fallback or removed entirely. Consider adding a `SkillManifest` parameter to `OrchestratorConfig` or to the constructor methods. Update any tests in `crates/orchestrator/` that rely on `build_default_manifest` to use the real skill file or a test fixture.
      Files: `crates/orchestrator/src/orchestrator.rs`, `crates/orchestrator/src/config.rs`
      Blocked by: "Create `skills/orchestrator.md` routing skill file"
      Non-blocking

## Group 3 — Verification

_Depends on: Group 2._

- [x] **Run verification suite** `[S]`
      Run `cargo check`, `cargo clippy`, and `cargo test` across the full workspace. Verify all existing tests pass including the new orchestrator skill integration test. Verify that `skills/orchestrator.md` loads through `SkillLoader` without validation errors.
      Files: (none — command-line verification only)
      Blocked by: All other tasks

## Implementation Notes

1. **`escalate_to` must be omitted, not set to `""`**: The validation rejects empty strings. Since the orchestrator is the top-level router with no escalation target, omit the `escalate_to` field from the frontmatter. The `#[serde(default)]` attribute on `Constraints.escalate_to` will default it to `None`.

2. **Tool names are stubs**: `list_agents` and `route_to_agent` will not resolve in the tool-registry until issues #8-10 are complete. The integration test must use `AllToolsExist` to bypass tool-existence validation.

3. **Filename must match `name` field**: `SkillLoader.load("orchestrator")` constructs path as `{skill_dir}/orchestrator.md`, so the file must be named `orchestrator.md` and have `name: orchestrator` in its frontmatter.

4. **Preamble must reference the structured output format**: The preamble should mention that routing decisions are returned as structured JSON with `target_agent` and `reasoning` fields.
