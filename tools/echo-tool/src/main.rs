mod echo;
use echo::EchoTool;

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    mcp_tool_harness::serve_stdio_tool(EchoTool::new(), "echo-tool").await
}
