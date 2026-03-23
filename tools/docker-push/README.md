# docker-push

An MCP tool server that pushes a tagged Docker image to a container registry, returning structured JSON with push status, digest, and logs.

## Build

```sh
cargo build -p docker-push
```

## Run

```sh
cargo run -p docker-push
```

The server uses stdio transport: it reads MCP messages from stdin and writes responses to stdout. All logging output is directed to stderr so it does not interfere with the MCP protocol stream.

## Test

```sh
cargo test -p docker-push
```

## Input Parameters

| Parameter      | Type   | Required | Description                                                         |
|----------------|--------|----------|---------------------------------------------------------------------|
| `image`        | string | yes      | Full image reference (e.g., `ghcr.io/spore/spore-agent:0.1`)       |
| `registry_url` | string | no       | Override registry URL; falls back to the `REGISTRY_URL` env var     |

## Output Format

JSON object with the following fields:

| Field      | Type   | Description                                      |
|------------|--------|--------------------------------------------------|
| `success`  | bool   | Whether the push succeeded                       |
| `image`    | string | The resolved image reference that was pushed      |
| `digest`   | string | sha256 digest extracted from docker push output, or empty string |
| `push_log` | string | Combined stdout/stderr from `docker push`        |

Example response:

```json
{
  "success": true,
  "image": "ghcr.io/spore/spore-agent:0.1",
  "digest": "sha256:abc123def456...",
  "push_log": "The push refers to repository..."
}
```

## Environment Variables

| Variable       | Description                                                                                           |
|----------------|-------------------------------------------------------------------------------------------------------|
| `REGISTRY_URL` | Fallback registry URL used when `registry_url` is not provided as a parameter                         |

Resolution order: the `registry_url` parameter takes precedence over the `REGISTRY_URL` environment variable. If neither is provided and the image has no registry prefix, Docker's default registry is used.

## Test with MCP Inspector

```sh
npx @modelcontextprotocol/inspector cargo run -p docker-push
```

This launches the MCP Inspector, which connects to the docker-push server and provides an interactive UI for sending requests and viewing responses. Use it to verify that the tool advertises its capabilities and handles calls correctly.

## Prerequisites

- Docker must be installed and the `docker` command must be available on PATH.
- Authentication to the target registry must be pre-configured via `docker login` or credential helpers before invoking the tool. The tool does not handle login.
- Input validation rejects shell metacharacters; only alphanumeric characters, `.`, `-`, `_`, `/`, and `:` are allowed in image references and registry URLs.
