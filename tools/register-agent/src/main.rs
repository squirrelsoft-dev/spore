mod register_agent;
use register_agent::RegisterAgentTool;

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    mcp_tool_harness::serve_stdio_tool(RegisterAgentTool::new(), "register-agent").await
}
