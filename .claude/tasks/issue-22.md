# Task Breakdown: End-to-end self-bootstrapping pipeline validation

> Validate the complete self-bootstrapping pipeline by describing a capability in natural language and verifying that the system produces a skill file, implements tools, packages a Docker image, registers with the orchestrator, and routes live requests to the new agent.

## Group 1 — Test scenario and docker-compose infrastructure

_Tasks in this group can be done in parallel._

- [x] **Define the test scenario document** `[S]`
      Create a file `tests/e2e/SCENARIO.md` that documents the chosen test scenario: a temperature-conversion agent. Describe the natural language input ("An agent that converts temperatures between Fahrenheit, Celsius, and Kelvin"), the expected skill file structure (name, tools, constraints), the expected tool (`convert_temperature`), and the success criteria for each pipeline stage. This document is the reference for all subsequent tasks and ensures everyone agrees on what "passing" looks like.
      Files: `tests/e2e/SCENARIO.md`
      Blocking: "Write step 1 validator: skill-writer invocation", "Write step 2 validator: tool-coder invocation", "Write step 3 validator: deploy-agent invocation", "Write step 4 validator: orchestrator routing", "Write the E2E shell script orchestrator"

- [x] **Create `docker-compose.e2e.yml`** `[M]`
      Create a docker-compose file at the project root that defines the multi-container test environment. Services needed: `skill-writer` (agent-runtime with `SKILL_NAME=skill-writer`), `tool-coder` (agent-runtime with `SKILL_NAME=tool-coder`), `deploy-agent` (agent-runtime with `SKILL_NAME=deploy-agent`), `orchestrator` (agent-runtime with `SKILL_NAME=orchestrator`, configured with `AGENT_ENDPOINTS` and `AGENT_DESCRIPTIONS` env vars pointing to the other services). All services share a Docker network for inter-agent HTTP. The `deploy-agent` service needs Docker socket access (`/var/run/docker.sock:/var/run/docker.sock`) for image building. Each service exposes port 8080 internally with unique host-mapped ports. Use the project `Dockerfile` with `--build-arg SKILL_NAME=<name>` for each service. Include health checks using `GET /health` on each container's port 8080.
      Files: `docker-compose.e2e.yml`
      Blocking: "Write the E2E shell script orchestrator"

- [x] **Create orchestrator config file for E2E** `[S]`
      Create `tests/e2e/orchestrator-config.yml` with entries for the seed agents (skill-writer, tool-coder, deploy-agent) and their descriptions, following the `OrchestratorConfig` YAML schema from `crates/orchestrator/src/config.rs`. The URLs should match the docker-compose service names (e.g., `http://skill-writer:8080`). Include embedding provider/model settings if semantic routing is needed, or configure `AGENT_ENDPOINTS` and `AGENT_DESCRIPTIONS` environment variables in the compose file directly.
      Files: `tests/e2e/orchestrator-config.yml`
      Blocking: "Create `docker-compose.e2e.yml`"

## Group 2 — Pipeline step validators

_Depends on: Group 1._

- [x] **Write step 1 validator: skill-writer invocation** `[M]`
      Create `tests/e2e/validate_step1_skill.sh`. This script sends a `POST` to `skill-writer:8080/invoke` with an `AgentRequest` JSON body (`id` as UUID, `input` as the natural language description from the scenario). It captures the `AgentResponse`, extracts the `output.skill_yaml` field, and validates: (1) the YAML frontmatter deserializes to valid YAML with required fields (`name`, `version`, `description`, `model`, `tools`, `constraints`, `output`), (2) at least one tool is declared in the `tools` list, (3) the preamble (markdown body) is non-empty. Save the extracted skill file to `tests/e2e/artifacts/generated-skill.md` for use by subsequent steps. Use `curl` and `jq` for HTTP and JSON processing. Exit non-zero on any validation failure, printing the failure reason. Handle LLM non-determinism by validating structure, not exact content.
      Files: `tests/e2e/validate_step1_skill.sh`
      Blocked by: "Define the test scenario document"
      Blocking: "Write the E2E shell script orchestrator"

