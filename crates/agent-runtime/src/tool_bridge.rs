use agent_sdk::SkillManifest;
use rig::agent::{Agent, AgentBuilder, NoToolConfig, PromptHook};
use rig::completion::CompletionModel;
use rig::tool::rmcp::McpTool;
use rig::tool::ToolDyn;
use tool_registry::{RegistryError, ToolRegistry};

/// Discovers MCP tools from connected handles and wraps them as `McpTool`.
///
/// Resolves tool entries for the given manifest, then queries each connected
/// MCP server to list its available tools. Returns a flat list of `McpTool`
/// instances ready for attachment to a rig-core agent.
pub async fn resolve_mcp_tools(
    registry: &ToolRegistry,
    manifest: &SkillManifest,
) -> Result<Vec<McpTool>, RegistryError> {
    let entries = registry.resolve_for_skill(manifest)?;
    let mut mcp_tools = Vec::new();

    for entry in &entries {
        let handle = match &entry.handle {
            Some(h) => h,
            None => continue,
        };

        let tools_result = handle.peer().list_tools(None).await.map_err(|e| {
            RegistryError::ConnectionFailed {
                endpoint: entry.endpoint.clone(),
                reason: e.to_string(),
            }
        })?;

        let server_sink = handle.peer().clone();
        for tool_def in tools_result.tools {
            mcp_tools.push(McpTool::from_mcp_server(tool_def, server_sink.clone()));
        }
    }

    Ok(mcp_tools)
}

/// Attaches resolved MCP tools to a rig-core `AgentBuilder` and produces an `Agent`.
///
/// Each `McpTool` is boxed as a `dyn ToolDyn` and added to the builder before
/// calling `.build()` to finalize the agent.
pub fn build_agent_with_tools<M, P>(
    builder: AgentBuilder<M, P, NoToolConfig>,
    tools: Vec<McpTool>,
) -> Agent<M, P>
where
    M: CompletionModel,
    P: PromptHook<M>,
{
    let boxed: Vec<Box<dyn ToolDyn>> = tools
        .into_iter()
        .map(|t| Box::new(t) as Box<dyn ToolDyn>)
        .collect();
    builder.tools(boxed).build()
}
