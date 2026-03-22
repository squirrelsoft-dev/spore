# write-file

An MCP tool server that writes content to files, creating parent directories as needed.

## Build

```sh
cargo build -p write-file
```

## Run

```sh
cargo run -p write-file
```

The server uses stdio transport: it reads MCP messages from stdin and writes responses to stdout. All logging output is directed to stderr so it does not interfere with the MCP protocol stream.

## Test with MCP Inspector

```sh
npx @modelcontextprotocol/inspector cargo run -p write-file
```

This launches the MCP Inspector, which connects to the write-file server and provides an interactive UI for sending requests and viewing responses. Use it to verify that the tool advertises its capabilities and handles calls correctly.

## Test

```sh
cargo test -p write-file
```

## Parameters

| Parameter | Type   | Description                                                                 |
|-----------|--------|-----------------------------------------------------------------------------|
| `path`    | string | Absolute or relative path of the file to write. Parent directories are created if they do not exist. |
| `content` | string | Content to write to the file.                                               |
