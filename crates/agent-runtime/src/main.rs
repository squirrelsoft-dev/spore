mod tool_bridge;

use std::path::PathBuf;
use std::sync::Arc;

use rig::client::CompletionClient;
use rig::providers::openai;
use skill_loader::SkillLoader;
use tool_registry::{ToolEntry, ToolRegistry};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Step 1: Create the tool registry
    println!("[1/6] Creating tool registry");
    let registry = Arc::new(ToolRegistry::new());

    // Step 2: Register hardcoded tool entries
    println!("[2/6] Registering tool entries");
    register_default_tools(&registry)?;

    // Step 3: Connect all tool servers
    println!("[3/6] Connecting to tool servers");
    registry.connect_all().await?;

    // Step 4: Load skill manifest
    println!("[4/6] Loading skill manifest");
    let tool_checker = skill_loader::AllToolsExist;
    let loader = SkillLoader::new(
        PathBuf::from("./skills"),
        registry.clone(),
        Box::new(tool_checker),
    );
    let manifest = loader.load("echo").await?;

    // Step 5: Resolve MCP tools for the skill
    println!("[5/6] Resolving MCP tools");
    let tools = tool_bridge::resolve_mcp_tools(&registry, &manifest).await?;

    // Step 6: Build rig-core agent with tools
    println!("[6/6] Building agent");
    let openai_client = openai::Client::new("placeholder-key")?;
    let builder = openai_client.agent("gpt-4o").preamble(&manifest.preamble);
    let _agent = tool_bridge::build_agent_with_tools(builder, tools);

    println!("Agent startup complete");
    Ok(())
}

/// Register the default set of tool entries into the registry.
fn register_default_tools(registry: &ToolRegistry) -> Result<(), Box<dyn std::error::Error>> {
    let entry = ToolEntry {
        name: "echo-tool".to_string(),
        version: "0.1.0".to_string(),
        endpoint: "mcp://localhost:7001".to_string(),
        handle: None,
    };
    registry.register(entry)?;
    Ok(())
}

