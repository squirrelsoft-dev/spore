# Spec: Write deploy-agent preamble body

> From: .claude/tasks/issue-21.md

## Objective

Write the markdown preamble body for `skills/deploy-agent.md`, the section after the closing `---` frontmatter delimiter. This preamble is the core instruction set that teaches an LLM how to build, push, and register agent containers. The deploy-agent is the third agent in Spore's self-bootstrapping factory and the first agent produced BY the factory (skill-writer + tool-coder), closing the self-extension loop so that new agents can be described, written, tooled, and deployed entirely within the platform.

## Current State

- **Dockerfile** (`/workspaces/spore/Dockerfile`): Implements a two-stage build pattern:
  - Stage 1 (`rust:latest AS builder`): Installs musl-tools and cmake, adds `x86_64-unknown-linux-musl` target, creates stub source files for dependency caching, runs `cargo build --release --target x86_64-unknown-linux-musl -p agent-runtime` twice (first for cached deps, then with real sources), and verifies the binary is statically linked.
  - Stage 2 (`FROM scratch`): Copies only the static `agent-runtime` binary, the `skills/` directory, and CA certificates. Sets `SKILL_NAME` and `SKILL_DIR` env vars, exposes port 8080, runs as non-root user 1000.
- **SkillManifest** (`crates/agent-sdk/src/skill_manifest.rs`): Has `name: String` and `version: String` fields, which are the source for image tagging (`spore-{name}:{version}`).
- **agent-runtime** (`crates/agent-runtime/src/main.rs`): The binary packaged into containers. It loads config from env, registers tools, connects tool servers, loads the skill manifest from `SKILL_DIR`, builds a provider-backed agent, applies constraint enforcement, and starts an HTTP server on the configured bind address (default port 8080).
- **Orchestrator** (`skills/orchestrator.md`): Uses `list_agents` and `route_to_agent` tools. New agents must be registered so the orchestrator can discover and route to them.
- **Existing preamble pattern** (`skills/tool-coder.md`): Establishes the structure: introductory paragraph establishing lineage, detailed reference sections, a numbered Process section, and an Output section. The deploy-agent preamble must follow this same depth and structure.
- **README lines 109-118**: Describes the self-bootstrapping factory: skill-writer and tool-coder are seed agents; deploy-agent is the third agent they produce together, packaging runtime + skill into a Docker image and registering the endpoint.

## Requirements

Each requirement maps to a numbered section from the task description:

1. **Introduction paragraph**: Must establish the deploy-agent as the third agent in Spore's factory and the first produced by the factory itself (skill-writer + tool-coder). Must explain that this closes the self-extension loop. Must not use standalone `---` lines.

2. **Docker Build Process section**: Must document the multi-stage build from the project Dockerfile:
   - Stage 1: `rust:latest` builder with musl-tools, `cargo build --release --target x86_64-unknown-linux-musl` for fully static binary, dependency caching via stub sources.
   - Stage 2: `FROM scratch` final image containing only the `agent-runtime` binary, the skill file, and CA certificates (`/etc/ssl/certs/ca-certificates.crt`).
   - Port 8080 exposed, non-root user 1000.
   - Reference that final images should be 1-5MB.
   - Must note Docker-in-Docker consideration (socket mounting or remote builder if deploy-agent itself runs in a container).

3. **Image Tagging Convention section**: Tag format `spore-{agent-name}:{version}`, derived from `SkillManifest.name` and `SkillManifest.version` fields. Example: `spore-deploy-agent:0.1`.

4. **Registry Push section**: Push built image to configured container registry. Registry endpoint sourced from environment variable or configuration.

5. **Orchestrator Registration section**: Call the orchestrator's agent registry API to register the new agent's HTTP endpoint. Reference that the orchestrator uses `list_agents` and `route_to_agent` tools so the new agent becomes discoverable and routable.

6. **Health Verification section**: After deployment, call `/health` on the new agent endpoint. Verify successful response before reporting success.

7. **Process section**: Numbered steps covering the full workflow:
   - Receive skill name and version
   - Locate compiled agent-runtime binary and skill file
   - Build Docker image following the scratch-based pattern
   - Tag with naming convention
   - Push to registry
   - Register endpoint with orchestrator
   - Verify health
   - Return results

8. **Output section**: Structured JSON with three fields:
   - `image_uri`: registry path of the pushed image
   - `endpoint_url`: HTTP endpoint where the agent is reachable
   - `health_check`: result of post-deployment health verification

9. **No standalone `---` lines**: Use `----` (four dashes) for horizontal rules if needed.

## Implementation Details

### Preamble Structure (in order)

The preamble should follow this exact section ordering, matching the depth of `skills/tool-coder.md`:

1. **Opening paragraph** (no heading): "You are the deploy-agent..." establishing identity, lineage (third agent, first factory-produced), and the self-extension loop closure.

