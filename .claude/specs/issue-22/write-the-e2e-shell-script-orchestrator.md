# Spec: Write the E2E shell script orchestrator

> From: .claude/tasks/issue-22.md

## Objective

Create `scripts/e2e-test.sh` as the top-level test driver for the end-to-end self-bootstrapping pipeline. The script orchestrates the full lifecycle: starts the docker-compose environment, waits for services to become healthy, runs four step validators sequentially, preserves artifacts for debugging, dumps container logs on failure, tracks elapsed time, and cleans up in a trap. It supports `--no-cleanup` and `--timeout` flags to aid debugging and CI integration.

## Current State

### Dockerfile (`Dockerfile`)

The project builds a statically-linked `agent-runtime` binary using a multi-stage Docker build:
- Builder stage: `rust:latest`, targets `x86_64-unknown-linux-musl`, produces a static binary at `target/x86_64-unknown-linux-musl/release/agent-runtime`.
- Runtime stage: `FROM scratch`, copies the binary to `/agent-runtime`, copies `skills/` to `/skills/`.
- Accepts `SKILL_NAME` build-arg (default `echo`), sets `SKILL_DIR=/skills`, exposes port 8080, runs as user 1000.
- Entrypoint is `/agent-runtime`.

### Health endpoint (`crates/agent-runtime/src/http.rs`)

- `GET /health` returns a JSON `HealthResponse` with fields `name` (String), `version` (String), `status` (HealthStatus).
- `HealthStatus` is an enum: `Healthy`, `Degraded(String)`, `Unhealthy(String)`.
- A healthy response looks like: `{"name":"...","version":"...","status":"Healthy"}`.
- The runtime listens on port 8080 (configured via `RuntimeConfig::bind_addr`, default `0.0.0.0:8080`).

### Docker-compose file (not yet created)

`docker-compose.e2e.yml` will define services: `skill-writer`, `tool-coder`, `deploy-agent`, and `orchestrator`, each built from the project `Dockerfile` with different `SKILL_NAME` build-args. Each service exposes port 8080 internally with unique host-mapped ports. Services share a Docker network for inter-agent HTTP.

### Step validator scripts (not yet created)

Four shell scripts under `tests/e2e/`:
- `validate_step1_skill.sh` -- invokes skill-writer, validates skill YAML structure, saves to artifacts.
- `validate_step2_tools.sh` -- invokes tool-coder, validates compilation result, saves to artifacts.
- `validate_step3_deploy.sh` -- invokes deploy-agent, validates image and endpoint, saves to artifacts.
- `validate_step4_route.sh` -- invokes orchestrator, validates routing and numeric answers, saves to artifacts.

Each exits non-zero on failure with diagnostic output. Each expects service URLs to be reachable.

## Requirements

- The script must be a Bash script (`#!/usr/bin/env bash`) with `set -euo pipefail`.
- Parse CLI flags: `--no-cleanup` (skip teardown on exit), `--timeout <seconds>` (default 600, i.e. 10 minutes).
- Start the docker-compose environment: `docker compose -f docker-compose.e2e.yml up -d --build`.
- Wait for all four services to become healthy by polling `GET /health` with exponential backoff.
- Create `tests/e2e/artifacts/` directory for intermediate outputs.
- Run step validators 1 through 4 sequentially; stop on first failure.
- On failure: dump container logs, print which step failed and the error, exit non-zero.
- On success: print a summary of all steps passed with per-step and total elapsed times.
- Always run cleanup in a trap (unless `--no-cleanup` is set): `docker compose -f docker-compose.e2e.yml down -v --remove-orphans`.
- Enforce the overall timeout: if elapsed time exceeds the limit, kill remaining work and exit non-zero.
- Preserve all intermediate artifacts in `tests/e2e/artifacts/` regardless of outcome.
- Dump container logs to `tests/e2e/artifacts/container-logs.txt` on failure.
- The script must be executable (`chmod +x`).
- All functions must be under 50 lines per project rules.

