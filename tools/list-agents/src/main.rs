mod list_agents;
use list_agents::ListAgentsTool;

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    mcp_tool_harness::serve_stdio_tool(ListAgentsTool::new(), "list-agents").await
}
