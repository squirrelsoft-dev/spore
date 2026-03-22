mod validate_skill;
use validate_skill::ValidateSkillTool;

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    mcp_tool_harness::serve_stdio_tool(ValidateSkillTool::new(), "validate-skill").await
}
