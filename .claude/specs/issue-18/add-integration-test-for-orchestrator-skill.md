# Spec: Add integration test for orchestrator skill
> From: .claude/tasks/issue-18.md

## Objective
Add an integration test named `load_orchestrator_skill` to the existing test file `crates/skill-loader/tests/example_skills_test.rs` that verifies the `skills/orchestrator.md` skill file loads correctly and all parsed `SkillManifest` fields match expected values.

## Current State
The test file already contains three integration tests following a consistent pattern:
- `load_echo_skill` -- tests `skills/echo.md`
- `load_cogs_analyst_skill` -- tests `skills/cogs-analyst.md`
- `load_skill_writer_skill` -- tests `skills/skill-writer.md`

Each test uses shared helpers `skills_dir()` (resolves `../../skills` from `CARGO_MANIFEST_DIR`) and `make_loader()` (creates a `SkillLoader` with `AllToolsExist` tool checker). There is no test yet for the orchestrator skill.

## Requirements
1. Add a new `#[tokio::test]` async function named `load_orchestrator_skill` at the end of the file.
2. Use the existing `skills_dir()` and `make_loader()` helpers -- do not duplicate them.
3. Load the skill via `loader.load("orchestrator").await.unwrap()`.
4. Assert every field of the returned `SkillManifest`:
   - `name` == `"orchestrator"`
   - `version` == `"1.0"`
   - `description` == `"Routes incoming requests to the best-matching specialized agent based on intent analysis"`
   - `model.provider` == `"anthropic"`
   - `model.name` == `"claude-sonnet-4-6"`
   - `model.temperature` ~= `0.1` (use `(value - 0.1).abs() < f64::EPSILON` pattern)
   - `tools` == `["list_agents", "route_to_agent"]`
   - `constraints.max_turns` == `3`
   - `constraints.confidence_threshold` ~= `0.9` (same float comparison pattern)
   - `constraints.escalate_to` == `None`
   - `constraints.allowed_actions` == `["route", "discover"]`
   - `output.format` == `"structured_json"`
   - `output.schema` has exactly 2 entries
   - `output.schema["target_agent"]` == `"string"`
   - `output.schema["reasoning"]` == `"string"`
   - `preamble` is not empty (`assert!(!manifest.preamble.is_empty())`)
   - `preamble` contains a routing-related keyword (e.g., `assert!(manifest.preamble.contains("route") || manifest.preamble.contains("router"))`)

## Implementation Details
- The test follows the exact same structure as `load_cogs_analyst_skill` (the most comprehensive existing test). Copy that pattern and adjust field values.
- `AllToolsExist` is already imported and used in `make_loader()`, so the stub tool names `list_agents` and `route_to_agent` will pass validation without needing to be registered in the tool registry.
- Float comparisons must use the `(value - expected).abs() < f64::EPSILON` idiom, consistent with the existing tests.
- The test file currently has no `load_orchestrator_skill` function, so appending to the end of the file is the cleanest approach.
- No new imports are needed -- everything required (`SkillLoader`, `AllToolsExist`, `ToolRegistry`, `Arc`) is already imported.

## Dependencies
- **Blocked by**: "Create `skills/orchestrator.md` routing skill file" -- the `orchestrator.md` file must exist before this test can pass. The test directly loads and parses that file.
- **No downstream blockers**: This task is non-blocking.

## Risks & Edge Cases
- **Skill file not yet created**: This test will fail at runtime if `skills/orchestrator.md` does not exist. This is expected since this task is blocked by skill file creation.
- **Preamble content sensitivity**: The preamble assertion (`contains("route")`) should use a word likely to appear in any reasonable routing preamble. The task description for the skill file specifies routing-only instructions, so "route" is a safe keyword.
- **Field value mismatch**: If the skill file is created with different values than specified in the task breakdown (e.g., different description text), the test will fail. The spec values here are taken directly from the task breakdown in `.claude/tasks/issue-18.md` and must stay in sync.

## Verification
1. `cargo check -p skill-loader --tests` -- confirms the test compiles.
2. `cargo test -p skill-loader --test example_skills_test load_orchestrator_skill` -- confirms the test passes (requires `skills/orchestrator.md` to exist).
3. `cargo test -p skill-loader --test example_skills_test` -- confirms all existing tests still pass alongside the new one.
4. `cargo clippy -p skill-loader --tests` -- confirms no lint warnings in the test file.
