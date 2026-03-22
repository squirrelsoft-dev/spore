use rmcp::{
    handler::server::{router::tool::ToolRouter, wrapper::Parameters},
    model::{ServerCapabilities, ServerInfo},
    schemars, tool, tool_handler, tool_router, ServerHandler,
};

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct ReadFileRequest {
    /// The path to the file to read (absolute or relative)
    pub path: String,
}

#[derive(Debug, Clone)]
pub struct ReadFileTool {
    tool_router: ToolRouter<Self>,
}

impl ReadFileTool {
    pub fn new() -> Self {
        Self {
            tool_router: Self::tool_router(),
        }
    }
}

#[tool_router]
impl ReadFileTool {
    #[tool(description = "Read the contents of a file from disk and return them as a string")]
    fn read_file(&self, Parameters(request): Parameters<ReadFileRequest>) -> String {
        match std::fs::read_to_string(&request.path) {
            Ok(content) => content,
            Err(e) => format!("Error reading '{}': {}", request.path, e),
        }
    }
}

#[tool_handler]
impl ServerHandler for ReadFileTool {
    fn get_info(&self) -> ServerInfo {
        ServerInfo::new(ServerCapabilities::builder().enable_tools().build())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn read_file_returns_content() {
        let path = std::env::temp_dir().join("read_file_test_content.txt");
        std::fs::write(&path, "hello from file").unwrap();
        let tool = ReadFileTool::new();
        let result = tool.read_file(Parameters(ReadFileRequest {
            path: path.to_string_lossy().to_string(),
        }));
        assert_eq!(result, "hello from file");
    }

    #[tokio::test]
    async fn read_file_returns_error_for_missing_file() {
        let tool = ReadFileTool::new();
        let result = tool.read_file(Parameters(ReadFileRequest {
            path: std::env::temp_dir()
                .join("read_file_nonexistent_abc123.txt")
                .to_string_lossy()
                .to_string(),
        }));
        assert!(result.contains("Error"));
    }

    #[tokio::test]
    async fn read_file_returns_error_for_directory() {
        let tool = ReadFileTool::new();
        let result = tool.read_file(Parameters(ReadFileRequest {
            path: std::env::temp_dir().to_string_lossy().to_string(),
        }));
        assert!(result.contains("Error"));
    }
}
