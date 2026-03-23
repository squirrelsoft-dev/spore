# Spec: Write README.md

> From: .claude/tasks/issue-50.md

## Objective
Create `tools/register-agent/README.md` that documents the register-agent MCP tool server, following the established documentation pattern used by docker-build and docker-push READMEs. This gives users and contributors a consistent reference for building, running, testing, and understanding the tool.

## Current State
- `tools/docker-build/README.md` and `tools/docker-push/README.md` establish the documentation pattern: purpose line, Build/Run/Test sections with cargo commands, MCP Inspector section, Parameters table, Output table with examples, Environment Variables, and Security Considerations.
- The register-agent tool does not yet exist (blocked by core implementation tasks), but the task breakdown in `issue-50.md` fully specifies the interface: parameters (`name`, `url`, `description`), output fields (`success`, `agent_name`, `registered_url`, `error`), and environment variable (`ORCHESTRATOR_URL` with default `http://orchestrator:8080`).

## Requirements
- Follow the same section ordering and formatting as `tools/docker-build/README.md`
- Include all sections specified in the task: purpose, build/run/test commands, MCP Inspector command, parameters table, output JSON table, success/failure examples, environment variables, security considerations
- Package name in all cargo commands must be `register-agent`
- Parameters table must document three required parameters: `name` (string), `url` (string), `description` (string)
- Output table must document four fields: `success` (boolean), `agent_name` (string), `registered_url` (string), `error` (string)
- Environment variables section must document `ORCHESTRATOR_URL` with default value `http://orchestrator:8080` and explain resolution order
- Success example must show a JSON object with `success: true`, `agent_name`, and `registered_url` populated
- Failure example must show a JSON object with `success: false`, an `error` message, and empty `registered_url`
- Security considerations must cover: input validation (name/url/description), no shell execution, and URL format validation

## Implementation Details

### Sections to include (in order)

1. **Title and purpose line**: `# register-agent` followed by a one-sentence description: "An MCP tool server that registers an agent with the orchestrator by POSTing its name, URL, and description, returning structured JSON with registration status."

2. **Build section**: `cargo build -p register-agent`

3. **Run section**: `cargo run -p register-agent` with the standard stdio transport explanation paragraph (copied from docker-build/docker-push pattern).

4. **Test with MCP Inspector section**: `npx @modelcontextprotocol/inspector cargo run -p register-agent` with the standard explanatory paragraph.

5. **Test section**: `cargo test -p register-agent`

6. **Parameters table** (Input Parameters):

   | Name          | Type   | Required | Description                                      |
   |---------------|--------|----------|--------------------------------------------------|
   | `name`        | string | yes      | Agent name (alphanumeric, hyphens, underscores)  |
   | `url`         | string | yes      | Agent endpoint URL (must be valid URL format)    |
   | `description` | string | yes      | Human-readable description of the agent          |

7. **Output section** with table:

   | Field            | Type    | Description                                              |
   |------------------|---------|----------------------------------------------------------|
   | `success`        | boolean | Whether the registration completed successfully          |
   | `agent_name`     | string  | The name of the registered agent                         |
   | `registered_url` | string  | The URL at which the agent was registered (empty on failure) |
   | `error`          | string  | Error message (empty on success, present on failure)     |

8. **Success example**:
   ```json
   {
     "success": true,
     "agent_name": "my-agent",
     "registered_url": "http://my-agent:8080",
     "error": ""
   }
   ```

9. **Failure example**:
   ```json
   {
     "success": false,
     "agent_name": "my-agent",
     "registered_url": "",
     "error": "Failed to register agent: orchestrator returned 503"
   }
   ```

10. **Environment Variables table**:

    | Variable          | Description                                                              |
    |-------------------|--------------------------------------------------------------------------|
    | `ORCHESTRATOR_URL` | Base URL of the orchestrator service (default: `http://orchestrator:8080`) |

    Include a paragraph explaining: the `ORCHESTRATOR_URL` environment variable sets the base URL for the orchestrator. If not set, it defaults to `http://orchestrator:8080`. The registration payload is POSTed to `{ORCHESTRATOR_URL}/register`.

11. **Security Considerations** (bulleted with bold labels, matching docker-build style):
    - **Name validation** -- Agent names are restricted to alphanumeric characters, hyphens, and underscores. Shell metacharacters are rejected.
    - **URL validation** -- The `url` parameter is validated for proper URL format. Malformed URLs are rejected before any HTTP request is made.
    - **Description validation** -- Empty descriptions are rejected.
    - **No shell execution** -- The tool uses `reqwest` to make HTTP requests directly, without spawning a shell process.

## Dependencies
- Blocked by: "Implement register_agent tool logic" (the README documents the implemented interface; it should be written after or alongside implementation to ensure accuracy)
- Blocking: None

## Risks & Edge Cases
- The actual implementation may diverge slightly from the task breakdown spec (e.g., field names, error message format). The README should be reviewed against the final implementation and updated if needed.
- The `error` field behavior (empty string vs. absent) should match whatever serde serialization the implementation uses. The spec assumes it is always present as a string.

## Verification
- The README file exists at `tools/register-agent/README.md`
- All sections listed above are present and in the specified order
- Cargo commands use `-p register-agent` consistently
- The parameters table lists exactly three required parameters: `name`, `url`, `description`
- The output table lists exactly four fields: `success`, `agent_name`, `registered_url`, `error`
- Success and failure JSON examples are valid JSON and consistent with the output table
- The `ORCHESTRATOR_URL` environment variable is documented with its default value
- Security considerations cover input validation for all three parameters, URL format validation, and no-shell-execution
- Formatting is consistent with `tools/docker-build/README.md` (heading levels, table alignment, code fence language tags)
