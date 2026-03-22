mod docker_push;
use docker_push::DockerPushTool;

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    mcp_tool_harness::serve_stdio_tool(DockerPushTool::new(), "docker-push").await
}
