use std::env;
use std::fmt;

use agent_sdk::SkillManifest;
use rig::agent::Agent;
use rig::client::CompletionClient;
use rig::completion::Prompt;
use rig::providers::{anthropic, openai};
use tool_registry::ToolRegistry;

use crate::tool_bridge;

// ================================================================
// Error type
// ================================================================

/// Errors that can occur when building or prompting a provider-backed agent.
#[derive(Debug)]
pub enum ProviderError {
    /// The requested provider string is not recognized.
    UnsupportedProvider { provider: String },
    /// The required API key environment variable is missing.
    MissingApiKey { provider: String, env_var: String },
    /// The underlying client or agent failed to build.
    ClientBuild(String),
    /// An error occurred while prompting the agent.
    Prompt(String),
}

impl fmt::Display for ProviderError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnsupportedProvider { provider } => {
                write!(f, "unsupported provider: {provider}")
            }
            Self::MissingApiKey { provider, env_var } => {
                write!(
                    f,
                    "missing API key for provider {provider}: \
                     set environment variable {env_var}"
                )
            }
            Self::ClientBuild(msg) => {
                write!(f, "client build error: {msg}")
            }
            Self::Prompt(msg) => {
                write!(f, "prompt error: {msg}")
            }
        }
    }
}

impl std::error::Error for ProviderError {}

// ================================================================
// BuiltAgent enum
// ================================================================

type OpenAiModel = openai::responses_api::ResponsesCompletionModel;
type AnthropicModel = anthropic::completion::CompletionModel;

/// A provider-dispatched agent wrapper that hides the concrete model type.
///
/// Each variant holds a fully-configured rig-core `Agent` for a specific
/// provider. Call [`BuiltAgent::prompt`] to send a message regardless of
/// which provider backs the agent.
pub enum BuiltAgent {
    OpenAi(Agent<OpenAiModel>),
    Anthropic(Agent<AnthropicModel>),
}

impl BuiltAgent {
    /// Send a prompt to the underlying agent and return the response text.
    pub async fn prompt(&self, input: &str) -> Result<String, ProviderError> {
        match self {
            Self::OpenAi(agent) => agent
                .prompt(input)
                .await
                .map_err(|e| ProviderError::Prompt(e.to_string())),
            Self::Anthropic(agent) => agent
                .prompt(input)
                .await
                .map_err(|e| ProviderError::Prompt(e.to_string())),
        }
    }
}

// ================================================================
// build_agent
// ================================================================

/// Build a fully-configured agent from a skill manifest.
///
/// Reads `manifest.model.provider` to select the rig-core provider, fetches
/// the API key from the environment, resolves MCP tools via the registry,
/// and returns a [`BuiltAgent`] ready for prompting.
pub async fn build_agent(
    manifest: &SkillManifest,
    registry: &ToolRegistry,
) -> Result<BuiltAgent, ProviderError> {
    let provider = manifest.model.provider.as_str();
    let model_name = &manifest.model.name;
    let preamble = &manifest.preamble;
    let temperature = manifest.model.temperature;

    tracing::info!(provider, model = %model_name, "building agent");

    let tools = resolve_tools(registry, manifest).await?;
    let max_turns = manifest.constraints.max_turns;

    match provider {
        "openai" => build_openai_agent(model_name, preamble, temperature, tools, max_turns),
        "anthropic" => build_anthropic_agent(model_name, preamble, temperature, tools, max_turns),
        other => Err(ProviderError::UnsupportedProvider {
            provider: other.to_string(),
        }),
    }
}

// ================================================================
// Internal helpers
// ================================================================

/// Read an environment variable, returning a [`ProviderError::MissingApiKey`] on failure.
fn read_api_key(provider: &str, env_var: &str) -> Result<String, ProviderError> {
    env::var(env_var).map_err(|_| ProviderError::MissingApiKey {
        provider: provider.to_string(),
        env_var: env_var.to_string(),
    })
}

/// Resolve MCP tools for the manifest via the tool registry.
async fn resolve_tools(
    registry: &ToolRegistry,
    manifest: &SkillManifest,
) -> Result<Vec<rig::tool::rmcp::McpTool>, ProviderError> {
    tool_bridge::resolve_mcp_tools(registry, manifest)
        .await
        .map_err(|e| ProviderError::ClientBuild(e.to_string()))
}

/// Construct an OpenAI-backed agent.
fn build_openai_agent(
    model_name: &str,
    preamble: &str,
    temperature: f64,
    tools: Vec<rig::tool::rmcp::McpTool>,
    max_turns: u32,
) -> Result<BuiltAgent, ProviderError> {
    let api_key = read_api_key("openai", "OPENAI_API_KEY")?;
    let client = openai::Client::new(api_key)
        .map_err(|e| ProviderError::ClientBuild(e.to_string()))?;

    let builder = client
        .agent(model_name)
        .preamble(preamble)
        .temperature(temperature);
    let agent = tool_bridge::build_agent_with_tools(builder, tools, max_turns);

    tracing::info!("openai agent built successfully");
    Ok(BuiltAgent::OpenAi(agent))
}

/// Construct an Anthropic-backed agent.
fn build_anthropic_agent(
    model_name: &str,
    preamble: &str,
    temperature: f64,
    tools: Vec<rig::tool::rmcp::McpTool>,
    max_turns: u32,
) -> Result<BuiltAgent, ProviderError> {
    let api_key = read_api_key("anthropic", "ANTHROPIC_API_KEY")?;
    let client = anthropic::Client::builder()
        .api_key(api_key)
        .build()
        .map_err(|e| ProviderError::ClientBuild(e.to_string()))?;

    let builder = client
        .agent(model_name)
        .preamble(preamble)
        .temperature(temperature);
    let agent = tool_bridge::build_agent_with_tools(builder, tools, max_turns);

    tracing::info!("anthropic agent built successfully");
    Ok(BuiltAgent::Anthropic(agent))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unsupported_provider_displays_name() {
        let err = ProviderError::UnsupportedProvider {
            provider: "cohere".to_string(),
        };
        assert_eq!(err.to_string(), "unsupported provider: cohere");
    }

    #[test]
    fn missing_api_key_displays_details() {
        let err = ProviderError::MissingApiKey {
            provider: "openai".to_string(),
            env_var: "OPENAI_API_KEY".to_string(),
        };
        let msg = err.to_string();
        assert!(msg.contains("openai"));
        assert!(msg.contains("OPENAI_API_KEY"));
    }

    #[test]
    fn client_build_error_displays_message() {
        let err = ProviderError::ClientBuild("connection refused".to_string());
        assert_eq!(err.to_string(), "client build error: connection refused");
    }

    #[test]
    fn prompt_error_displays_message() {
        let err = ProviderError::Prompt("timeout".to_string());
        assert_eq!(err.to_string(), "prompt error: timeout");
    }

    #[test]
    fn read_api_key_returns_error_when_missing() {
        // Use a key name that definitely does not exist
        let result = read_api_key("test_provider", "SPORE_TEST_NONEXISTENT_KEY_12345");
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(matches!(err, ProviderError::MissingApiKey { .. }));
    }
}
