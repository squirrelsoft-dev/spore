# Spec: Write step 4 validator — orchestrator routing

> From: .claude/tasks/issue-22.md

## Objective

Create `tests/e2e/validate_step4_route.sh`, a shell script that sends domain-level temperature-conversion requests through the orchestrator's `/invoke` endpoint and validates that the orchestrator correctly routes them to the deployed temperature-conversion agent, which returns numerically reasonable results with high confidence and non-empty tool usage.

## Current State

### Endpoint contract (`crates/agent-runtime/src/http.rs`)

The orchestrator exposes `POST /invoke` accepting an `AgentRequest` JSON body and returning an `AgentResponse` JSON body.

**`AgentRequest`** (`crates/agent-sdk/src/agent_request.rs`):
```
{
  "id": "<uuid>",
  "input": "<string>",
  "context": <optional json value>,
  "caller": <optional string>
}
```

**`AgentResponse`** (`crates/agent-sdk/src/agent_response.rs`):
```
{
  "id": "<uuid>",
  "output": <json value>,
  "confidence": <f32>,
  "escalated": <bool>,
  "escalate_to": <optional string>,
  "tool_calls": [
    {
      "tool_name": "<string>",
      "input": <json value>,
      "output": <json value>
    }
  ]
}
```

- `confidence` is an `f32` in `[0.0, 1.0]`.
- `escalate_to` is omitted from serialization when `None` (`skip_serializing_if`).
- `tool_calls` is a `Vec<ToolCallRecord>` where each record has `tool_name`, `input`, and `output` fields (all JSON values except `tool_name` which is a string).

### Orchestrator routing (`crates/orchestrator/src/orchestrator.rs`)

The orchestrator implements `MicroAgent::invoke` by calling `self.dispatch(request)`. Dispatch performs routing via:
1. **`target_agent` context field** — if the request `context` contains `{"target_agent": "<name>"}`, route directly to that agent.
2. **Semantic routing** — if an embedding model is configured, match the request input against agent descriptions by cosine similarity.
3. **NoRoute error** — if neither method matches, return `OrchestratorError::NoRoute`.

After routing, the orchestrator calls `try_invoke` (which checks agent health before forwarding) and then handles any escalation chain. The orchestrator skill manifest (`skills/orchestrator.md`) declares a confidence threshold of 0.9 and max turns of 3.

### Error mapping (`crates/agent-runtime/src/http.rs`)

- `AgentError::ConfidenceTooLow` returns HTTP 200 (not a server error).
- `AgentError::Internal` returns HTTP 500.
- `AgentError::ToolCallFailed` returns HTTP 502.
- Successful invocations return HTTP 200 with `AgentResponse` JSON body.

### Artifact convention

Prior steps save outputs to `tests/e2e/artifacts/`. This script saves its responses to `tests/e2e/artifacts/step4-response.json` for downstream consumption.

## Requirements

- The script must be a Bash script (`#!/usr/bin/env bash`) with `set -euo pipefail`.
- Accept the orchestrator base URL via an environment variable `ORCHESTRATOR_URL` with a default of `http://orchestrator:8080` (matching docker-compose service name).
- Accept an artifacts directory via `ARTIFACTS_DIR` with a default of `tests/e2e/artifacts`.

### Test case 1: Convert 100F to C

1. Construct an `AgentRequest` JSON body with:
   - `id`: a generated UUID (use `uuidgen` or a hardcoded test UUID).
   - `input`: `"Convert 100 degrees Fahrenheit to Celsius"`
   - `context`: `null` (let the orchestrator route semantically).
   - `caller`: `null`.
