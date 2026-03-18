use std::sync::Arc;

use agent_runtime::config::RuntimeConfig;
use agent_runtime::http;
use agent_runtime::provider;
use agent_runtime::runtime_agent::RuntimeAgent;
use agent_sdk::MicroAgent;
use skill_loader::SkillLoader;
use tool_registry::{ToolEntry, ToolExists, ToolRegistry};
use tracing_subscriber::EnvFilter;

/// A wrapper that delegates `ToolExists` checks to a shared `ToolRegistry`.
struct RegistryToolChecker(Arc<ToolRegistry>);

impl ToolExists for RegistryToolChecker {
    fn tool_exists(&self, name: &str) -> bool {
        self.0.tool_exists(name)
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::from_default_env()
                .add_directive(tracing::Level::INFO.into()),
        )
        .with_writer(std::io::stderr)
        .with_ansi(false)
        .init();

    // Step 1: Load configuration from environment
    tracing::info!("[1/7] Loading configuration");
    let config = RuntimeConfig::from_env()?;
    tracing::info!(
        skill_name = %config.skill_name,
        skill_dir = %config.skill_dir.display(),
        bind_addr = %config.bind_addr,
        "Configuration loaded"
    );

    // Step 2: Create and populate the tool registry
    tracing::info!("[2/7] Registering tool entries");
    let registry = Arc::new(ToolRegistry::new());
    register_tool_endpoints(&registry)?;

    // Step 3: Connect all tool servers
    tracing::info!("[3/7] Connecting to tool servers");
    registry.connect_all().await?;

    // Step 4: Load skill manifest
    tracing::info!("[4/7] Loading skill manifest");
    let tool_checker = RegistryToolChecker(registry.clone());
    let loader = SkillLoader::new(
        config.skill_dir,
        registry.clone(),
        Box::new(tool_checker),
    );
    let manifest = loader.load(&config.skill_name).await?;

    // Step 5: Build provider-backed agent
    tracing::info!("[5/7] Building agent");
    let agent = provider::build_agent(&manifest, &registry).await?;

    // Step 6: Wrap as MicroAgent
    tracing::info!("[6/7] Creating runtime agent");
    let runtime_agent = RuntimeAgent::new(manifest, agent, registry.clone());
    let micro_agent: Arc<dyn MicroAgent> = Arc::new(runtime_agent);

    // Step 7: Start HTTP server
    tracing::info!(bind_addr = %config.bind_addr, "[7/7] Starting HTTP server");
    http::start_server(micro_agent, config.bind_addr).await?;
    Ok(())
}

/// Parse the `TOOL_ENDPOINTS` environment variable and register each entry.
///
/// Expects a comma-separated list of `name=endpoint` pairs, e.g.
/// `echo-tool=mcp://localhost:7001,other=mcp://localhost:7002`.
/// Falls back to `echo-tool=mcp://localhost:7001` when the variable is unset.
fn register_tool_endpoints(
    registry: &ToolRegistry,
) -> Result<(), Box<dyn std::error::Error>> {
    let endpoints = std::env::var("TOOL_ENDPOINTS")
        .unwrap_or_else(|_| "echo-tool=mcp://localhost:7001".to_string());

    for pair in endpoints.split(',') {
        let pair = pair.trim();
        if pair.is_empty() {
            continue;
        }
        let (name, endpoint) = pair.split_once('=').ok_or_else(|| {
            format!("Invalid TOOL_ENDPOINTS entry: '{pair}' (expected name=endpoint)")
        })?;
        let entry = ToolEntry {
            name: name.trim().to_string(),
            version: "0.1.0".to_string(),
            endpoint: endpoint.trim().to_string(),
            action_type: None,
            handle: None,
        };
        tracing::info!(name = %entry.name, endpoint = %entry.endpoint, "Registering tool");
        registry.register(entry)?;
    }

    Ok(())
}
