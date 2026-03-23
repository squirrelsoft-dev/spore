# Spec: Add integration test for deploy-agent skill

> From: .claude/tasks/issue-21.md

## Objective

Add a `load_deploy_agent_skill` integration test to `crates/skill-loader/tests/example_skills_test.rs` that validates the `deploy-agent` skill file loads correctly and all frontmatter fields and preamble keywords match expected values. This ensures the deploy-agent skill file is well-formed and parseable by the skill-loader before it reaches production.

## Current State

The file `crates/skill-loader/tests/example_skills_test.rs` contains five existing integration tests that all follow an identical pattern:

1. Call `skills_dir()` to get the path to the `skills/` directory.
2. Call `make_loader(&dir)` to create a `SkillLoader` with a `ToolRegistry` and `AllToolsExist` validator.
3. Call `loader.load("<skill-name>").await.unwrap()` to parse the skill markdown file.
4. Assert every field on the returned `SkillManifest`:
   - `name`, `version`, `description` (exact string equality)
   - `model.provider`, `model.name`, `model.temperature` (float comparison with epsilon)
   - `tools` (exact `Vec<String>` equality)
   - `constraints.max_turns`, `constraints.confidence_threshold` (float with epsilon), `constraints.escalate_to` (`Option<String>`), `constraints.allowed_actions`
   - `output.format`, `output.schema` (check `.len()` then individual keys)
   - `preamble` (non-empty check, then keyword `contains` assertions)

Existing tests: `load_cogs_analyst_skill`, `load_echo_skill`, `load_skill_writer_skill`, `load_orchestrator_skill`, `load_tool_coder_skill`.

## Requirements

- Add a new `#[tokio::test]` async function named `load_deploy_agent_skill`.
- Use the same `skills_dir()` and `make_loader()` helpers.
- Load the skill via `loader.load("deploy-agent").await.unwrap()`.
- Assert **all** `SkillManifest` fields with exact expected values (these values will be defined by the blocked-by task "Create `skills/deploy-agent.md` with YAML frontmatter"):
  - `manifest.name` == `"deploy-agent"`
  - `manifest.version` (exact value from frontmatter)
  - `manifest.description` (exact value from frontmatter)
  - `manifest.model.provider` (exact value)
  - `manifest.model.name` (exact value)
  - `manifest.model.temperature` (float comparison using `(val - expected).abs() < f64::EPSILON`)
  - `manifest.tools` (exact vec of tool name strings)
  - `manifest.constraints.max_turns` (exact integer)
  - `manifest.constraints.confidence_threshold` (float with epsilon)
  - `manifest.constraints.escalate_to` (exact `Option<String>`)
  - `manifest.constraints.allowed_actions` (exact vec of strings)
  - `manifest.output.format` (exact string)
  - `manifest.output.schema` -- assert `.len()` then each key-value pair
- Assert `manifest.preamble` is non-empty.
- Assert preamble contains the following keyword groups (using `contains` with `||` alternatives and descriptive failure messages):
  - `"Docker"` or `"docker"` or `"container"` -- preamble should reference containerization
  - `"registry"` or `"push"` -- preamble should reference pushing to a registry
  - `"orchestrator"` or `"register"` -- preamble should reference registering with the orchestrator
  - `"health"` or `"verify"` -- preamble should reference health checks or verification
  - `"scratch"` or `"minimal"` -- preamble should reference minimal/scratch-based images

## Implementation Details

### Test function signature and structure

```
#[tokio::test]
async fn load_deploy_agent_skill() {
    let dir = skills_dir();
    let loader = make_loader(&dir);
    let manifest = loader.load("deploy-agent").await.unwrap();

    // 1. Identity fields
    // 2. Model config fields
    // 3. Tools list
    // 4. Constraints fields
    // 5. Output fields
    // 6. Preamble non-empty + keyword checks
}
```

### Field assertions

Exact values for `version`, `description`, `model.*`, `tools`, `constraints.*`, `output.*` depend on the frontmatter defined in the blocked-by task. The implementer must read `skills/deploy-agent.md` at implementation time to extract these values.

### Preamble keyword checks

Each keyword check should follow the pattern used in `load_tool_coder_skill` and `load_skill_writer_skill`: an `assert!` with a `contains` call (or `||` of multiple `contains` calls) and a descriptive trailing string message. For example:

```
assert!(
    manifest.preamble.contains("Docker") || manifest.preamble.contains("docker") || manifest.preamble.contains("container"),
    "preamble should reference Docker or containerization"
);
```

Five such assertions are required, one per keyword group listed in Requirements.

## Dependencies

- **Blocked by:** "Create `skills/deploy-agent.md` with YAML frontmatter", "Write deploy-agent preamble body" -- the skill file must exist with finalized frontmatter values before exact assertions can be written.
- **Blocking:** "Run verification suite" -- the test must be in place before the full verification pass.

## Risks & Edge Cases

- If the `deploy-agent.md` frontmatter values change after this test is written, the test will fail. The implementer should verify the test against the actual file at implementation time.
- The skill name `"deploy-agent"` contains a hyphen; this is consistent with other skills (e.g., `"cogs-analyst"`, `"skill-writer"`, `"tool-coder"`) so no special handling is needed.
- The `AllToolsExist` validator is used in tests, meaning tool names in the frontmatter do not need to actually exist in the `ToolRegistry`. No additional setup is required.
- Preamble keyword checks use case-insensitive alternatives (e.g., `"Docker"` or `"docker"`) to avoid brittleness from capitalization changes.

## Verification

- `cargo test --test example_skills_test load_deploy_agent_skill` passes.
- All five preamble keyword assertions are present in the test.
- All `SkillManifest` fields are asserted (no field left unchecked).
- The test follows the exact structural pattern of the existing tests -- no new helpers, no new imports, no deviations.
