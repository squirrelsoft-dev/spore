# route-to-agent

An MCP tool server that routes a request to a named agent by looking up its endpoint in environment variables, forwarding the payload, and returning the structured JSON response.

## Build

```sh
cargo build -p route-to-agent
```

## Run

```sh
cargo run -p route-to-agent
```

The server uses stdio transport: it reads MCP messages from stdin and writes responses to stdout. All logging output is directed to stderr so it does not interfere with the MCP protocol stream.

## Test with MCP Inspector

```sh
npx @modelcontextprotocol/inspector cargo run -p route-to-agent
```

This launches the MCP Inspector, which connects to the route-to-agent server and provides an interactive UI for sending requests and viewing responses. Use it to verify that the tool advertises its capabilities and handles calls correctly.

## Test

```sh
cargo test -p route-to-agent
```

## Parameters

| Name         | Type   | Required | Description                                     |
|--------------|--------|----------|-------------------------------------------------|
| `agent_name` | string | yes      | Name of the target agent to route the request to |
| `input`      | string | yes      | The request payload to forward to the agent      |

## Output

The tool returns a JSON response with the following fields:

| Field        | Type    | Description                                                    |
|--------------|---------|----------------------------------------------------------------|
| `success`    | boolean | Whether the request completed successfully                     |
| `agent_name` | string  | The name of the target agent                                   |
| `response`   | object  | The agent's response object (null on failure)                  |
| `error`      | string  | Error message (empty on success, present on failure)           |

### Success example

```json
{
  "success": true,
  "agent_name": "skill-writer",
  "response": {
    "id": "a1b2c3d4-e5f6-7890-abcd-ef1234567890",
    "output": "Generated skill file successfully",
    "confidence": 0.95,
    "escalated": false,
    "tool_calls": []
  },
  "error": ""
}
```

### Error examples

Agent not found in configured endpoints:

```json
{
  "success": false,
  "agent_name": "unknown-agent",
  "response": null,
  "error": "Agent 'unknown-agent' not found in AGENT_ENDPOINTS"
}
```

Agent is unreachable:

```json
{
  "success": false,
  "agent_name": "skill-writer",
  "response": null,
  "error": "Request failed: error sending request for url (http://skill-writer:8080/invoke)"
}
```

Agent returns an error HTTP status:

```json
{
  "success": false,
  "agent_name": "skill-writer",
  "response": null,
  "error": "HTTP 503 Service Unavailable: service overloaded"
}
```

## Environment Variables

| Variable          | Required | Description                                                                                      |
|-------------------|----------|--------------------------------------------------------------------------------------------------|
| `AGENT_ENDPOINTS` | yes      | Comma-separated list of `name=url` pairs (e.g. `skill-writer=http://skill-writer:8080,tool-coder=http://tool-coder:9090`) |

`AGENT_ENDPOINTS` defines the available agents and their URLs. Each entry is a `name=url` pair separated by commas. Whitespace around names and URLs is trimmed. The tool appends `/invoke` to the resolved URL when forwarding requests.

## Usage

Set environment variables and run the server:

```sh
AGENT_ENDPOINTS="skill-writer=http://skill-writer:8080,tool-coder=http://tool-coder:9090" \
cargo run -p route-to-agent
```

To test with MCP Inspector:

```sh
AGENT_ENDPOINTS="skill-writer=http://skill-writer:8080,tool-coder=http://tool-coder:9090" \
npx @modelcontextprotocol/inspector cargo run -p route-to-agent
```

Then call the `route_to_agent` tool with `{"agent_name": "skill-writer", "input": "write a greeting skill"}` to route a request to the specified agent.
