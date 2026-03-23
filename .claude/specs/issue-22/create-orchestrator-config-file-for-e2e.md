# Spec: Create orchestrator config file for E2E

> From: .claude/tasks/issue-22.md

## Objective

Create `tests/e2e/orchestrator-config.yml`, a YAML configuration file that the orchestrator can load via `OrchestratorConfig::from_file()` during end-to-end tests. The file declares seed agent entries following the `OrchestratorConfig` schema defined in `crates/orchestrator/src/config.rs`. This is the static configuration that tells the orchestrator which agents exist and how to reach them within the docker-compose E2E network.

## Current State

- The `tests/e2e/` directory does not exist yet.
- `OrchestratorConfig` supports loading from a YAML file via `from_file()` (see `crates/orchestrator/src/config.rs:57-65`). The schema expects a top-level `agents` array and optional `embedding_provider`, `embedding_model`, and `similarity_threshold` fields.
- Each agent entry (`AgentConfig`) has three required string fields: `name`, `description`, and `url`.
- The orchestrator resolves agent URLs by appending `/invoke` and `/health` to the base `url` field (see `crates/orchestrator/src/agent_endpoint.rs`).
- Six seed skill files exist under `skills/`: `echo`, `skill-writer`, `orchestrator`, `cogs-analyst`, `tool-coder`, and `deploy-agent`.
- The orchestrator skill (`skills/orchestrator.md`) is the router itself and should NOT appear as a routable agent entry in the config.

## Requirements

- Create directory `tests/e2e/` if it does not exist.
- Create `tests/e2e/orchestrator-config.yml` that deserializes cleanly into `OrchestratorConfig`.
- The `agents` array must contain entries for the five seed agents (excluding `orchestrator` itself): `echo`, `skill-writer`, `cogs-analyst`, `tool-coder`, and `deploy-agent`.
- Each agent entry must have:
  - `name`: matching the skill file's `name` field exactly.
  - `description`: a concise sentence describing the agent's capability, suitable for semantic routing.
  - `url`: a docker-compose service URL using the pattern `http://<service-name>:8080` where `<service-name>` matches the agent name (with hyphens for multi-word names). Port 8080 is the conventional agent-runtime HTTP port.
- Include `similarity_threshold: 0.8` as a reasonable default for semantic routing in E2E tests.
- Do NOT include `embedding_provider` or `embedding_model` -- these are optional and will be configured via environment variables in docker-compose if needed.

## Implementation Details

### Files to create

1. **`tests/e2e/orchestrator-config.yml`** with the following content:

   ```yaml
   agents:
     - name: echo
       description: "Echoes input back for testing"
       url: "http://echo:8080"

     - name: skill-writer
       description: "Produces validated skill files from plain-language descriptions"
       url: "http://skill-writer:8080"

     - name: cogs-analyst
       description: "Analyzes cost-of-goods-sold data and produces financial reports"
       url: "http://cogs-analyst:8080"

     - name: tool-coder
       description: "Generates MCP tool implementations from specifications"
       url: "http://tool-coder:8080"

     - name: deploy-agent
       description: "Handles deployment workflows and infrastructure provisioning"
       url: "http://deploy-agent:8080"

   similarity_threshold: 0.8
   ```

### Files NOT modified by this task

- `crates/orchestrator/src/config.rs` -- no schema changes needed.
- `docker-compose.yml` -- service definitions are a separate task that this spec blocks.

### Key design decisions

- **Agent descriptions match skill frontmatter:** The `description` values are drawn from each skill's frontmatter `description` field to ensure consistency between the skill definitions and the orchestrator's routing metadata.
- **Docker-compose service names as hostnames:** In a docker-compose network, service names resolve as DNS hostnames. The URL pattern `http://<service-name>:8080` assumes each agent runs as a separate docker-compose service on port 8080, which is the standard port used by `agent-runtime`.
- **Orchestrator excluded from agents list:** The orchestrator is the router, not a routable target. Including it would create a self-referential routing loop.
- **Similarity threshold at 0.8:** A threshold of 0.8 is strict enough to avoid false routing in E2E tests while permitting reasonable semantic matches. This can be tuned later based on test results.
- **No embedding config in YAML:** Embedding provider and model are environment-specific settings better suited to environment variables or docker-compose env files, not a static config committed to the repo.

## Dependencies

- Blocked by: Nothing (can be created independently).
- Blocking: docker-compose setup (needs to know the agent service names and config file location).

## Risks & Edge Cases

- **Port mismatch:** If agent-runtime binds to a port other than 8080, all URLs in this config will be wrong. Mitigation: confirm the default port in agent-runtime's `main.rs` or config during implementation. Adjust if needed.
- **Service name mismatch with docker-compose:** The service names used here (`echo`, `skill-writer`, `cogs-analyst`, `tool-coder`, `deploy-agent`) must exactly match the service names defined in the docker-compose file. Any discrepancy will cause DNS resolution failures at runtime. The docker-compose task should reference this config file as the source of truth for service naming.
- **YAML parsing strictness:** `serde_yaml` will reject unknown fields by default unless `#[serde(deny_unknown_fields)]` is absent (it is absent on `OrchestratorConfig`). Extra whitespace or comments are fine. However, any typos in field names (e.g., `agent` instead of `agents`) will cause deserialization to fail.
- **Trailing slashes on URLs:** The `AgentEndpoint::new()` constructor trims trailing slashes, so `http://echo:8080/` and `http://echo:8080` are equivalent. The config uses the no-trailing-slash form for consistency.

## Verification

- `tests/e2e/orchestrator-config.yml` exists and is valid YAML.
- The file contains exactly five agent entries with names: `echo`, `skill-writer`, `cogs-analyst`, `tool-coder`, `deploy-agent`.
- Each entry has all three required fields: `name`, `description`, `url`.
- The `similarity_threshold` field is present and set to `0.8`.
- No `embedding_provider` or `embedding_model` fields are present.
- The file can be loaded by `OrchestratorConfig::from_file("tests/e2e/orchestrator-config.yml")` without error (verifiable once the crate is available in a test harness).