2. **`## Docker Build Process`**: Document the two-stage Dockerfile pattern in detail. Include the exact cargo build command (`cargo build --release --target x86_64-unknown-linux-musl -p agent-runtime`), the `FROM scratch` pattern, what gets copied (binary, skill file, CA certs), env vars (`SKILL_NAME`, `SKILL_DIR`, `SSL_CERT_FILE`), port 8080, user 1000. Note expected image size (1-5MB). Include Docker-in-Docker caveat.

3. **`## Image Tagging Convention`**: Format is `spore-{agent-name}:{version}`. Values come from `SkillManifest` fields `name` and `version`. Provide concrete example.

4. **`## Registry Push`**: Push tagged image to configured registry. Registry URL from `REGISTRY_URL` environment variable or equivalent config. Full image reference becomes `{registry}/spore-{agent-name}:{version}`.

5. **`## Orchestrator Registration`**: After the image is running, register the agent's endpoint with the orchestrator so it appears in `list_agents` results and can be targeted by `route_to_agent`. Include the information needed for registration (agent name, endpoint URL, capabilities/description from the skill manifest).

6. **`## Health Verification`**: HTTP GET to `{endpoint_url}/health`. Expect 200 OK. Retry with backoff if the container is still starting. Only report success after health check passes.

7. **`## Process`**: Numbered list (8 steps as enumerated in requirements).

8. **`## Output`**: Describe the three structured JSON fields with explanations matching the `output.schema` in the frontmatter.

### Key Patterns from Dockerfile

- Dependency caching: stub `lib.rs`/`main.rs` files created first, then `cargo build` to cache deps, then real sources copied and rebuilt.
- Static linking verification: `file ... | grep -qE 'static(-pie)? linked'`
- Scratch base: zero OS layer, only the binary, skill files, and CA certs.
- Environment: `SKILL_NAME=${SKILL_NAME}`, `SKILL_DIR=/skills`, `SSL_CERT_FILE=/etc/ssl/certs/ca-certificates.crt`
- Security: `USER 1000` (non-root), `EXPOSE 8080`

### Integration Points

- The deploy-agent's three tools (`docker_build`, `docker_push`, `register_agent`) are declared in frontmatter but not yet implemented. The preamble must provide enough detail that these tools' implementations can be derived from it.
- The orchestrator registration API shape is implied by the orchestrator skill's `list_agents` and `route_to_agent` tools.

## Dependencies

- Blocked by: None (logically follows the frontmatter task but can be written independently since the structure is fully specified)
- Blocking: "Add integration test for deploy-agent skill" (the integration test checks preamble keyword presence: "Docker"/"docker"/"container", "registry"/"push", "orchestrator"/"register", "health"/"verify", "scratch"/"minimal")

## Risks & Edge Cases

- **Keyword coverage for tests**: The integration test checks for specific keywords in the preamble. The preamble must naturally include: "Docker" or "docker" or "container", "registry" or "push", "orchestrator" or "register", "health" or "verify", "scratch" or "minimal". Missing any of these will cause test failure.
- **Standalone `---` lines**: YAML parsers may interpret `---` in the body as a new frontmatter block. The body must not contain any line that is exactly `---`. Use `----` if a horizontal rule is needed.
- **Preamble depth**: Like tool-coder's preamble (which is approximately 200 lines of detailed instructions), the deploy-agent preamble must encode enough detail for an LLM to orchestrate the full deployment without seeing the Dockerfile or runtime source. Insufficient detail would make the agent non-functional.
- **Docker-in-Docker**: The preamble should note that if the deploy-agent itself runs inside a container, Docker socket access or a remote builder is required. This is a documentation concern, not something to solve in this issue.
- **Future tool implementation**: The preamble describes how `docker_build`, `docker_push`, and `register_agent` should behave, which effectively serves as a spec for their eventual implementation by tool-coder. Inaccuracies here propagate to tool implementation.

## Verification

- The file `skills/deploy-agent.md` contains a preamble body after the closing `---` delimiter.
- The preamble is non-empty and contains all nine required sections/elements.
- The preamble contains the keywords checked by the integration test: at least one of "Docker"/"docker"/"container", "registry"/"push", "orchestrator"/"register", "health"/"verify", "scratch"/"minimal".
- No line in the preamble body is exactly `---` (three dashes alone).
- The Docker build pattern described matches the actual `Dockerfile` (multi-stage, musl static linking, scratch base, CA certs, port 8080, user 1000).
- The image tagging convention uses `spore-{agent-name}:{version}` derived from `SkillManifest.name` and `SkillManifest.version`.
- The Output section documents exactly three fields: `image_uri`, `endpoint_url`, `health_check`.
- The Process section has numbered steps covering the full workflow.
- `cargo test` passes, including the `load_deploy_agent_skill` integration test (once that test is implemented by the blocking task).
- The preamble style and depth are comparable to `skills/tool-coder.md`.
