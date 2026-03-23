# Spec: Write step 3 validator — deploy-agent invocation

> From: .claude/tasks/issue-22.md

## Objective

Create `tests/e2e/validate_step3_deploy.sh`, a shell script that sends the skill file path and tool implementation paths from steps 1-2 to the deploy-agent's `/invoke` endpoint, then validates the structured JSON response contains a correctly formatted image URI, a valid HTTP endpoint URL, and a healthy health-check status. The script uses generous timeouts to accommodate slow Docker builds.

## Current State

### Deploy-agent output schema (`skills/deploy-agent.md`)

The deploy-agent returns structured JSON with three fields:

- `image_uri` (string): Full image reference pushed to the registry, following the convention `{REGISTRY_URL}/spore-{agent-name}:{version}` (e.g., `ghcr.io/spore/spore-deploy-agent:0.1`).
- `endpoint_url` (string): HTTP address where the deployed agent is reachable (e.g., `http://deploy-agent:8080`).
- `health_check` (string): `"healthy"` if the agent responded with 200 OK to a post-deployment health verification, or an error description if the health check failed.

### HTTP API (`crates/agent-runtime/src/http.rs`)

- `POST /invoke` accepts `Json<AgentRequest>` and returns `Json<AgentResponse>` on success or a JSON-serialized `AgentError` on failure.
- `GET /health` returns `Json<HealthResponse>` with fields `name`, `version`, `status`.

### AgentRequest (`crates/agent-sdk/src/agent_request.rs`)

```json
{
  "id": "<uuid>",
  "input": "<string>",
  "context": <optional json value>,
  "caller": <optional string>
}
```

### AgentResponse (`crates/agent-sdk/src/agent_response.rs`)

```json
{
  "id": "<uuid>",
  "output": <json value>,
  "confidence": <float>,
  "escalated": <bool>,
  "escalate_to": <optional string>,
  "tool_calls": [<ToolCallRecord>, ...]
}
```

The deploy-agent's structured output lives inside the `output` field of `AgentResponse`. Based on the skill manifest's output schema, `output` is expected to be a JSON object with keys `image_uri`, `endpoint_url`, and `health_check`.

### Artifacts from prior steps

- `tests/e2e/artifacts/generated-skill.md` — skill file produced by step 1 (skill-writer).
- `tests/e2e/artifacts/step2-response.json` — response from step 2 (tool-coder), containing `output.implementation_paths` with paths to generated tool code.

### Image tagging convention (`skills/deploy-agent.md`)

Images are tagged as `spore-{agent-name}:{version}` where `agent-name` and `version` come from the skill manifest YAML frontmatter. The full URI includes the registry prefix: `{REGISTRY_URL}/spore-{agent-name}:{version}`.

## Requirements

### Script behavior

- The script is a standalone bash script at `tests/e2e/validate_step3_deploy.sh` with a `#!/usr/bin/env bash` shebang and `set -euo pipefail`.
- It reads the generated skill file from `tests/e2e/artifacts/generated-skill.md` and the step 2 response from `tests/e2e/artifacts/step2-response.json`.
- It constructs an `AgentRequest` JSON body with:
  - `id`: a freshly generated UUID (via `uuidgen` or equivalent).
  - `input`: a string describing the deployment task, including the skill file path and the tool implementation paths extracted from the step 2 response. For example: `"Deploy the agent defined by skill file at /path/to/generated-skill.md with tools at /path/to/tool1, /path/to/tool2"`.
  - `context`: an optional JSON object containing `skill_path` and `tool_paths` fields for structured access.
  - `caller`: `"e2e-test"`.
- It sends a `POST` to `http://deploy-agent:8080/invoke` with the constructed request body.
- The deploy-agent host is configurable via an environment variable `DEPLOY_AGENT_HOST` (default: `deploy-agent:8080`).
- The `curl` timeout is set to at least 300 seconds (5 minutes) to allow for Docker image builds. Use `--max-time 600` (10 minutes) as the outer bound and `--connect-timeout 30` for connection establishment.
- On success (HTTP 200), the response is saved to `tests/e2e/artifacts/step3-response.json`.

