# Task Breakdown: Build deploy-agent using the factory

> Create `skills/deploy-agent.md`, the third agent and first factory-produced agent, which packages a runtime binary and skill file into a minimal Docker image, pushes it to a registry, and registers the new agent endpoint with the orchestrator.

## Group 1 — Create skill file with frontmatter and preamble

_Tasks in this group can be done in parallel._

- [x] **Create `skills/deploy-agent.md` with YAML frontmatter** `[S]`
      Create the file with frontmatter matching the SkillManifest schema. Fields: `name: deploy-agent`, `version: "0.1"`, `description` summarizing the agent's purpose (packages a runtime binary and skill file into a minimal Docker image, pushes to a registry, and registers the agent with the orchestrator), `model` with `provider: anthropic`, `name: claude-sonnet-4-6`, `temperature: 0.1`, `tools: [docker_build, docker_push, register_agent]`, `constraints` with `max_turns: 10`, `confidence_threshold: 0.9`, `escalate_to: human_reviewer`, `allowed_actions: [read, execute, deploy]`, `output` with `format: structured_json` and schema keys `image_uri: string`, `endpoint_url: string`, `health_check: string`. Follow the exact format of existing skills like `skills/skill-writer.md` and `skills/tool-coder.md`. Ensure `version` is quoted to prevent YAML float coercion.
      Files: `skills/deploy-agent.md`
      Blocking: "Write deploy-agent preamble body"

- [x] **Write deploy-agent preamble body** `[M]`
      Write the markdown body (preamble) after the closing `---` delimiter. This is the core of the skill — it instructs the LLM on how to build, push, and register agent containers. The preamble must include:
      (1) An introduction establishing this as the third agent in Spore's self-bootstrapping factory and the first agent produced BY the factory (skill-writer + tool-coder). Explain that this closes the self-extension loop: from this point, new agents are described, written, tooled, and deployed entirely within the system.
      (2) A **Docker Build Process** section documenting the build pattern from the project `Dockerfile`: multi-stage build, compile with `cargo build --release --target x86_64-unknown-linux-musl` for a fully static binary, `FROM scratch` final stage, copy the agent-runtime binary and skill file, copy CA certificates for TLS, expose port 8080, run as non-root user 1000. Reference that images should be 1-5MB.
      (3) An **Image Tagging Convention** section: tag images as `spore-{agent-name}:{version}` derived from the skill manifest's `name` and `version` fields (from `SkillManifest` in `crates/agent-sdk/src/skill_manifest.rs`).
      (4) A **Registry Push** section: push the built image to the configured container registry (sourced from environment variable or configuration).
      (5) An **Orchestrator Registration** section: call the orchestrator's agent registry API to register the new agent's HTTP endpoint so it becomes discoverable and routable. Reference that the orchestrator uses `list_agents` and `route_to_agent` tools (from `skills/orchestrator.md`).
      (6) A **Health Verification** section: after deployment, call `/health` on the new agent endpoint to verify it started correctly before reporting success.
      (7) A **Process** section with numbered steps: receive the skill name and version, locate the compiled agent-runtime binary and skill file, build the Docker image following the scratch-based pattern, tag it with the naming convention, push to the registry, register the endpoint with the orchestrator, verify health, return results.
      (8) An **Output** section describing the structured JSON response with `image_uri` (registry path of the pushed image), `endpoint_url` (HTTP endpoint where the agent is reachable), and `health_check` (result of the post-deployment health verification).
      (9) Avoid any standalone `---` lines in the body (use `----` for horizontal rules if needed).
      Reference files for accuracy: `Dockerfile` (Docker build pattern), `crates/agent-sdk/src/skill_manifest.rs` (manifest fields for tagging), `crates/agent-runtime/src/main.rs` (the binary being packaged), `skills/orchestrator.md` (registration target), `README.md` lines 109-118 (self-bootstrapping factory description).
      Files: `skills/deploy-agent.md`
      Blocking: "Add integration test for deploy-agent skill"

## Group 2 — Integration test

_Depends on: Group 1._