2. Send `POST ${ORCHESTRATOR_URL}/invoke` with `Content-Type: application/json`.
3. Use a generous timeout (60 seconds) on the curl call to account for LLM latency.
4. Assert HTTP status code is 200.
5. Save the full response body to `${ARTIFACTS_DIR}/step4-response-q1.json`.
6. Parse the JSON response with `jq` and validate:
   - **Confidence**: `.confidence >= 0.8`. Use `jq` numeric comparison. Print the actual confidence value on failure.
   - **Numerically reasonable output**: Extract a numeric value from `.output` (which may be a JSON object or string depending on the agent's output format). The expected answer is approximately 37.78 (the exact conversion of 100F). Validate the extracted number is between 37.0 and 39.0 (inclusive) to allow for rounding and formatting differences. If `.output` is a string, use `grep -oE` or `jq` to extract a float with a regex pattern. If `.output` is an object, check common field names (`.output.result`, `.output.value`, `.output.answer`, `.output.temperature`, `.output.celsius`) for the numeric value.
   - **Non-empty tool_calls**: `.tool_calls | length > 0`. This proves the agent used its `convert_temperature` tool rather than answering from LLM knowledge alone.

### Test case 2: Convert 0K to C

1. Construct an `AgentRequest` with `input`: `"Convert 0 Kelvin to Celsius"`.
2. Send `POST ${ORCHESTRATOR_URL}/invoke`.
3. Assert HTTP 200.
4. Save the response to `${ARTIFACTS_DIR}/step4-response-q2.json`.
5. Validate:
   - **Confidence**: `.confidence >= 0.8`.
   - **Numerically reasonable output**: The expected answer is approximately -273.15. Validate the extracted number is between -274.0 and -273.0 (inclusive).
   - **Non-empty tool_calls**: `.tool_calls | length > 0`.

### Output and exit behavior

- Print a clear pass/fail message per test case and per assertion.
- On the first validation failure, print the failing assertion name, the expected range or condition, and the actual value, then exit with code 1.
- On success of all assertions, print a summary line (e.g., `"Step 4: All routing validations passed"`) and exit with code 0.
- Save a combined summary to `${ARTIFACTS_DIR}/step4-response.json` containing both query responses for downstream consumption.

## Implementation Details

### Files to create

**`tests/e2e/validate_step4_route.sh`**

The script structure:

1. **Header and configuration**:
   - Shebang: `#!/usr/bin/env bash`
   - `set -euo pipefail`
   - `ORCHESTRATOR_URL="${ORCHESTRATOR_URL:-http://orchestrator:8080}"`
   - `ARTIFACTS_DIR="${ARTIFACTS_DIR:-tests/e2e/artifacts}"`
   - `SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"`
   - Ensure `${ARTIFACTS_DIR}` directory exists (`mkdir -p`).

2. **Helper function `assert_http_ok`**:
   - Takes a label and a curl output file path.
   - Extracts the HTTP status code (use `curl -w '%{http_code}' -o <file>` pattern).
   - If status is not 200, print error with the label and status code, then `exit 1`.

3. **Helper function `extract_number`**:
   - Takes a `jq` path and a JSON file.
   - Attempts to extract a number from the response output field.
   - Strategy: first try `.output` as a direct number; then try common object fields (`.output.result`, `.output.value`, `.output.answer`, `.output.temperature`, `.output.celsius`); then try parsing `.output` as a string and extracting a float with a regex pattern like `-?[0-9]+\.?[0-9]*`.
   - Prints the extracted number to stdout. Prints an error and returns non-zero if no number is found.

4. **Helper function `assert_in_range`**:
   - Takes a label, actual value, min, and max.
   - Uses `awk` for floating-point comparison (bash cannot compare floats natively).
   - If out of range, print the label, expected range, and actual value, then `exit 1`.

5. **Helper function `assert_confidence`**:
   - Takes a label, a JSON file, and a minimum threshold (0.8).
   - Extracts `.confidence` with `jq`.
   - Uses `awk` for `>=` comparison.
   - Fails with a diagnostic message if below threshold.

6. **Helper function `assert_tool_calls_nonempty`**:
   - Takes a label and a JSON file.
   - Checks `.tool_calls | length` with `jq`.
   - Fails if length is 0.

7. **Test case 1 execution**:
   - Generate a UUID for the request (use `uuidgen` if available, otherwise a hardcoded UUID like `"00000000-0000-4000-a000-000000000001"`).
   - Build the JSON payload using `jq -n` with `--arg` for the input string and id.
   - Call `curl -s -w '\n%{http_code}' -X POST "${ORCHESTRATOR_URL}/invoke" -H 'Content-Type: application/json' -d "${PAYLOAD}" --max-time 60`.
   - Split response body from status code (last line).
   - Call `assert_http_ok`.
   - Save body to `${ARTIFACTS_DIR}/step4-response-q1.json`.
   - Call `assert_confidence "Q1" <file> 0.8`.
   - Extract number, call `assert_in_range "Q1: 100F->C" <number> 37.0 39.0`.
   - Call `assert_tool_calls_nonempty "Q1"`.
   - Print `"PASS: Query 1 (100F -> C)"`.

8. **Test case 2 execution**:
   - Same structure with input `"Convert 0 Kelvin to Celsius"`.
   - Save to `${ARTIFACTS_DIR}/step4-response-q2.json`.
   - Assert confidence >= 0.8.
   - Assert number in range -274.0 to -273.0.
   - Assert tool_calls non-empty.
   - Print `"PASS: Query 2 (0K -> C)"`.

9. **Summary**:
   - Combine both response files into `${ARTIFACTS_DIR}/step4-response.json` using `jq -n --slurpfile q1 <q1file> --slurpfile q2 <q2file> '{query1: $q1[0], query2: $q2[0]}'`.
   - Print `"Step 4: All routing validations passed"`.
   - Exit 0.

### Files to modify

None.

### Integration points

- Consumed by `scripts/e2e-test.sh` (Group 3), which calls this script after steps 1-3 have completed.
- Reads no artifacts from prior steps (the orchestrator routing is self-contained at this point since the temperature agent is already deployed and registered).
- Writes `step4-response-q1.json`, `step4-response-q2.json`, and `step4-response.json` to the artifacts directory.

### Tool dependencies

- `bash`, `curl`, `jq`, `awk` — all standard tools expected to be available in the E2E test environment.
- `uuidgen` — optional; falls back to a hardcoded UUID if not available.

## Dependencies

- Blocked by: "Define the test scenario document" (`tests/e2e/SCENARIO.md`) — provides the canonical test queries and expected outputs.
- Blocking: "Write the E2E shell script orchestrator" (`scripts/e2e-test.sh`) — calls this validator as step 4 in the pipeline.

## Risks & Edge Cases

- **LLM non-determinism in output format**: The temperature-conversion agent's `.output` field may be a plain number, a string like `"37.78 degrees Celsius"`, or a structured object like `{"celsius": 37.78}`. The `extract_number` helper must handle all three forms. The validation uses a wide numeric range (37.0-39.0 and -274.0 to -273.0) to account for rounding.
- **Orchestrator routing failure**: If semantic routing is not configured or the temperature agent is not registered, the orchestrator returns `AgentError::Internal("NoRoute { input: ... }")` with HTTP 500. The `assert_http_ok` check will catch this with a clear error message.
- **Agent health check delay**: The orchestrator calls `try_invoke` which checks agent health before forwarding. If the temperature agent is still starting, the health check may fail. The E2E orchestrator script (`scripts/e2e-test.sh`) is responsible for waiting until all services are healthy before running step 4. This script does not retry on its own.
- **Confidence threshold mismatch**: The orchestrator skill manifest declares `confidence_threshold: 0.9`, but the routed agent (temperature converter) controls its own response confidence. The validation threshold in this script (0.8) is intentionally lower than the orchestrator's routing threshold to avoid false negatives from minor confidence variations.
- **`escalate_to` field absence**: The `escalate_to` field uses `skip_serializing_if = "Option::is_none"`, so it may be entirely absent from the JSON response. The script does not check this field — it only validates confidence, output, and tool_calls.
- **Curl timeout**: A 60-second timeout per request should be sufficient for a single LLM call through the orchestrator, but in degraded environments it could be tight. The timeout is configurable by editing the script but is not exposed as a variable to keep the interface simple.
- **Floating-point extraction from strings**: The regex `-?[0-9]+\.?[0-9]*` may match unrelated numbers in verbose output (e.g., "100" from the input echoed in the output). The extraction strategy should prefer structured fields first and fall back to regex only as a last resort, prioritizing numbers that match the expected range.

## Verification

- `bash -n tests/e2e/validate_step4_route.sh` confirms no syntax errors.
- `shellcheck tests/e2e/validate_step4_route.sh` produces no errors (if shellcheck is available).
- The script is executable (`chmod +x`).
- Manual review confirms:
  - Both test cases (100F->C and 0K->C) are exercised.
  - Confidence threshold is 0.8.
  - Numeric ranges are 37.0-39.0 and -274.0 to -273.0.
  - Tool calls non-empty assertion is present for both queries.
  - Artifacts are saved to the expected paths.
  - Exit code is 1 on any failure, 0 on full success.
  - All helper functions are under 50 lines.
  - No hardcoded URLs (configurable via `ORCHESTRATOR_URL`).
