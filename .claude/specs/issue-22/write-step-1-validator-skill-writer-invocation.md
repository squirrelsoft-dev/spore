# Spec: Write step 1 validator — skill-writer invocation

> From: .claude/tasks/issue-22.md

## Objective

Create `tests/e2e/validate_step1_skill.sh`, a shell script that POSTs an `AgentRequest` to the skill-writer agent's `/invoke` endpoint, validates that the response contains a well-formed skill file (valid YAML frontmatter with all required `SkillManifest` fields, at least one tool, non-empty preamble), and saves the extracted skill file to `tests/e2e/artifacts/generated-skill.md` for use by downstream pipeline steps.

## Current State

### `/invoke` endpoint (`crates/agent-runtime/src/http.rs`)

The `POST /invoke` handler accepts a JSON `AgentRequest` body and returns a JSON `AgentResponse` (or an `AgentError` serialized as JSON with an appropriate HTTP status code).

### AgentRequest (`crates/agent-sdk/src/agent_request.rs`)

```rust
pub struct AgentRequest {
    pub id: Uuid,
    pub input: String,
    pub context: Option<Value>,
    pub caller: Option<String>,
}
```

### AgentResponse (`crates/agent-sdk/src/agent_response.rs`)

```rust
pub struct AgentResponse {
    pub id: Uuid,
    pub output: Value,
    pub confidence: f32,
    pub escalated: bool,
    pub escalate_to: Option<String>,
    pub tool_calls: Vec<ToolCallRecord>,
}
```