### Validation checks

All validations operate on the `output` field of the `AgentResponse`. The script must perform the following checks, exiting non-zero with a diagnostic message on any failure:

1. **HTTP status**: The response HTTP status code is 200. Any other status is a failure.
2. **Response structure**: The response body is valid JSON and contains an `output` field that is a JSON object.
3. **`image_uri` is non-empty**: `output.image_uri` exists, is a string, and is not empty.
4. **`image_uri` follows naming convention**: `output.image_uri` matches the regex pattern `spore-[a-z0-9-]+:[0-9a-z.-]+` (the `spore-{name}:{version}` suffix). The full URI may contain a registry prefix (e.g., `ghcr.io/spore/`), so the check validates that the URI ends with or contains this pattern. Use a regex like `spore-[a-z][a-z0-9-]*:[0-9]+(\.[0-9]+)*`.
5. **`endpoint_url` is non-empty**: `output.endpoint_url` exists, is a string, and is not empty.
6. **`endpoint_url` is a valid HTTP URL**: `output.endpoint_url` starts with `http://` or `https://`. Validate with a regex or string prefix check.
7. **`health_check` is "healthy"**: `output.health_check` exists, is a string, and equals `"healthy"` (exact match, case-sensitive).

### Error handling

- If `tests/e2e/artifacts/generated-skill.md` does not exist, exit with code 1 and a message indicating step 1 must complete first.
- If `tests/e2e/artifacts/step2-response.json` does not exist, exit with code 1 and a message indicating step 2 must complete first.
- If `curl` fails (network error, timeout), exit with code 1 and print the curl error.
- If any validation check fails, print which check failed, the actual value received, and exit with code 1.
- On non-200 HTTP responses, print the status code and response body for diagnostics.

### Output

- On success, print a summary: `"Step 3 PASSED: deploy-agent produced image_uri=<uri>, endpoint_url=<url>, health_check=healthy"`.
- On failure, print: `"Step 3 FAILED: <reason>"` with the specific check that failed and the actual value.
- Save the full response to `tests/e2e/artifacts/step3-response.json` regardless of validation outcome (as long as a response was received).

## Implementation Details

### Files to create

**`tests/e2e/validate_step3_deploy.sh`**

The script follows this structure:

1. **Setup and configuration**:
   - Shebang: `#!/usr/bin/env bash`
   - `set -euo pipefail`
   - Define `DEPLOY_AGENT_HOST` from environment or default to `deploy-agent:8080`.
   - Define `ARTIFACTS_DIR` as the directory containing step artifacts (default: `tests/e2e/artifacts`).
   - Define `INVOKE_URL="http://${DEPLOY_AGENT_HOST}/invoke"`.
   - Define curl timeout constants: `CONNECT_TIMEOUT=30`, `MAX_TIME=600`.

2. **Pre-flight checks**:
   - Verify `tests/e2e/artifacts/generated-skill.md` exists. Exit 1 if missing.
   - Verify `tests/e2e/artifacts/step2-response.json` exists. Exit 1 if missing.
   - Verify `curl` and `jq` are available on `PATH`.

3. **Extract inputs from prior steps**:
   - Read the skill file path: `SKILL_PATH="${ARTIFACTS_DIR}/generated-skill.md"`.
   - Extract tool implementation paths from step 2 response using `jq`: `TOOL_PATHS=$(jq -r '.output.implementation_paths // [] | join(", ")' "${ARTIFACTS_DIR}/step2-response.json")`.

4. **Construct the request body**:
   - Generate a UUID for the request `id` (use `uuidgen` if available, fall back to a `/proc/sys/kernel/random/uuid` read, or construct one via other means).
   - Build the JSON request using `jq -n` with `--arg` flags to safely interpolate the skill path and tool paths into the `input` string and `context` object.

