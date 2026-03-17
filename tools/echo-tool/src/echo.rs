use rmcp::{
    handler::server::{router::tool::ToolRouter, wrapper::Parameters},
    model::{ServerCapabilities, ServerInfo},
    schemars, tool, tool_handler, tool_router, ServerHandler,
};

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct EchoRequest {
    /// The message to echo back
    pub message: String,
}

#[derive(Debug, Clone)]
pub struct EchoTool {
    tool_router: ToolRouter<Self>,
}

impl EchoTool {
    pub fn new() -> Self {
        Self {
            tool_router: Self::tool_router(),
        }
    }
}

#[tool_router]
impl EchoTool {
    #[tool(description = "Returns the input message unchanged")]
    fn echo(&self, Parameters(request): Parameters<EchoRequest>) -> String {
        request.message
    }
}

#[tool_handler]
impl ServerHandler for EchoTool {
    fn get_info(&self) -> ServerInfo {
        ServerInfo::new(ServerCapabilities::builder().enable_tools().build())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn call_echo(tool: &EchoTool, message: &str) -> String {
        tool.echo(Parameters(EchoRequest {
            message: message.to_string(),
        }))
    }

    #[tokio::test]
    async fn echo_returns_input_unchanged() {
        let tool = EchoTool::new();
        let result = call_echo(&tool, "hello world");
        assert_eq!(result, "hello world");
    }

    #[tokio::test]
    async fn echo_returns_empty_string() {
        let tool = EchoTool::new();
        let result = call_echo(&tool, "");
        assert_eq!(result, "");
    }

    #[tokio::test]
    async fn echo_preserves_whitespace_and_special_chars() {
        let tool = EchoTool::new();
        let input = "  line1\nline2\ttab  ";
        let result = call_echo(&tool, input);
        assert_eq!(result, input);
    }

    #[tokio::test]
    async fn echo_preserves_unicode() {
        let tool = EchoTool::new();
        let input = "\u{1F600} hello \u{00E9}\u{00E8}\u{00EA} \u{4E16}\u{754C}";
        let result = call_echo(&tool, input);
        assert_eq!(result, input);
    }

    #[tokio::test]
    async fn echo_result_is_success() {
        let tool = EchoTool::new();
        let result = call_echo(&tool, "test");
        assert!(!result.is_empty(), "echo should return a non-empty result for non-empty input");
    }
}