The `output` field is a `serde_json::Value`. For the skill-writer agent, this is a JSON object with two keys (per the skill-writer's `output.schema`):
- `skill_yaml`: `string` -- the complete skill file content in markdown-with-frontmatter format
- `validation_result`: `string` -- description of the validation outcome

### SkillManifest (`crates/agent-sdk/src/skill_manifest.rs`)

```rust
pub struct SkillManifest {
    pub name: String,
    pub version: String,
    pub description: String,
    pub model: ModelConfig,       // sub-fields: provider, name, temperature
    pub preamble: String,
    pub tools: Vec<String>,
    pub constraints: Constraints, // sub-fields: max_turns, confidence_threshold, escalate_to, allowed_actions
    pub output: OutputSchema,     // sub-fields: format, schema
}
```

The YAML frontmatter in a skill file contains all fields except `preamble`. The `preamble` is the markdown body after the closing `---` delimiter. Together they map to all 8 `SkillManifest` fields.

### Required YAML frontmatter fields (from skill file format)

The YAML block between `---` delimiters must contain these top-level keys:
- `name` (String, non-empty)
- `version` (String, must be quoted in YAML)
- `description` (String)
- `model` (object with `provider`, `name`, `temperature`)
- `tools` (list of strings, at least one entry)
- `constraints` (object with `max_turns`, `confidence_threshold`, `allowed_actions`, optional `escalate_to`)
- `output` (object with `format`, `schema`)

### Skill-writer output (`skills/skill-writer.md`)

The skill-writer agent returns structured JSON with `skill_yaml` (the complete markdown-with-frontmatter skill file) and `validation_result` (a description of validation outcomes). The `skill_yaml` value is a string containing the full skill file content, starting with `---` and including the YAML frontmatter, closing `---`, and the markdown preamble body.

### Test scenario (from `tests/e2e/SCENARIO.md`, defined by prerequisite)

The scenario uses a temperature-conversion agent. The natural language input is: "An agent that converts temperatures between Fahrenheit, Celsius, and Kelvin". The validator must not check for exact content (due to LLM non-determinism) but must validate structural correctness.

## Requirements

1. **Create `tests/e2e/validate_step1_skill.sh`** as an executable Bash script.

2. **Accept the skill-writer base URL** via the `SKILL_WRITER_URL` environment variable, defaulting to `http://skill-writer:8080` (the docker-compose service name). This allows running the script both inside the docker-compose network and against a manually started service.

3. **Construct an `AgentRequest` JSON body** with:
   - `id`: a valid UUID v4 (generate via `uuidgen` or a hardcoded test UUID)
   - `input`: the natural language description from the scenario ("An agent that converts temperatures between Fahrenheit, Celsius, and Kelvin")
   - `context`: `null`
   - `caller`: `"e2e-test"`

4. **POST to `${SKILL_WRITER_URL}/invoke`** using `curl`:
   - Set `Content-Type: application/json`
   - Use a generous timeout (120 seconds minimum) since the LLM call may be slow
   - Capture the HTTP status code and response body separately
   - Fail if the HTTP status is not 200

5. **Parse the `AgentResponse` JSON** using `jq`:
   - Extract `.output.skill_yaml` as the raw skill file content string
   - Fail if `.output.skill_yaml` is null or empty
   - Optionally log `.output.validation_result` for debugging

6. **Validate the extracted skill file** by parsing its structure. Validations (all structural, not content-exact):

   a. **YAML frontmatter is parseable**: The content starts with `---`, contains a second `---` delimiter, and the text between them is valid YAML. Use a lightweight YAML check (e.g., pipe to `python3 -c "import yaml; yaml.safe_load(open('/dev/stdin'))"` or use `yq` if available).

   b. **Required top-level YAML keys present**: The frontmatter YAML contains all 7 required keys: `name`, `version`, `description`, `model`, `tools`, `constraints`, `output`. Check using `yq` or equivalent YAML query tool.

   c. **`name` is non-empty**: The `name` field is a non-empty, non-whitespace-only string.

   d. **`version` is non-empty**: The `version` field is a non-empty string.

   e. **`model` has required sub-fields**: `model.provider` and `model.name` are present and non-empty.

   f. **`tools` has at least one entry**: The `tools` field is a YAML list with length >= 1.

   g. **`constraints` has required sub-fields**: `constraints.max_turns` and `constraints.confidence_threshold` are present. `constraints.allowed_actions` is a non-empty list.

   h. **`output` has required sub-fields**: `output.format` is present and is one of `json`, `structured_json`, or `text`. `output.schema` is present and non-empty.

   i. **Preamble is non-empty**: The markdown body after the closing `---` delimiter is non-empty and not whitespace-only.

7. **Save the skill file** to `tests/e2e/artifacts/generated-skill.md`. Create the `artifacts/` directory if it does not exist.

8. **Exit codes and output**:
   - Exit 0 on success, printing a summary of what was validated.
   - Exit non-zero on any validation failure, printing the specific failure reason to stderr.
   - Print the full `AgentResponse` JSON to a log file at `tests/e2e/artifacts/step1-response.json` for debugging.

9. **Script conventions**:
   - Use `set -euo pipefail` at the top.
   - Use `#!/usr/bin/env bash` as the shebang line.
   - Define a helper function for each validation check to keep the main flow readable.
   - Each helper function should print a check name, run the check, and print PASS/FAIL.
   - Keep functions under 50 lines per project rules.

## Implementation Details

### File to create

**`tests/e2e/validate_step1_skill.sh`**

The script follows this high-level flow:

1. **Setup**: Set `SKILL_WRITER_URL` default, define `ARTIFACTS_DIR` as `tests/e2e/artifacts`, create the artifacts directory with `mkdir -p`, define `SCRIPT_DIR` using `dirname` for path portability.

2. **Helper: `fail()`**: Print the message to stderr and exit 1. Used by all validation helpers.

3. **Helper: `check_http_response()`**: Run `curl` with `--silent --show-error --max-time 120 --write-out "%{http_code}"` to POST the `AgentRequest`. Capture HTTP status code and body. Fail if status is not `200`. Save response body to `${ARTIFACTS_DIR}/step1-response.json`.

4. **Helper: `extract_skill_yaml()`**: Use `jq -r '.output.skill_yaml'` to extract the skill file content from the response JSON. Fail if the result is null or empty. Save to `${ARTIFACTS_DIR}/generated-skill.md`.

5. **Helper: `extract_frontmatter()`**: Given the skill file content, extract the YAML between the first and second `---` delimiters. Use `sed` or `awk` to isolate the frontmatter block. Save to a temp file for subsequent YAML queries.

6. **Helper: `validate_yaml_parseable()`**: Attempt to parse the extracted frontmatter as YAML. Use `python3 -c "import sys, yaml; yaml.safe_load(sys.stdin)"` or `yq eval '.' -` to verify it is valid YAML. Fail with an error message if parsing fails.

7. **Helper: `validate_required_keys()`**: Check that each of the 7 required top-level keys (`name`, `version`, `description`, `model`, `tools`, `constraints`, `output`) exists in the YAML. Use `yq` to query each key and fail if any returns null.

8. **Helper: `validate_non_empty_strings()`**: Check that `name` and `version` are non-empty strings. Use `yq` to extract each value and test it is not empty or whitespace-only.

9. **Helper: `validate_model_config()`**: Check that `model.provider` and `model.name` exist and are non-empty.

10. **Helper: `validate_tools()`**: Check that `tools` is a list and has at least one entry. Use `yq eval '.tools | length'` and assert the result is >= 1.

11. **Helper: `validate_constraints()`**: Check that `constraints.max_turns`, `constraints.confidence_threshold`, and `constraints.allowed_actions` are present. Verify `allowed_actions` is a non-empty list.

12. **Helper: `validate_output_schema()`**: Check that `output.format` is one of `json`, `structured_json`, or `text`. Check that `output.schema` is present and non-empty (has at least one key).

13. **Helper: `validate_preamble()`**: Extract the content after the second `---` delimiter from the skill file. Trim whitespace and fail if the result is empty.

14. **Main flow**: Call each helper in sequence, printing a status line for each check (e.g., `[PASS] YAML frontmatter is parseable`). On success, print a summary line: `Step 1 validation PASSED: skill file saved to ${ARTIFACTS_DIR}/generated-skill.md`.

### External tool dependencies

The script requires these tools, all standard in a Docker-based CI environment:
- `bash` (4.x+)
- `curl` (for HTTP requests)
- `jq` (for JSON parsing)
- `yq` (for YAML parsing -- Mike Farah's `yq` v4+; alternatively fall back to `python3` with PyYAML)
- `uuidgen` (for generating the request UUID; from `uuid-runtime` package, or use a hardcoded UUID)

The script should check for the presence of `jq` and `yq` (or `python3`) at startup and fail early with a clear error message if missing.

### Sample `AgentRequest` payload

```json
{
  "id": "<uuid>",
  "input": "An agent that converts temperatures between Fahrenheit, Celsius, and Kelvin",
  "context": null,
  "caller": "e2e-test"
}
```

### Sample expected `AgentResponse` structure (illustrative, not exact)

```json
{
  "id": "<uuid>",
  "output": {
    "skill_yaml": "---\nname: temperature-converter\nversion: \"1.0\"\ndescription: Converts temperatures...\nmodel:\n  provider: anthropic\n  name: claude-sonnet-4-6\n  temperature: 0.2\ntools:\n  - convert_temperature\nconstraints:\n  max_turns: 5\n  confidence_threshold: 0.9\n  allowed_actions:\n    - read\n    - query\noutput:\n  format: structured_json\n  schema:\n    result: string\n---\nYou are a temperature conversion agent...\n",
    "validation_result": "All checks passed."
  },
  "confidence": 0.95,
  "escalated": false,
  "tool_calls": [...]
}
```

The validator must NOT check for specific values (like `temperature-converter` or `convert_temperature`) since LLM output is non-deterministic. Only structural validity matters.

## Dependencies

- **Blocked by**: "Define the test scenario document" (`tests/e2e/SCENARIO.md`) -- provides the agreed-upon input description and success criteria.
- **Blocking**: "Write the E2E shell script orchestrator" (`scripts/e2e-test.sh`) -- calls this script as step 1 of the pipeline.

## Risks & Edge Cases

1. **`yq` availability**: Not all environments have `yq` installed. The script should attempt `yq` first and fall back to `python3 -c "import yaml; ..."` if `yq` is not available. If neither is available, fail early with an installation hint.

2. **LLM non-determinism**: The skill-writer may produce different field values, different tool names, or different preamble text on each run. The validator must check only structural properties (key presence, type, non-emptiness), never exact values.

3. **`skill_yaml` escaping**: The `skill_yaml` value is a string inside JSON. `jq -r` will unescape it, but the script must handle the case where the string contains embedded newlines, quotes, or special characters. Using `jq -r` to extract and piping directly to a file handles this correctly.

4. **Frontmatter delimiter edge cases**: The skill file format specification (validation rule 9) forbids standalone `---` lines in the preamble. However, the validator should be robust against unexpected `---` lines: use only the first and second `---` occurrences to delimit the frontmatter, treating everything after the second `---` as preamble regardless of content.

5. **Empty or error responses**: The skill-writer may return a non-200 HTTP status (e.g., 500 for internal errors, 422 for max turns exceeded). The script must handle these gracefully, printing the HTTP status and response body before exiting non-zero.

6. **Large response bodies**: The `skill_yaml` string could be large if the LLM generates a verbose preamble. This is not a concern for `jq` or file I/O but the script should not attempt to pass the content as a shell variable for string comparison; use temp files instead.

7. **`output` field structure**: The `AgentResponse.output` is `serde_json::Value`, so its internal structure depends on the agent. If the skill-writer returns `output` as a raw string instead of a JSON object (e.g., if the LLM does not follow the structured output schema), the `jq` extraction `.output.skill_yaml` will return null. The script should detect this and print a helpful error indicating the output structure was unexpected.

8. **Timeout**: LLM inference can take 30-60+ seconds. The `curl` timeout of 120 seconds provides headroom. If the request times out, `curl` exits non-zero and the script should report a timeout rather than a parse error.

## Verification

1. The script file exists at `tests/e2e/validate_step1_skill.sh` and is executable (`chmod +x`).
2. `bash -n tests/e2e/validate_step1_skill.sh` succeeds (syntax check, no parse errors).
3. The script uses `set -euo pipefail` for strict error handling.
4. All helper functions are under 50 lines.
5. The script checks for required external tools (`curl`, `jq`, `yq`/`python3`) at startup.
6. Running the script without a reachable skill-writer service produces a clear connection error (not a cryptic parse failure).
7. The artifacts directory path and file names match what downstream scripts expect: `tests/e2e/artifacts/generated-skill.md` for the skill file and `tests/e2e/artifacts/step1-response.json` for the raw response.
