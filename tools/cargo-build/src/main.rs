mod cargo_build;
use cargo_build::CargoBuildTool;

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    mcp_tool_harness::serve_stdio_tool(CargoBuildTool::new(), "cargo-build").await
}