## Implementation Details

### File to create

**`scripts/e2e-test.sh`**

The script is organized into the following sections and functions:

### 1. Shebang and strict mode

```
#!/usr/bin/env bash
set -euo pipefail
```

### 2. Constants and defaults

- `COMPOSE_FILE="docker-compose.e2e.yml"` -- path to the compose file (relative to project root).
- `ARTIFACTS_DIR="tests/e2e/artifacts"` -- directory for intermediate outputs.
- `DEFAULT_TIMEOUT=600` -- default overall timeout in seconds (10 minutes).
- `HEALTH_MAX_WAIT=60` -- maximum seconds to wait for health checks.
- `HEALTH_INITIAL_INTERVAL=1` -- initial polling interval in seconds.
- `SERVICES=("skill-writer" "tool-coder" "deploy-agent" "orchestrator")` -- list of service names matching docker-compose service definitions.
- `HOST_PORTS` associative array mapping service names to their host-mapped ports (must match `docker-compose.e2e.yml`). Example: `skill-writer=8081`, `tool-coder=8082`, `deploy-agent=8083`, `orchestrator=8084`.
- `STEPS=("tests/e2e/validate_step1_skill.sh" "tests/e2e/validate_step2_tools.sh" "tests/e2e/validate_step3_deploy.sh" "tests/e2e/validate_step4_route.sh")` -- ordered list of validator scripts.
- `STEP_NAMES=("skill-writer invocation" "tool-coder invocation" "deploy-agent invocation" "orchestrator routing")` -- human-readable step names for output.

### 3. Global state variables

- `NO_CLEANUP=false` -- toggled by `--no-cleanup` flag.
- `TIMEOUT=$DEFAULT_TIMEOUT` -- overridden by `--timeout` flag.
- `START_TIME` -- captured via `date +%s` at script start for elapsed time tracking.
- `STEP_TIMES=()` -- array accumulating per-step elapsed seconds.

### 4. Function: `usage`

Print usage information and exit. Describes `--no-cleanup`, `--timeout`, and `--help` flags.

### 5. Function: `parse_args`

Loop over `"$@"`, handling:
- `--no-cleanup` -- set `NO_CLEANUP=true`.
- `--timeout` -- consume the next argument as `TIMEOUT`; validate it is a positive integer.
- `--help` / `-h` -- call `usage`.
- Any unknown flag -- print error and call `usage`.

### 6. Function: `log_info`

Print a timestamped info message to stdout. Format: `[YYYY-MM-DDTHH:MM:SS] INFO: <message>`.

### 7. Function: `log_error`

Print a timestamped error message to stderr. Format: `[YYYY-MM-DDTHH:MM:SS] ERROR: <message>`.

### 8. Function: `elapsed_since`

Accept a start timestamp as argument. Compute and print `$(( $(date +%s) - $1 ))`.

### 9. Function: `check_timeout`

Compare `elapsed_since "$START_TIME"` against `$TIMEOUT`. If exceeded, call `log_error` with a timeout message and `exit 1`. The trap will handle cleanup.

### 10. Function: `cleanup`

Called by the EXIT trap. If `NO_CLEANUP` is true, print a message that cleanup is skipped and the environment is still running, then return. Otherwise, run `docker compose -f "$COMPOSE_FILE" down -v --remove-orphans 2>/dev/null || true`. Print total elapsed time.

### 11. Function: `dump_container_logs`

Run `docker compose -f "$COMPOSE_FILE" logs --no-color` and write output to `$ARTIFACTS_DIR/container-logs.txt`. Also print a message indicating where logs were saved. This is called on step failure before exiting.

### 12. Function: `wait_for_health`