5. **Send the request**:
   - Use `curl -s -w '\n%{http_code}' --connect-timeout ${CONNECT_TIMEOUT} --max-time ${MAX_TIME} -H 'Content-Type: application/json' -d "${REQUEST_BODY}" "${INVOKE_URL}"`.
   - Separate the HTTP body from the status code (last line of output).

6. **Save response**:
   - Write the response body to `tests/e2e/artifacts/step3-response.json`.

7. **Validate HTTP status**:
   - Check status code is `200`. If not, print diagnostics and exit 1.

8. **Validate response fields** (using `jq`):
   - Check `output` field exists and is an object.
   - Check `output.image_uri` is a non-empty string.
   - Check `output.image_uri` matches the `spore-` naming pattern using `jq test()` or a bash regex.
   - Check `output.endpoint_url` is a non-empty string.
   - Check `output.endpoint_url` starts with `http://` or `https://`.
   - Check `output.health_check` equals `"healthy"`.

9. **Print summary**:
   - On all checks passing, print success message and exit 0.

### Helper function pattern

Use a `validate` helper to keep each check concise:

```bash
validate() {
    local description="$1"
    local actual="$2"
    local condition="$3"
    if ! eval "$condition"; then
        echo "Step 3 FAILED: ${description} (got: ${actual})"
        exit 1
    fi
}
```

Or use individual `jq` exit-code checks, printing diagnostics inline.

### Files to modify

None. This is a new file.

## Dependencies

- Blocked by: "Define the test scenario document" (provides the agreed-upon test scenario and success criteria).
- Blocking: "Write the E2E shell script orchestrator" (the top-level driver runs this script as step 3).

## Risks & Edge Cases

- **Docker build timeouts**: Docker image builds inside the deploy-agent can take several minutes, especially on cold caches. The 10-minute `--max-time` should handle this, but CI environments with limited resources may need more. The timeout constants are defined as variables for easy adjustment.
- **Registry URL variability**: The `image_uri` validation checks for the `spore-{name}:{version}` pattern anywhere in the string, not at a fixed position, because the registry prefix varies by environment (could be `localhost:5000/`, `ghcr.io/spore/`, or absent entirely).
- **LLM non-determinism**: The deploy-agent is LLM-driven, so the exact format of `image_uri` and `endpoint_url` may vary. Validations check structural correctness (pattern matching, URL prefix), not exact values.
- **Step 2 response format**: The script depends on `output.implementation_paths` being an array of strings in the step 2 response. If the tool-coder returns a different structure, the `jq` extraction will yield an empty string, which is still a valid (if less informative) input to the deploy-agent.
- **UUID generation**: Different environments may or may not have `uuidgen`. The script should have a fallback (e.g., reading from `/proc/sys/kernel/random/uuid` on Linux, or using Python/jq to generate one).
- **`jq` version compatibility**: The `jq` `test()` function for regex matching requires jq 1.5+. This is widely available but worth noting. If compatibility is a concern, fall back to bash `[[ =~ ]]` for regex checks.
- **Network DNS resolution**: Inside the docker-compose network, `deploy-agent` resolves to the container. Outside docker-compose (e.g., local testing), the `DEPLOY_AGENT_HOST` override allows pointing to `localhost:<mapped-port>`.

## Verification

- `bash -n tests/e2e/validate_step3_deploy.sh` produces no syntax errors.
- `shellcheck tests/e2e/validate_step3_deploy.sh` produces no errors (warnings acceptable for intentional patterns).
- The script is executable (`chmod +x`).
- The script exits 1 with a clear message when `tests/e2e/artifacts/generated-skill.md` does not exist.
- The script exits 1 with a clear message when `tests/e2e/artifacts/step2-response.json` does not exist.
- The script exits 1 with a clear message when `curl` or `jq` is not found.
- All functions in the script are under 50 lines per project rules.
- No hardcoded host/port values; the deploy-agent endpoint is configurable via `DEPLOY_AGENT_HOST`.
