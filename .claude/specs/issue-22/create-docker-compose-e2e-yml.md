# Spec: Create `docker-compose.e2e.yml`

> From: .claude/tasks/issue-22.md

## Objective

Create a Docker Compose file at the repository root that defines four services (skill-writer, tool-coder, deploy-agent, orchestrator) for end-to-end testing. Each service builds from the project's existing `Dockerfile` using the `SKILL_NAME` build arg to select its skill file. The orchestrator service receives environment variables pointing it at the three agent services. The deploy-agent service mounts the Docker socket for container operations. All services share a single Docker network and expose unique host-mapped ports for external access and health-check probing.

## Current State

### Dockerfile (`Dockerfile`)

The project has a single multi-stage Dockerfile that:
1. Builds a statically-linked `agent-runtime` binary (musl target).
2. Produces a `FROM scratch` final image containing only the binary, the `/skills/` directory, and CA certificates.
3. Accepts a build arg `SKILL_NAME` (default: `echo`) which becomes the `SKILL_NAME` environment variable at runtime.
4. Sets `SKILL_DIR=/skills` and `SSL_CERT_FILE=/etc/ssl/certs/ca-certificates.crt`.
5. Exposes port `8080` and runs as user `1000`.
6. Entrypoint is `/agent-runtime`.

### Runtime Configuration (`crates/agent-runtime/src/config.rs`)

`RuntimeConfig::from_env()` reads:
- `SKILL_NAME` (required) -- which skill file to load.
- `SKILL_DIR` (optional, default `./skills`) -- directory containing skill markdown files.
- `BIND_ADDR` (optional, default `0.0.0.0:8080`) -- socket address for the HTTP server.

### HTTP Server (`crates/agent-runtime/src/http.rs`)

The agent-runtime exposes two endpoints:
- `GET /health` -- returns JSON with `name`, `version`, `status` fields. This is the health-check target.
- `POST /invoke` -- forwards requests to the agent.

### Tool Endpoints (`crates/agent-runtime/src/main.rs`)

`TOOL_ENDPOINTS` env var: comma-separated `name=endpoint` pairs (e.g., `echo-tool=mcp://localhost:7001`). Falls back to `echo-tool=mcp://localhost:7001` when unset.

### Orchestrator Configuration (`crates/orchestrator/src/config.rs`)

`OrchestratorConfig::from_env()` reads:
- `AGENT_ENDPOINTS` (required) -- comma-separated `name=url` pairs (e.g., `skill-writer=http://skill-writer:8080`).
- `AGENT_DESCRIPTIONS` (optional) -- comma-separated `name=description` pairs.
- `EMBEDDING_PROVIDER` (optional) -- embedding provider name.
- `EMBEDDING_MODEL` (optional) -- embedding model name.
- `SIMILARITY_THRESHOLD` (optional) -- float for routing similarity.

### Skill Files (`skills/`)

Four skill files exist for the four services:
- `skills/skill-writer.md` -- name: `skill-writer`, version: `0.1`
- `skills/tool-coder.md` -- name: `tool-coder`, version: `0.1`
- `skills/deploy-agent.md` -- name: `deploy-agent`, version: `0.1`
- `skills/orchestrator.md` -- name: `orchestrator`, version: `1.0`

### Note on the Orchestrator Binary

The orchestrator crate (`crates/orchestrator/`) is currently a library crate with no `main.rs`. The `agent-runtime` binary loads the `orchestrator` skill file the same way it loads any other skill -- via the `SKILL_NAME` env var. Therefore all four services use the same `agent-runtime` binary and the same Dockerfile; only the `SKILL_NAME` build arg differs.

## Requirements

1. **File location**: Create `docker-compose.e2e.yml` at the repository root (`/workspaces/spore/docker-compose.e2e.yml`).

2. **Four services**: `skill-writer`, `tool-coder`, `deploy-agent`, `orchestrator`.

3. **Shared build context**: All services build from the same `Dockerfile` at `.` (repository root), each passing a different `SKILL_NAME` build arg matching its skill file name.

4. **Unique host-mapped ports**: Each service listens internally on `8080` (the default `BIND_ADDR`). Map to unique host ports:
   - `skill-writer`: host port `8081`
   - `tool-coder`: host port `8082`
   - `deploy-agent`: host port `8083`
   - `orchestrator`: host port `8084`

5. **Health checks**: Do not define `healthcheck` directives in the compose file. The `FROM scratch` image contains no shell, `curl`, or `wget`, so container-internal health check commands cannot execute. The E2E shell script (which this file blocks) will handle health polling from the host side by curling each service's `GET /health` endpoint on its host-mapped port.

6. **Shared network**: All services join a single custom bridge network named `spore-e2e` so they can reach each other by service name (e.g., `http://skill-writer:8080`).

7. **Docker socket mount for deploy-agent**: The `deploy-agent` service mounts `/var/run/docker.sock:/var/run/docker.sock` so it can build and push Docker images from inside its container. This requires overriding the default user (scratch image runs as `1000`) -- set `user: "0"` (root) on the deploy-agent service only, since accessing the Docker socket typically requires root or docker-group membership.

8. **Orchestrator environment variables**: The orchestrator service needs `AGENT_ENDPOINTS` set to the comma-separated list of the three agent services:
   ```
   AGENT_ENDPOINTS=skill-writer=http://skill-writer:8080,tool-coder=http://tool-coder:8080,deploy-agent=http://deploy-agent:8080
   ```
   Optionally set `AGENT_DESCRIPTIONS` with each agent's description from their skill files (descriptions must not contain commas or equals signs due to the parser's `parse_comma_pairs` format).

