# docker-build

An MCP tool server that builds Docker images from a Dockerfile and context directory. It validates all inputs, invokes `docker build`, and returns a JSON result containing the image ID and build log.

## Build

```sh
cargo build -p docker-build
```

## Run

```sh
cargo run -p docker-build
```

The server uses stdio transport: it reads MCP messages from stdin and writes responses to stdout. All logging output is directed to stderr so it does not interfere with the MCP protocol stream.

## Test with MCP Inspector

```sh
npx @modelcontextprotocol/inspector cargo run -p docker-build
```

This launches the MCP Inspector, which connects to the docker-build server and provides an interactive UI for sending requests and viewing responses. Use it to verify that the tool advertises its capabilities and handles calls correctly.

## Test

```sh
cargo test -p docker-build
```

## Parameters

| Name         | Type              | Required | Description                                          |
|--------------|-------------------|----------|------------------------------------------------------|
| `context`    | string            | yes      | Build context path (must be within working directory) |
| `tag`        | string            | yes      | Image tag (e.g. `my-app:latest`)                     |
| `build_args` | map<string,string> | no       | Optional build arguments passed via `--build-arg`    |
| `dockerfile` | string            | no       | Optional path to Dockerfile (must be within working directory) |

## Output

The tool returns a JSON response with the following fields:

| Field       | Type    | Description                                           |
|-------------|---------|-------------------------------------------------------|
| `success`   | boolean | Whether the Docker build completed successfully       |
| `image_id`  | string  | Built image ID (may be empty if ID cannot be parsed)  |
| `tag`       | string  | The tag applied to the built image                    |
| `build_log` | string  | Combined stdout and stderr output from Docker         |

### Success example

```json
{
  "success": true,
  "image_id": "sha256:abc123def456",
  "tag": "my-app:latest",
  "build_log": "Step 1/3 : FROM alpine\n..."
}
```

### Failure example

```json
{
  "success": false,
  "build_log": "Invalid context path: path traversal not allowed"
}
```

## Security Considerations

- **Path validation** -- Context and Dockerfile paths are canonicalized and verified to stay within the current working directory. Path traversal (`..`) is rejected.
- **Tag validation** -- Tags are restricted to alphanumeric characters and `._:/-`. Shell metacharacters are rejected.
- **Build-arg sanitization** -- Both keys and values of build arguments are checked for shell metacharacters (`;&|$` and others). Invalid arguments are rejected before invoking Docker.
- **No shell execution** -- The tool invokes `docker` directly via `std::process::Command` without spawning a shell, preventing shell injection attacks.

## Notes

- **Docker-in-Docker** -- When running inside a container, the host Docker socket must be mounted (e.g. `-v /var/run/docker.sock:/var/run/docker.sock`) for builds to work.
- **Image ID parsing** -- The `image_id` field may be empty if the build output does not contain a recognizable image ID. This can happen with certain BuildKit output configurations.
