use agent_sdk::SkillManifest;
use rig::agent::{Agent, AgentBuilder, NoToolConfig, PromptHook};
use rig::completion::CompletionModel;
use rig::tool::rmcp::McpTool;
use rig::tool::ToolDyn;
use tool_registry::{RegistryError, ToolEntry, ToolRegistry};

/// Discovers MCP tools from connected handles and wraps them as `McpTool`.
///
/// Resolves tool entries for the given manifest, then filters out any entries
/// whose `action_type` is not in the provided `allowed_actions` list. If
/// `allowed_actions` is empty, no filtering is applied. Entries with no
/// `action_type` are always included. Finally, queries each connected MCP
/// server to list its available tools and returns a flat list of `McpTool`
/// instances ready for attachment to a rig-core agent.
pub async fn resolve_mcp_tools(
    registry: &ToolRegistry,
    manifest: &SkillManifest,
    allowed_actions: &[String],
) -> Result<Vec<McpTool>, RegistryError> {
    let entries = registry.resolve_for_skill(manifest)?;

    let entries: Vec<ToolEntry> = if allowed_actions.is_empty() {
        entries
    } else {
        entries
            .into_iter()
            .filter(|entry| match &entry.action_type {
                None => true,
                Some(t) => allowed_actions.iter().any(|a| a == t),
            })
            .collect()
    };

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
