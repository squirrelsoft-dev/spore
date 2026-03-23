# Spec: Write step 2 validator — tool-coder invocation

> From: .claude/tasks/issue-22.md

## Objective

Create `tests/e2e/validate_step2_tools.sh`, a shell script that validates the tool-coder pipeline stage. The script sends a previously generated skill file to the tool-coder agent's `/invoke` endpoint, validates that the response indicates successful compilation with non-empty tool output and valid implementation paths, and supports up to 3 retry attempts to account for LLM non-determinism.

## Current State

### Agent HTTP interface (`crates/agent-runtime/src/http.rs`)

The agent-runtime exposes two endpoints:

- `POST /invoke` — accepts `Json<AgentRequest>`, returns `Json<AgentResponse>` on success or a JSON-serialized `AgentError` on failure.
- `GET /health` — returns `Json<HealthResponse>` with `name`, `version`, and `status` fields.

### AgentRequest schema (`crates/agent-sdk/src/agent_request.rs`)

```json
{
  "id": "<uuid>",
  "input": "<string>",
  "context": null | <json-value>,
  "caller": null | "<string>"
}
```

- `id` — UUID v4 identifying the request.
- `input` — the primary input string (for step 2, this is the full contents of the generated skill file).
- `context` — optional JSON value for additional context; may be `null`.
- `caller` — optional string identifying the calling agent; may be `null`.

### AgentResponse schema (`crates/agent-sdk/src/agent_response.rs`)

```json
{
  "id": "<uuid>",
  "output": <json-value>,
  "confidence": <float>,
  "escalated": <bool>,
  "escalate_to": null | "<string>",
  "tool_calls": [...]
}
```

- `output` — a `serde_json::Value` containing the agent's structured output.
- `confidence` — float between 0.0 and 1.0.
- `escalated` — boolean indicating whether the agent escalated.

### tool-coder output schema (`skills/tool-coder.md`)

The tool-coder agent returns structured JSON in the `output` field with three string fields:

- `tools_generated` — comma-separated list of tool names that were generated (e.g., `"convert_temperature"`). Empty string if no tools were needed.
- `compilation_result` — summary of build outcome; contains `"success"` if all tools compiled, or compiler error output on failure.
- `implementation_paths` — comma-separated list of filesystem paths where tool crates were written (e.g., `"tools/convert-temperature"`).

### Artifact dependency

Step 1 (`validate_step1_skill.sh`) saves the generated skill file to `tests/e2e/artifacts/generated-skill.md`. This script reads that file as its input.

### E2E test architecture

The E2E shell script orchestrator (`scripts/e2e-test.sh`) calls each step validator in sequence. Each validator is expected to:
- Exit 0 on success, non-zero on failure.
- Print diagnostic messages to stdout/stderr explaining pass/fail.
- Save its response artifacts for downstream steps and debugging.

## Requirements

### Script behavior

- The script must be a Bash script (`#!/usr/bin/env bash`) with `set -euo pipefail`.
- Read the generated skill file from `tests/e2e/artifacts/generated-skill.md`. Exit with error if the file does not exist or is empty.
- Construct an `AgentRequest` JSON payload with:
  - `id` — a generated UUID v4 (use `uuidgen` or a fallback method).
  - `input` — the full contents of the generated skill file.
  - `context` — `null`.
  - `caller` — `"e2e-test"`.
- Send `POST` to `${TOOL_CODER_URL:-http://tool-coder:8080}/invoke` with `Content-Type: application/json`. Use an environment variable for the base URL so the script works both inside docker-compose (default) and in local testing.
- Use a generous timeout for the HTTP request (at least 120 seconds per attempt) since the tool-coder must generate code, write files, and run `cargo build`.
- On HTTP-level failure (non-2xx status or connection error), treat the attempt as failed.

### Retry logic

- Allow up to 3 attempts total (configurable via `${MAX_RETRIES:-3}`).
- On each failed attempt, print the attempt number, failure reason, and the response body (if available).
- Wait between retries with a short backoff (e.g., 5 seconds between attempts).
- If all attempts fail, exit non-zero with a summary of all failures.

### Response validation

After receiving a 200 response, validate the following using `jq`:

1. **HTTP status**: The response status code is 200.
2. **`output.compilation_result` contains "success"**: Extract `.output.compilation_result` from the response JSON and check that it contains the substring `success` (case-insensitive). This accounts for variations like `"success"`, `"Build success"`, `"compilation success"`, etc.
3. **`output.tools_generated` is non-empty**: Extract `.output.tools_generated` and verify it is a non-empty string (not `""`, not `null`).
4. **`output.implementation_paths` lists valid paths**: Extract `.output.implementation_paths` and verify:
   - It is a non-empty string (not `""`, not `null`).
   - Each comma-separated entry starts with `tools/` (validating they are under the expected directory).

If any validation check fails, the attempt counts as a failure (triggering a retry if attempts remain).

### Artifact output

- Save the raw response JSON to `tests/e2e/artifacts/step2-response.json` on each attempt (overwriting previous attempt).
- On success, the final saved response is the successful one.

### Output messages

- Print a clear header at the start: `"=== Step 2: Tool-coder invocation ==="`.
- On each attempt, print `"Attempt N/3..."`.
- On validation failure, print which specific check failed and the actual value.
- On final success, print `"Step 2 PASSED: tools generated and compiled successfully"` along with the tool names from `tools_generated`.
- On final failure (all retries exhausted), print `"Step 2 FAILED: tool-coder did not produce valid output after N attempts"` and exit with code 1.

## Implementation Details

### File to create

