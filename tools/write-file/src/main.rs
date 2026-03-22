mod write_file;
use write_file::WriteFileTool;

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    mcp_tool_harness::serve_stdio_tool(WriteFileTool::new(), "write-file").await
}