- [x] **Write step 2 validator: tool-coder invocation** `[M]`
      Create `tests/e2e/validate_step2_tools.sh`. This script reads the generated skill file from `tests/e2e/artifacts/generated-skill.md`, sends it as the `input` field in a `POST` to `tool-coder:8080/invoke`, and validates the response: (1) `output.compilation_result` contains "success", (2) `output.tools_generated` is non-empty, (3) `output.implementation_paths` lists valid paths under `tools/`. Save the response to `tests/e2e/artifacts/step2-response.json`. Exit non-zero with diagnostics on failure. Allow retries (up to 3 attempts) since LLM-generated code may not compile on the first try.
      Files: `tests/e2e/validate_step2_tools.sh`
      Blocked by: "Define the test scenario document"
      Blocking: "Write the E2E shell script orchestrator"

- [x] **Write step 3 validator: deploy-agent invocation** `[M]`
      Create `tests/e2e/validate_step3_deploy.sh`. This script constructs an input combining the skill file path and tool paths from steps 1-2, sends it as `POST deploy-agent:8080/invoke`, and validates: (1) `output.image_uri` is non-empty and follows the `spore-{name}:{version}` convention, (2) `output.endpoint_url` is a valid HTTP URL, (3) `output.health_check` is "healthy". Save the response to `tests/e2e/artifacts/step3-response.json`. Use generous timeouts (5+ minutes) since Docker builds are slow.
      Files: `tests/e2e/validate_step3_deploy.sh`
      Blocked by: "Define the test scenario document"
      Blocking: "Write the E2E shell script orchestrator"

- [x] **Write step 4 validator: orchestrator routing** `[M]`
      Create `tests/e2e/validate_step4_route.sh`. This script sends a domain request (`POST orchestrator:8080/invoke` with input "Convert 100 degrees Fahrenheit to Celsius") and validates: (1) the response is a valid `AgentResponse` with `confidence >= 0.8`, (2) the `output` field contains a numerically reasonable answer (approximately 37.78), (3) the `tool_calls` list is non-empty (proving the agent used its tools). Also test with a second query ("Convert 0 Kelvin to Celsius", expecting approximately -273.15) to verify the agent handles multiple conversions. Save responses to `tests/e2e/artifacts/step4-response.json`.
      Files: `tests/e2e/validate_step4_route.sh`
      Blocked by: "Define the test scenario document"
      Blocking: "Write the E2E shell script orchestrator"

## Group 3 — E2E test script

_Depends on: Groups 1 and 2._

- [x] **Write the E2E shell script orchestrator** `[L]`
      Create `scripts/e2e-test.sh` as the top-level test driver. This script: (1) starts the docker-compose environment with `docker compose -f docker-compose.e2e.yml up -d --build`, (2) waits for all services to become healthy by polling `/health` endpoints with exponential backoff (1s, 2s, 4s, ..., up to 60s total), (3) creates `tests/e2e/artifacts/` directory for intermediate outputs, (4) runs each step validator in sequence (step 1 through step 4), capturing logs and intermediate artifacts at each stage, (5) on any step failure: dumps container logs (`docker compose logs`), prints the step number and error, and exits non-zero, (6) on success: prints a summary of all steps passed, (7) always runs cleanup in a trap: `docker compose -f docker-compose.e2e.yml down -v --remove-orphans`. Include a `--no-cleanup` flag for debugging that skips teardown. Include a `--timeout` flag (default 10 minutes) for the overall test. Print elapsed time per step and total. Ensure all intermediate outputs (generated skill file, tool code, Docker build logs, orchestrator routing decisions) are saved for post-mortem debugging.
      Files: `scripts/e2e-test.sh`
      Blocked by: "Create `docker-compose.e2e.yml`", "Write step 1 validator: skill-writer invocation", "Write step 2 validator: tool-coder invocation", "Write step 3 validator: deploy-agent invocation", "Write step 4 validator: orchestrator routing"
      Blocking: "Add Rust integration test wrapper"

## Group 4 — Rust integration test and documentation

_Depends on: Group 3._

