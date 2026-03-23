# register-agent

An MCP tool server that registers an agent with the orchestrator by POSTing its name, URL, and description, returning structured JSON with registration status.

## Build

```sh
cargo build -p register-agent
```

## Run

```sh
cargo run -p register-agent
```

The server uses stdio transport: it reads MCP messages from stdin and writes responses to stdout. All logging output is directed to stderr so it does not interfere with the MCP protocol stream.

## Test with MCP Inspector

```sh
npx @modelcontextprotocol/inspector cargo run -p register-agent
```

This launches the MCP Inspector, which connects to the register-agent server and provides an interactive UI for sending requests and viewing responses. Use it to verify that the tool advertises its capabilities and handles calls correctly.

## Test

```sh
cargo test -p register-agent
```

## Parameters

| Name          | Type   | Required | Description                              |
|---------------|--------|----------|------------------------------------------|
| `name`        | string | yes      | Agent name (alphanumeric, hyphens, underscores, dots) |
| `url`         | string | yes      | Agent endpoint URL (must be valid URL format)    |
| `description` | string | yes      | Human-readable description of the agent          |

## Output

The tool returns a JSON response with the following fields:

| Field            | Type    | Description                                          |
|------------------|---------|------------------------------------------------------|
| `success`        | boolean | Whether the registration completed successfully          |
| `agent_name`     | string  | The name of the registered agent                         |
| `registered_url` | string  | The URL at which the agent was registered (empty on failure) |
| `error`          | string  | Error message (empty on success, present on failure)     |

### Success example

```json
{
  "success": true,
  "agent_name": "my-agent",
  "registered_url": "http://my-agent:8080",
  "error": ""
}
```

### Failure example

```json
{
  "success": false,
  "agent_name": "my-agent",
  "registered_url": "",
  "error": "Failed to register agent: orchestrator returned 503"
}
```

## Environment Variables

| Variable           | Description                                                              |
|-------------------|--------------------------------------------------------------------------|
| `ORCHESTRATOR_URL` | Base URL of the orchestrator service (default: `http://orchestrator:8080`) |

The `ORCHESTRATOR_URL` environment variable sets the base URL for the orchestrator. If not set, it defaults to `http://orchestrator:8080`. The registration payload is POSTed to `{ORCHESTRATOR_URL}/register`.

## Security Considerations

- **Name validation** -- Agent names are restricted to ASCII alphanumeric characters, dots, underscores, and hyphens (`._-`). Shell metacharacters and whitespace are rejected.
- **URL validation** -- URLs must begin with `http://` or `https://`. Other schemes and empty values are rejected.
- **Description validation** -- Descriptions must be non-empty to prevent registering agents without meaningful metadata.
- **No shell execution** -- The tool sends HTTP requests directly via `reqwest` without spawning a shell, preventing shell injection attacks.