**`tests/e2e/validate_step2_tools.sh`**

The script structure should follow this outline:

1. **Header and setup** — shebang, `set -euo pipefail`, define constants (`ARTIFACTS_DIR`, `SKILL_FILE`, `RESPONSE_FILE`, `MAX_RETRIES`, `TOOL_CODER_URL`, `REQUEST_TIMEOUT`).

2. **`generate_uuid` helper** — produce a UUID v4 using `uuidgen` if available, falling back to reading from `/proc/sys/kernel/random/uuid`, or constructing from random hex as a last resort. Keep this under 10 lines.

3. **`validate_response` function** — accepts a file path to the response JSON, runs all three validation checks using `jq`, returns 0 on success or 1 on failure. Print which check failed. Keep this under 40 lines. Checks:
   - Extract `compilation_result` from `.output.compilation_result`, verify it matches `success` (case-insensitive grep or jq `test`).
   - Extract `tools_generated` from `.output.tools_generated`, verify non-empty and non-null.
   - Extract `implementation_paths` from `.output.implementation_paths`, verify non-empty and that all comma-separated entries start with `tools/`.

4. **Input validation** — verify `SKILL_FILE` exists and is non-empty.

5. **Retry loop** — `for attempt in $(seq 1 "$MAX_RETRIES"); do ... done`. Each iteration:
   - Build the JSON payload using `jq -n` with `--arg` for `input` (read from skill file) and `id` (from `generate_uuid`).
   - Call `curl -s -w '\n%{http_code}' --max-time "$REQUEST_TIMEOUT" -X POST ...` to capture both the response body and HTTP status code.
   - Parse out the HTTP status code from the last line.
   - If status is not 200, print error and continue to next attempt.
   - Save the response body to `RESPONSE_FILE`.
   - Call `validate_response "$RESPONSE_FILE"`. If it returns 0, print success and exit 0.
   - Otherwise, print failure details and sleep before the next attempt.

6. **Final failure** — if the loop completes without success, print failure summary and `exit 1`.

### Dependencies (CLI tools)

- `bash` (4.0+)
- `curl` — for HTTP requests
- `jq` — for JSON construction and parsing
- `uuidgen` or `/proc/sys/kernel/random/uuid` — for UUID generation

No additional tools or dependencies are required.

### Environment variables

| Variable | Default | Description |
|---|---|---|
| `TOOL_CODER_URL` | `http://tool-coder:8080` | Base URL of the tool-coder service |
| `MAX_RETRIES` | `3` | Maximum number of attempts |
| `REQUEST_TIMEOUT` | `120` | Curl timeout in seconds per attempt |
| `ARTIFACTS_DIR` | `tests/e2e/artifacts` | Directory for input/output artifacts |

## Dependencies

- Blocked by: "Define the test scenario document" (provides the test scenario reference and establishes the artifact directory structure)
- Blocking: "Write the E2E shell script orchestrator" (which calls this script as step 2 in the pipeline)

## Risks & Edge Cases

- **tool-coder `output` field structure**: The tool-coder output schema defines `tools_generated`, `compilation_result`, and `implementation_paths` as strings, but the `AgentResponse.output` field is `serde_json::Value`. The tool-coder agent should produce a JSON object at `.output` with these three string fields. If the agent wraps them differently (e.g., nested under another key, or returns the whole thing as a string), the `jq` extraction paths will need adjustment. The validation should log the raw `.output` value on failure to aid debugging.

- **LLM non-determinism in `compilation_result`**: The tool-coder prompt says to include `"success"` if all tools compiled, but the exact wording may vary (e.g., `"Success"`, `"Build succeeded"`, `"All tools compiled successfully"`). The validation uses case-insensitive substring matching to handle this.

- **Empty `tools_generated` when tools already exist**: The tool-coder checks the tool-registry for existing tools and only generates missing ones. If the test scenario's tool (`convert_temperature`) already exists in the registry, `tools_generated` could be empty and `implementation_paths` could be empty — which would fail validation. This is unlikely in a clean E2E environment but worth noting. The test scenario document should clarify that the E2E environment starts with a clean state.

- **Large skill file in JSON payload**: The skill file contents are embedded as a string in the JSON `input` field. Special characters (quotes, backslashes, newlines) must be properly escaped. Using `jq --arg` for payload construction handles this automatically.

- **Curl timeout**: Tool-coder may take a long time if it needs multiple compile-fix cycles (up to 15 turns per its `max_turns` constraint). The 120-second timeout should be sufficient, but if the agent is slow due to LLM latency, retries provide additional runway. Total worst-case wall time: 3 attempts x 120s = 360s (6 minutes).

- **`implementation_paths` format**: The spec says paths are comma-separated strings like `"tools/read-file, tools/write-file"`. Validation should handle optional whitespace around commas when splitting.

## Verification

- `bash -n tests/e2e/validate_step2_tools.sh` succeeds (syntax check).
- `shellcheck tests/e2e/validate_step2_tools.sh` produces no errors (if shellcheck is available).
- The script is executable (`chmod +x`).
- Manual review confirms:
  - All three validation checks are implemented (compilation_result, tools_generated, implementation_paths).
  - Retry logic attempts up to 3 times with sleep between attempts.
  - The script reads from `tests/e2e/artifacts/generated-skill.md` and writes to `tests/e2e/artifacts/step2-response.json`.
  - Environment variables are used for configurable values with sensible defaults.
  - Diagnostic output clearly identifies which check failed and the actual value received.
  - No functions exceed 50 lines.
  - No hardcoded URLs (uses `TOOL_CODER_URL` env var).
