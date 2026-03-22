pub use rmcp::ServerHandler;

use rmcp::ServiceExt;
use tracing_subscriber::{self, EnvFilter};

/// Serve an MCP tool over stdin/stdout.
///
/// Initializes tracing to stderr (with ANSI disabled), logs a startup message,
/// then serves the given `tool` as an MCP server on stdio and waits for it to
/// finish.
pub async fn serve_stdio_tool<T: ServerHandler>(
    tool: T,
    tool_name: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::from_default_env().add_directive(tracing::Level::DEBUG.into()),
        )
        .with_writer(std::io::stderr)
        .with_ansi(false)
        .init();

    tracing::info!("Starting {tool_name} MCP server");

    let service = tool
        .serve(rmcp::transport::stdio())
        .await
        .inspect_err(|e| tracing::error!("serving error: {:?}", e))?;

    service.waiting().await?;
    Ok(())
}