- [x] **Add Rust integration test wrapper** `[M]`
      Create `tests/e2e_bootstrap_test.rs` at the workspace root. This integration test wraps the shell script for `cargo test` integration. Use `#[test] #[ignore]` (since it requires Docker and is slow). The test calls `Command::new("bash").arg("scripts/e2e-test.sh")` and asserts exit code 0. Capture stdout and stderr, printing them on failure. Add a `#[cfg(feature = "e2e")]` gate so it only runs when explicitly requested. Add the `e2e` feature to the workspace `Cargo.toml` if needed. This allows running the test via `cargo test --features e2e -- --ignored e2e_bootstrap_test` while keeping normal `cargo test` fast.
      Files: `tests/e2e_bootstrap_test.rs`, `Cargo.toml`
      Blocked by: "Write the E2E shell script orchestrator"
      Blocking: "Write README section for E2E testing"

- [x] **Write README section for E2E testing** `[S]`
      Add a section to `README.md` documenting how to run the E2E test. Include prerequisites (Docker, docker-compose, sufficient API credits), the command (`./scripts/e2e-test.sh` or `cargo test --features e2e -- --ignored`), expected runtime (5-10 minutes), how to debug failures (check `tests/e2e/artifacts/`), the `--no-cleanup` flag, and cost considerations (each run invokes multiple LLM calls).
      Files: `README.md`
      Blocked by: "Add Rust integration test wrapper"
      Blocking: None

## Group 5 — Verification

_Depends on: Group 4._

- [x] **Run verification suite** `[S]`
      Run `cargo check`, `cargo clippy`, and `cargo test` to ensure the Rust wrapper compiles and does not break existing tests. Verify the shell scripts are executable (`chmod +x`). Do a dry run of `docker compose -f docker-compose.e2e.yml config` to validate the compose file syntax. Do NOT run the full E2E test in this step (it requires LLM API keys and Docker) — just verify everything is syntactically valid and the project still builds.
      Files: (none — command-line verification only)
      Blocked by: All other tasks
      Blocking: None

## Implementation Notes

1. **Shell-first approach**: The triage comment recommends a shell script as the initial validation approach, with a Rust integration test as a follow-up. This breakdown follows that recommendation, with the Rust test being a thin wrapper around the shell script.

2. **LLM non-determinism**: Every validator checks structure, not exact content. Skill files are validated by field presence and type, not by exact values. Tool code is validated by compilation success, not by exact implementation. Agent output is validated by numeric reasonableness (e.g., 37-39 for 100F to C), not by exact equality.

3. **Cost awareness**: Each E2E run invokes at minimum 4 LLM calls (skill-writer, tool-coder, deploy-agent routing, temperature-agent invocation). With retries, this could be 8-12 calls. The README section should document this.

4. **Depends on ALL prior issues**: As the triage comment notes, this is the capstone test. Issues #5 through #21 must all be complete for the pipeline to function end-to-end. The test will fail if any component is broken.

5. **Docker-in-Docker**: The deploy-agent container must have Docker access to build images. The docker-compose file mounts the Docker socket. This is acceptable for testing but should be documented as a security consideration.

6. **Orchestrator has no binary**: The orchestrator crate is currently a library (`lib.rs` only, no `main.rs`). The E2E test assumes it runs inside the `agent-runtime` binary with `SKILL_NAME=orchestrator`. However, the orchestrator has its own routing logic that differs from a standard agent-runtime. The docker-compose setup may need the orchestrator to run as a separate binary or the agent-runtime needs to detect the orchestrator skill and use `Orchestrator::from_config` instead of the standard `RuntimeAgent`. This is a potential blocker that may need resolution in prior issues.

7. **Artifact preservation**: Every step saves its outputs to `tests/e2e/artifacts/` so that failures at step 3 still have the generated skill file from step 1 available for debugging.

8. **No CI/CD yet**: There is no `.github/workflows/` directory. This test could become the foundation for CI, but that is a follow-up concern.

## Critical Files for Implementation

- `scripts/e2e-test.sh` — Top-level test driver script
- `docker-compose.e2e.yml` — Multi-container test environment
- `tests/e2e/SCENARIO.md` — Test scenario definition
- `tests/e2e/validate_step{1-4}_*.sh` — Per-step validators
- `tests/e2e_bootstrap_test.rs` — Rust integration test wrapper
- `crates/orchestrator/src/orchestrator.rs` — Orchestrator routing logic
- `crates/agent-runtime/src/http.rs` — /invoke and /health endpoints
