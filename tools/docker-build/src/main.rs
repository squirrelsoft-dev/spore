mod docker_build;
use docker_build::DockerBuildTool;

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    mcp_tool_harness::serve_stdio_tool(DockerBuildTool::new(), "docker-build").await
}