Accept a service name and host port as arguments. Implements exponential backoff health polling:
- Initialize `interval=$HEALTH_INITIAL_INTERVAL` and `elapsed=0`.
- Loop while `elapsed < HEALTH_MAX_WAIT`:
  - `check_timeout` (enforce overall timeout).
  - Attempt `curl -sf "http://localhost:${port}/health"` and capture the response.
  - Parse the response with `jq -r '.status'`.
  - If status is `"Healthy"`, return 0.
  - If status is `"Degraded"`, log a warning but continue waiting.
  - Sleep `$interval` seconds.
  - Update `elapsed` and double `interval` (cap at 16 seconds).
- If the loop exits without a healthy response, return 1.

### 13. Function: `wait_for_all_services`

Iterate over `SERVICES` array. For each service, call `wait_for_health "$service" "${HOST_PORTS[$service]}"`. If any service fails to become healthy within the timeout, call `log_error`, `dump_container_logs`, and `exit 1`. Log each service as it becomes healthy.

### 14. Function: `run_step`

Accept a step index (0-based) as argument. Extract the script path from `STEPS[$index]` and the human-readable name from `STEP_NAMES[$index]`.
- `check_timeout`.
- Log that the step is starting.
- Record `step_start=$(date +%s)`.
- Run `bash "${STEPS[$index]}"` capturing exit code.
- Record `step_elapsed=$(elapsed_since "$step_start")` and append to `STEP_TIMES`.
- If exit code is non-zero: log the failure with step name and elapsed time, call `dump_container_logs`, and `exit 1`.
- If exit code is zero: log success with step name and elapsed time.

### 15. Function: `print_summary`

Print a formatted summary table showing each step name, its pass/fail status, and elapsed time. Print total elapsed time. Example output:

```
========================================
  E2E Pipeline Results
========================================
  Step 1: skill-writer invocation   PASS  (12s)
  Step 2: tool-coder invocation     PASS  (45s)
  Step 3: deploy-agent invocation   PASS  (180s)
  Step 4: orchestrator routing      PASS  (8s)
----------------------------------------
  Total elapsed: 245s
  Status: ALL STEPS PASSED
========================================
```

### 16. Function: `check_prerequisites`

Verify that required tools are installed before doing any work:
- `curl` -- for health check polling.
- `jq` -- for JSON parsing (used by health check and step validators).
- `docker` -- for container management.
- `docker compose` -- for multi-container orchestration (verify via `docker compose version`).

For each missing tool, print a clear error message and exit non-zero.

### 17. Main execution flow

```
parse_args "$@"
START_TIME=$(date +%s)
trap cleanup EXIT

log_info "Starting E2E pipeline test (timeout=${TIMEOUT}s)"

# Ensure we are in the project root (where docker-compose.e2e.yml lives)
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
cd "$PROJECT_ROOT"

check_prerequisites

# Create artifacts directory
mkdir -p "$ARTIFACTS_DIR"

# Start docker-compose environment
log_info "Building and starting services..."
docker compose -f "$COMPOSE_FILE" up -d --build 2>&1 | tee "$ARTIFACTS_DIR/compose-up.log"

# Wait for all services to become healthy
log_info "Waiting for services to become healthy..."
wait_for_all_services

# Run step validators sequentially
for i in "${!STEPS[@]}"; do
    run_step "$i"
done

# All steps passed
print_summary
log_info "E2E pipeline test PASSED"
```

### Error handling and edge cases

