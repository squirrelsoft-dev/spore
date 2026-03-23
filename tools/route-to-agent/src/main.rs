mod route_to_agent;
use route_to_agent::RouteToAgentTool;

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    mcp_tool_harness::serve_stdio_tool(RouteToAgentTool::new(), "route-to-agent").await
}