- [x] **Add integration test for deploy-agent skill** `[S]`
      Add a `load_deploy_agent_skill` test to `crates/skill-loader/tests/example_skills_test.rs` following the exact pattern of existing tests (e.g., `load_tool_coder_skill`, `load_skill_writer_skill`). The test should: call `loader.load("deploy-agent").await.unwrap()`, assert all frontmatter fields match expected values (`name: "deploy-agent"`, `version: "0.1"`, `description` contains "Docker" or "deploy", `model.provider: "anthropic"`, `model.name: "claude-sonnet-4-6"`, `model.temperature: 0.1`, `tools: ["docker_build", "docker_push", "register_agent"]`, `constraints.max_turns: 10`, `constraints.confidence_threshold: 0.9`, `constraints.escalate_to: Some("human_reviewer")`, `constraints.allowed_actions: ["read", "execute", "deploy"]`, `output.format: "structured_json"`, `output.schema` has 3 keys: `image_uri`, `endpoint_url`, `health_check`). Assert the preamble is non-empty and contains keyword presence checks: "Docker" or "docker" or "container", "registry" or "push", "orchestrator" or "register", "health" or "verify", "scratch" or "minimal". Use the same `make_loader` and `skills_dir` helpers already defined in the test file.
      Files: `crates/skill-loader/tests/example_skills_test.rs`
      Blocked by: "Create `skills/deploy-agent.md` with YAML frontmatter", "Write deploy-agent preamble body"
      Blocking: "Run verification suite"

## Group 3 — Verification

_Depends on: Group 2._

- [x] **Run verification suite** `[S]`
      Run `cargo check`, `cargo clippy`, and `cargo test` across the full workspace. Verify all existing tests pass plus the new `load_deploy_agent_skill` test. Confirm `SkillLoader::load("deploy-agent")` succeeds with `AllToolsExist` and the preamble contains Docker deployment guidance.
      Files: (none — command-line verification only)
      Blocked by: All other tasks

## Implementation Notes

1. **Do not modify frontmatter of other skills**: Only `skills/deploy-agent.md` is created. No changes to existing skill files.

2. **Preamble quality is the deliverable**: Like skill-writer and tool-coder, the deploy-agent's effectiveness depends on how thoroughly its preamble encodes the deployment process. The preamble must contain enough detail for an LLM to orchestrate Docker builds, registry pushes, and orchestrator registration without seeing the actual Dockerfile or runtime source.

3. **Stub tools are intentional**: `docker_build`, `docker_push`, and `register_agent` are declared in frontmatter but do not exist in the tool registry yet. They will be implemented by the tool-coder agent or manually via the tool-registry (issue #8). The test uses `AllToolsExist` to bypass this check, matching the approach used for skill-writer and tool-coder.

4. **`escalate_to: human_reviewer`**: The triage comment specifies this value. It is a non-empty string, so it satisfies the validation rule in `crates/skill-loader/src/validation.rs`.

5. **`deploy` is a new action category**: The `allowed_actions` list includes `deploy` alongside `read` and `execute`. This is a new action type not used by other skills. The validation layer does not restrict action names to a fixed set, so this is valid.

6. **Heavy infrastructure dependencies at runtime**: The deploy-agent skill file can be created and tested now (it is just a markdown file with YAML frontmatter), but it cannot be functionally exercised until Docker tooling, a container registry, and the orchestrator registration API are implemented (issues #8-15). This issue is specifically about the skill file, not the tools.

7. **Docker-in-Docker consideration**: The preamble should note that if the deploy-agent runs inside a container itself, it needs Docker access (socket mounting or remote builder). This is a deployment concern to document but not solve in this issue.

8. **Reference the existing Dockerfile pattern**: The project already has a well-structured `Dockerfile` with dependency caching, musl static linking, `FROM scratch`, CA certificate copying, and non-root user. The preamble should describe this exact pattern rather than inventing a new one.

## Critical Files for Implementation

- `skills/deploy-agent.md` — New file to create; the primary deliverable
- `skills/tool-coder.md` — Pattern to follow for frontmatter structure and preamble depth
- `Dockerfile` — Reference for the Docker build pattern the preamble must describe
- `crates/agent-sdk/src/skill_manifest.rs` — SkillManifest struct for image tagging convention
- `crates/agent-runtime/src/main.rs` — The binary being packaged
- `skills/orchestrator.md` — Registration target reference
- `crates/skill-loader/tests/example_skills_test.rs` — Add integration test following existing patterns