- **Trap ordering**: The `cleanup` function is registered as an EXIT trap before any docker-compose commands. This ensures cleanup runs even if the script is interrupted with SIGINT/SIGTERM (since EXIT traps fire on those signals in Bash).
- **Timeout enforcement**: `check_timeout` is called at the start of each health poll iteration and before each step. This provides granular timeout checks without a background watchdog process.
- **Partial artifacts**: If step 2 fails, artifacts from step 1 are already saved in `tests/e2e/artifacts/` and remain available. The `dump_container_logs` call on failure adds container logs to the same directory.
- **Docker compose version**: Use `docker compose` (v2 plugin syntax), not `docker-compose` (v1 standalone). The v2 syntax is the modern standard.
- **Port availability**: If host-mapped ports are already in use, `docker compose up` will fail. The error will propagate through `set -e` and trigger the cleanup trap. No special handling needed beyond the compose error message.
- **Non-TTY output**: Use `--no-color` for docker compose logs to ensure clean output in CI environments. Do not use ANSI escape codes in `log_info`/`log_error` -- keep output plain text for portability.
- **`set -e` and step exit codes**: The `run_step` function must capture the exit code of the validator script without letting `set -e` terminate the script prematurely. Use `bash "${STEPS[$index]}" || step_exit=$?` pattern or temporarily disable errexit.

### Files to modify

None. This is a new file.

### Integration points

- Reads `docker-compose.e2e.yml` from the project root (created by the "Create docker-compose.e2e.yml" task).
- Invokes `tests/e2e/validate_step{1-4}_*.sh` (created by the four step validator tasks).
- Writes artifacts to `tests/e2e/artifacts/` which are read by the Rust integration test wrapper for post-mortem debugging.
- The Rust integration test wrapper (`tests/e2e_bootstrap_test.rs`) calls this script via `Command::new("bash").arg("scripts/e2e-test.sh")` and asserts exit code 0.

## Dependencies

- Blocked by: "Create `docker-compose.e2e.yml`", "Write step 1 validator: skill-writer invocation", "Write step 2 validator: tool-coder invocation", "Write step 3 validator: deploy-agent invocation", "Write step 4 validator: orchestrator routing"
- Blocking: "Add Rust integration test wrapper"

## Risks & Edge Cases

- **Docker socket availability**: The script assumes Docker is running and the current user has permission to use it. In CI, this may require Docker-in-Docker or a privileged runner. The preflight check will catch a missing `docker` binary but not permission issues -- those will surface as compose errors.
- **Port conflicts**: Host-mapped ports (8081-8084) may conflict with other services. The compose file should use unique ports, but the script cannot detect pre-existing conflicts before `docker compose up` fails.
- **Build time**: `docker compose up --build` may take several minutes for the Rust compilation. This counts against the overall timeout. The default 600s (10 minutes) should be sufficient, but slow CI runners may need `--timeout 900` or higher.
- **Health check false positives**: A service might return `{"status":"Healthy"}` briefly before crashing. The step validators will catch this when their HTTP calls fail, but the error message may be confusing. Container logs in artifacts will clarify.
- **LLM API key availability**: The step validators invoke LLM-backed agents. If API keys are not set in the environment or docker-compose file, the agents will fail. This is outside the script's scope but should be documented in the README section.
- **Disk space**: Docker image builds and container logs can consume significant disk space. The script does not manage disk space -- this is an operational concern for the CI environment.
- **Signal handling**: If the user sends SIGTERM or SIGINT, the EXIT trap fires and runs `cleanup`. However, if `docker compose down` itself hangs, the script may not terminate cleanly. Adding a timeout to the cleanup `docker compose down` command (e.g., `timeout 30 docker compose ...`) would mitigate this.

## Verification

- `bash -n scripts/e2e-test.sh` passes (syntax check, no execution).
- `shellcheck scripts/e2e-test.sh` produces no errors (if shellcheck is available).
- `ls -la scripts/e2e-test.sh` shows the executable bit is set.
- `./scripts/e2e-test.sh --help` prints usage and exits 0.
- `./scripts/e2e-test.sh --timeout abc` exits non-zero with a clear error about invalid timeout value.
- `docker compose -f docker-compose.e2e.yml config` validates the compose file syntax (dry run, does not start services).
- `cargo check` and `cargo test` pass (no Rust changes in this task).
- Full E2E run is NOT verified in this task (requires LLM API keys and Docker) -- only syntax and flag parsing are verified.