9. **Orchestrator depends_on**: The orchestrator service should declare `depends_on` on the three agent services so Docker Compose starts them first.

10. **Container naming**: Use `container_name` matching the service name for predictable DNS and log identification: `skill-writer`, `tool-coder`, `deploy-agent`, `orchestrator`.

## Implementation Details

### File to create

**`docker-compose.e2e.yml`**

```yaml
version: "3.8"

networks:
  spore-e2e:
    driver: bridge

services:
  skill-writer:
    build:
      context: .
      dockerfile: Dockerfile
      args:
        SKILL_NAME: skill-writer
    container_name: skill-writer
    ports:
      - "8081:8080"
    networks:
      - spore-e2e

  tool-coder:
    build:
      context: .
      dockerfile: Dockerfile
      args:
        SKILL_NAME: tool-coder
    container_name: tool-coder
    ports:
      - "8082:8080"
    networks:
      - spore-e2e

  deploy-agent:
    build:
      context: .
      dockerfile: Dockerfile
      args:
        SKILL_NAME: deploy-agent
    container_name: deploy-agent
    ports:
      - "8083:8080"
    user: "0"
    volumes:
      - /var/run/docker.sock:/var/run/docker.sock
    networks:
      - spore-e2e

  orchestrator:
    build:
      context: .
      dockerfile: Dockerfile
      args:
        SKILL_NAME: orchestrator
    container_name: orchestrator
    ports:
      - "8084:8080"
    environment:
      AGENT_ENDPOINTS: "skill-writer=http://skill-writer:8080,tool-coder=http://tool-coder:8080,deploy-agent=http://deploy-agent:8080"
      AGENT_DESCRIPTIONS: "skill-writer=Produces validated skill files from plain-language descriptions,tool-coder=Generates compiles and validates Rust MCP tool implementations,deploy-agent=Packages runtime binary and skill file into Docker image and registers agent"
    depends_on:
      - skill-writer
      - tool-coder
      - deploy-agent
    networks:
      - spore-e2e
```

### Key design decisions

| Decision | Rationale |
|----------|-----------|
| No `healthcheck` directives in compose | The `FROM scratch` image contains no shell, `curl`, or `wget`. Health polling is delegated to the E2E test script that consumes this compose file. |
| `user: "0"` on deploy-agent only | Docker socket access requires root or docker-group membership. Other services remain at user `1000` (Dockerfile default). |
| `depends_on` without condition | Since there are no health checks in compose, `depends_on` only controls startup order, not readiness. The E2E script must wait for health before running tests. |
| `AGENT_DESCRIPTIONS` values avoid commas | The orchestrator's `parse_comma_pairs` function splits on `,` then on `=`. Descriptions must not contain commas or equals signs, so they are simplified from the original skill file descriptions. |
| Single Dockerfile for all services | The `SKILL_NAME` build arg is the only differentiator. This matches the project's architecture where `agent-runtime` is a generic binary parameterized by skill file. |
| Host ports 8081-8084 | Sequential and memorable. Avoids conflict with any service binding to 8080 on the host. |

### Files to modify

None. This is a new file.

## Dependencies

- **Blocked by**: Nothing. The Dockerfile and skill files already exist.
- **Blocking**: E2E shell script -- the test script will reference `docker-compose.e2e.yml` to bring up the services, poll health endpoints, run tests, and tear down.

## Risks & Edge Cases

1. **Docker socket security**: Mounting the Docker socket gives the deploy-agent container full control of the host Docker daemon. This is acceptable for local E2E testing but must not be used in production without scoping (e.g., Docker socket proxy).

2. **Port conflicts**: If host ports 8081-8084 are already in use, `docker compose up` will fail. The E2E script should handle this gracefully or document the port requirements.

3. **Build cache sharing**: All four services build from the same Dockerfile with different `SKILL_NAME` args. Docker Compose builds them sequentially by default. The builder stage (dependency compilation) is shared across builds since the `SKILL_NAME` arg is only used in the final stage. This means only the first build is slow; subsequent builds reuse the cached builder layer.

4. **`AGENT_DESCRIPTIONS` parsing**: The comma-separated format means descriptions cannot contain literal commas. The descriptions in the compose file are simplified versions of the original skill file descriptions with commas removed.

5. **Orchestrator startup race**: `depends_on` only ensures containers start in order, not that they are ready. The orchestrator may attempt to connect to agent endpoints before they are listening. The E2E script must poll all four health endpoints before running tests.

6. **`FROM scratch` limitations**: No shell means `docker exec` into containers is impossible. Debugging must rely on `docker logs` output only. The agent-runtime logs to stderr via `tracing_subscriber`.

## Verification

1. **File exists**: `docker-compose.e2e.yml` is present at the repository root.
2. **Valid YAML**: `docker compose -f docker-compose.e2e.yml config` parses without errors.
3. **Four services defined**: Config output shows exactly four services: `skill-writer`, `tool-coder`, `deploy-agent`, `orchestrator`.
4. **Build args correct**: Each service's build section specifies `SKILL_NAME` matching its service name.
5. **Port mappings unique**: Each service maps a unique host port (8081-8084) to container port 8080.
6. **Network shared**: All services are on the `spore-e2e` network.
7. **Docker socket mounted**: Only `deploy-agent` has the `/var/run/docker.sock` volume mount.
8. **Orchestrator env vars**: `AGENT_ENDPOINTS` contains all three agent service URLs using Docker DNS names.
9. **Orchestrator depends_on**: Lists all three agent services.
10. **No healthcheck directives**: No service defines a `healthcheck` block (scratch image incompatibility).
