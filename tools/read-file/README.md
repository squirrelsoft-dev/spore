# read-file

An MCP tool server that reads the contents of a file from disk and returns them as a string.

## Build

```sh
cargo build -p read-file
```

## Run

```sh
cargo run -p read-file
```

The server uses stdio transport: it reads MCP messages from stdin and writes responses to stdout. All logging output is directed to stderr so it does not interfere with the MCP protocol stream.

## Test with MCP Inspector

```sh
npx @modelcontextprotocol/inspector cargo run -p read-file
```

This launches the MCP Inspector, which connects to the read-file server and provides an interactive UI for sending requests and viewing responses. Use it to verify that the tool advertises its capabilities and handles calls correctly.

## Tool

### `read_file`

Reads the contents of a file from disk and returns them as a string.

| Input | Type   | Description                              |
|-------|--------|------------------------------------------|
| path  | string | Path to the file (absolute or relative)  |

On success, returns the full text content of the file. On failure, returns an error string starting with `"Error"` that includes the path and a description of the problem.
