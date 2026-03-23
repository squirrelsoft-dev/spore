# Spec: Create `skills/deploy-agent.md` with YAML frontmatter

> From: .claude/tasks/issue-21.md

## Objective

Create the skill file for the deploy-agent, which packages a runtime binary and skill file into a minimal Docker image, pushes it to a container registry, and registers the agent with the orchestrator. This file establishes the deploy-agent's identity, model configuration, tool requirements, execution constraints, and output contract via YAML frontmatter conforming to the SkillManifest schema.

## Current State

- Two seed skill files already exist: `skills/skill-writer.md` and `skills/tool-coder.md`. Both follow the markdown-with-YAML-frontmatter format parsed by the skill-loader crate.
- The `SkillManifest` struct in `crates/agent-sdk/src/skill_manifest.rs` defines 8 fields: `name`, `version`, `description`, `model` (ModelConfig), `preamble` (from the markdown body), `tools` (Vec<String>), `constraints` (Constraints), and `output` (OutputSchema).
- Validation in `crates/skill-loader/src/validation.rs` enforces: non-empty required strings, preamble non-empty, confidence_threshold in [0.0, 1.0], max_turns > 0, tool existence in registry, output format in {"json", "structured_json", "text"}, and escalate_to non-empty when present.
- The `skills/` directory is the canonical location for skill files. No `deploy-agent.md` exists yet.

## Requirements

1. Create `skills/deploy-agent.md` with YAML frontmatter delimited by `---` lines.
2. `name` must be `deploy-agent`.
3. `version` must be `"0.1"` (quoted to prevent YAML float coercion).
4. `description` must summarize the agent's purpose: packages a runtime binary and skill file into a minimal Docker image, pushes to a registry, and registers the agent with the orchestrator.
5. `model` must contain `provider: anthropic`, `name: claude-sonnet-4-6`, `temperature: 0.1`.
6. `tools` must list exactly: `docker_build`, `docker_push`, `register_agent`.
7. `constraints` must contain `max_turns: 10`, `confidence_threshold: 0.9`, `escalate_to: human_reviewer`, and `allowed_actions: [read, execute, deploy]`.
8. `output` must contain `format: structured_json` and `schema` with keys `image_uri: string`, `endpoint_url: string`, `health_check: string`.
9. The markdown body (preamble) must be left as a placeholder for the next task ("Write deploy-agent preamble body"). It must be non-empty to pass validation -- use a single-line placeholder such as `TODO: preamble body to be written in the next task.`
10. The file must pass all validation rules defined in `crates/skill-loader/src/validation.rs`, except for tool existence (the tools `docker_build`, `docker_push`, `register_agent` do not yet exist in the registry).
11. No standalone `---` lines in the preamble (validation rule 9 from skill-writer).

## Implementation Details

### File to create: `skills/deploy-agent.md`

Exact content:

```yaml
---
name: deploy-agent
version: "0.1"
description: Packages a runtime binary and skill file into a minimal Docker image, pushes to a registry, and registers the agent with the orchestrator
model:
  provider: anthropic
  name: claude-sonnet-4-6
  temperature: 0.1
tools:
  - docker_build
  - docker_push
  - register_agent
constraints:
  max_turns: 10
  confidence_threshold: 0.9
  escalate_to: human_reviewer
  allowed_actions:
    - read
    - execute
    - deploy
output:
  format: structured_json
  schema:
    image_uri: string
    endpoint_url: string
    health_check: string
---
TODO: preamble body to be written in the next task.
```

### Design decisions

- **Temperature 0.1**: Matches `tool-coder.md` and reflects the deterministic nature of deployment operations where precision matters more than creativity.
- **max_turns 10**: Matches `skill-writer.md`. Deployment involves a bounded sequence of steps (build, push, register, health-check), so 10 turns provides adequate headroom without being wasteful.
- **confidence_threshold 0.9**: Matches `skill-writer.md`. Deployment actions are consequential (pushing images, registering endpoints) and warrant high confidence before proceeding.
- **escalate_to human_reviewer**: Consistent with both existing skill files. Deployment failures or ambiguous situations should involve a human.
- **allowed_actions [read, execute, deploy]**: `read` for inspecting Dockerfiles and configs, `execute` for running docker build/push commands, `deploy` for the registration step. Notably does not include `write` since the deploy-agent should not be modifying source files.
- **Placeholder preamble**: The preamble is intentionally minimal because the next task ("Write deploy-agent preamble body") is responsible for writing the full behavioral instructions. The placeholder satisfies the non-empty preamble validation rule.

## Dependencies

- Blocked by: None
- Blocking: "Write deploy-agent preamble body"

## Risks & Edge Cases

- **Tool existence validation failure**: The tools `docker_build`, `docker_push`, and `register_agent` do not yet exist in the tool registry. Loading this skill file through the standard validation pipeline will fail the `check_tools_exist` validation. This is expected and consistent with how `tool-coder.md` and `skill-writer.md` were bootstrapped before their tools existed. The tool-coder agent is responsible for generating these tool implementations separately.
- **Version float coercion**: The version `"0.1"` must be quoted in YAML. Without quotes, YAML parsers interpret `0.1` as a float, which will fail deserialization into the `String` type. The spec mandates quoting.
- **Preamble placeholder**: The placeholder text must not be accidentally left in place when the blocking task is completed. The next task should fully replace it.
- **`deploy` action novelty**: The existing skill files use `read`, `write`, `execute`, and `query` as allowed actions. The `deploy` action is new and may need to be recognized by the orchestrator or constraint-checking logic if action validation is added in the future. Currently, `allowed_actions` is a `Vec<String>` with no whitelist enforcement.

## Verification

1. **Parse check**: Run the skill-loader's YAML frontmatter parser against `skills/deploy-agent.md` and confirm it deserializes into a valid `SkillManifest` struct without errors (using `AllToolsExist` stub to bypass tool registry checks).
2. **Field-by-field confirmation**: Verify each frontmatter field matches the values specified in the Requirements section above.
3. **Validation rules**: Confirm the file passes all validation checks in `crates/skill-loader/src/validation.rs`:
   - `name`, `version`, `model.provider`, `model.name` are non-empty strings.
   - Preamble is non-empty.
   - `confidence_threshold` (0.9) is in [0.0, 1.0].
   - `max_turns` (10) is greater than 0.
   - `output.format` ("structured_json") is in the allowed formats list.
   - `escalate_to` ("human_reviewer") is non-empty.
4. **No standalone `---` in preamble**: Confirm the markdown body contains no lines that are exactly `---`.
5. **Cargo test**: Run `cargo test` to ensure no existing tests are broken by the addition of the new file.
