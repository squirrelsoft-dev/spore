mod read_file;
use read_file::ReadFileTool;

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    mcp_tool_harness::serve_stdio_tool(ReadFileTool::new(), "read-file").await
}
