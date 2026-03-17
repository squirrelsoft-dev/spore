# echo-tool

A reference MCP tool server that returns input messages unchanged. This crate serves as the template for all future tool implementations in the project.

## Build

```sh
cargo build -p echo-tool
```

## Run

```sh
cargo run -p echo-tool
```

The server uses stdio transport: it reads MCP messages from stdin and writes responses to stdout. All logging output is directed to stderr so it does not interfere with the MCP protocol stream.

## Test with MCP Inspector

```sh
npx @modelcontextprotocol/inspector cargo run -p echo-tool
```

This launches the MCP Inspector, which connects to the echo-tool server and provides an interactive UI for sending requests and viewing responses. Use it to verify that the tool advertises its capabilities and handles calls correctly.

## Creating a New Tool

1. Create a new directory under `tools/` (e.g. `tools/my-tool/`).

2. Copy `tools/echo-tool/Cargo.toml` into your new directory and update the package name:
   ```toml
   [package]
   name = "my-tool"
   ```

3. Define a tool struct and annotate its impl block with `#[tool_router]`. Each tool method gets a `#[tool(description = "...")]` attribute:
   ```rust
   #[tool_router]
   impl MyTool {
       #[tool(description = "Describe what this tool does")]
       fn my_method(&self, Parameters(req): Parameters<MyRequest>) -> String {
           // implementation
       }
   }
   ```

4. Implement `ServerHandler` for your struct with `get_info()` returning capabilities with tools enabled:
   ```rust
   #[tool_handler]
   impl ServerHandler for MyTool {
       fn get_info(&self) -> ServerInfo {
           ServerInfo::new(ServerCapabilities::builder().enable_tools().build())
       }
   }
   ```

5. Write a `main()` function that initializes logging to stderr, creates the tool, and serves over stdio:
   ```rust
   #[tokio::main(flavor = "current_thread")]
   async fn main() -> Result<(), Box<dyn std::error::Error>> {
       tracing_subscriber::fmt()
           .with_env_filter(EnvFilter::from_default_env().add_directive(tracing::Level::DEBUG.into()))
           .with_writer(std::io::stderr)
           .with_ansi(false)
           .init();

       let service = MyTool::new()
           .serve(rmcp::transport::stdio())
           .await?;

       service.waiting().await?;
       Ok(())
   }
   ```

6. Add the new crate path to the workspace `members` list in the root `Cargo.toml`:
   ```toml
   [workspace]
   members = [
       # ...existing members...
       "tools/my-tool",
   ]
   ```

7. Verify everything works:
   ```sh
   cargo build -p my-tool
   cargo test -p my-tool
   cargo clippy -p my-tool
   ```

## Tool Registry

Registration of tools with the tool-registry is pending. See issue #8 for details.
