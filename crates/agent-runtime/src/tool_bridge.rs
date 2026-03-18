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

    let futures = entries.iter().filter_map(|entry| {
        entry.handle.as_ref().map(|handle| async move {
            let tools_result = handle.peer().list_tools(None).await.map_err(|e| {
                RegistryError::ConnectionFailed {
                    endpoint: entry.endpoint.clone(),
                    reason: e.to_string(),
                }
            })?;

            let server_sink = handle.peer().clone();
            Ok(tools_result
                .tools
                .into_iter()
                .map(|tool_def| McpTool::from_mcp_server(tool_def, server_sink.clone()))
                .collect::<Vec<_>>())
        })
    });

    let results: Vec<Result<Vec<McpTool>, RegistryError>> =
        futures::future::join_all(futures).await;

    results
        .into_iter()
        .collect::<Result<Vec<_>, _>>()
        .map(|v| v.into_iter().flatten().collect())
}

/// Attaches resolved MCP tools to a rig-core `AgentBuilder` and produces an `Agent`.
///
/// Each `McpTool` is boxed as a `dyn ToolDyn` and added to the builder before
/// calling `.build()` to finalize the agent. The `max_turns` parameter sets the
/// agent's default turn limit via `AgentBuilder::default_max_turns()`.
pub fn build_agent_with_tools<M, P>(
    builder: AgentBuilder<M, P, NoToolConfig>,
    tools: Vec<McpTool>,
    max_turns: u32,
) -> Agent<M, P>
where
    M: CompletionModel,
    P: PromptHook<M>,
{
    let boxed: Vec<Box<dyn ToolDyn>> = tools
        .into_iter()
        .map(|t| Box::new(t) as Box<dyn ToolDyn>)
        .collect();
    builder
        .default_max_turns(max_turns as usize)
        .tools(boxed)
        .build()
}
