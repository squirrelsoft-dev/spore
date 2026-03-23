# list-agents

An MCP tool server that reads agent registrations from environment variables and returns a filtered JSON array of registered agents.

## Build

```sh
cargo build -p list-agents
```

## Run

```sh
cargo run -p list-agents
```

The server uses stdio transport: it reads MCP messages from stdin and writes responses to stdout. All logging output is directed to stderr so it does not interfere with the MCP protocol stream.

## Test with MCP Inspector

```sh
npx @modelcontextprotocol/inspector cargo run -p list-agents
```

This launches the MCP Inspector, which connects to the list-agents server and provides an interactive UI for sending requests and viewing responses. Use it to verify that the tool advertises its capabilities and handles calls correctly.

## Test

```sh
cargo test -p list-agents
```

## Parameters

| Name     | Type   | Required | Description                                                        |
|----------|--------|----------|--------------------------------------------------------------------|
| `filter` | string | no       | Case-insensitive substring match against agent name and description |

## Output

The tool returns a JSON response with the following fields:

| Field    | Type   | Description                                              |
|----------|--------|----------------------------------------------------------|
| `agents` | array  | Array of agent objects with `name`, `url`, `description` |
| `error`  | string | Error message (empty on success, present on failure)     |

### Success example

```json
{
  "agents": [
    {
      "name": "skill-writer",
      "url": "http://skill-writer:8080",
      "description": "Writes skill files from templates"
    },
    {
      "name": "tool-coder",
      "url": "http://tool-coder:9090",
      "description": "Generates MCP tool implementations"
    }
  ]
}
```

### Filtered example

When `filter` is set to `"skill"`, only agents whose name or description contains "skill" (case-insensitive) are returned:

```json
{
  "agents": [
    {
      "name": "skill-writer",
      "url": "http://skill-writer:8080",
      "description": "Writes skill files from templates"
    }
  ]
}
```

### Empty result example

```json
{
  "agents": []
}
```

### Error example

```json
{
  "agents": [],
  "error": "invalid pair 'bad-entry', expected 'key=value'"
}
```

## Environment Variables

| Variable              | Required | Description                                                    |
|-----------------------|----------|----------------------------------------------------------------|
| `AGENT_ENDPOINTS`     | no       | Comma-separated list of `name=url` pairs (e.g. `a=http://a:8080,b=http://b:9090`). Returns empty array if unset. |
| `AGENT_DESCRIPTIONS`  | no       | Comma-separated list of `name=description` pairs (e.g. `a=My agent,b=Other agent`) |

`AGENT_ENDPOINTS` defines which agents are available. Each entry is a `name=url` pair separated by commas. Whitespace around names and URLs is trimmed.

`AGENT_DESCRIPTIONS` provides optional human-readable descriptions for agents. Entries that cannot be parsed are silently skipped. Agents without a matching description entry receive an empty description string.

## Usage

Set environment variables and run the server:

```sh
AGENT_ENDPOINTS="skill-writer=http://skill-writer:8080,tool-coder=http://tool-coder:9090" \
AGENT_DESCRIPTIONS="skill-writer=Writes skill files from templates,tool-coder=Generates MCP tool implementations" \
cargo run -p list-agents
```

To test with a filter using MCP Inspector:

```sh
AGENT_ENDPOINTS="skill-writer=http://skill-writer:8080,tool-coder=http://tool-coder:9090" \
AGENT_DESCRIPTIONS="skill-writer=Writes skill files from templates,tool-coder=Generates MCP tool implementations" \
npx @modelcontextprotocol/inspector cargo run -p list-agents
```

Then call the `list_agents` tool with `{"filter": "skill"}` to see only matching agents.
