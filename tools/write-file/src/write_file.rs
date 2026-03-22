use std::fs;
use std::path::Path;

use rmcp::{
    handler::server::{router::tool::ToolRouter, wrapper::Parameters},
    model::{ServerCapabilities, ServerInfo},
    schemars, tool, tool_handler, tool_router, ServerHandler,
};

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct WriteFileRequest {
    /// The file path to write to
    pub path: String,
    /// The content to write to the file
    pub content: String,
}

#[derive(Debug, Clone)]
pub struct WriteFileTool {
    tool_router: ToolRouter<Self>,
}

impl WriteFileTool {
    pub fn new() -> Self {
        Self {
            tool_router: Self::tool_router(),
        }
    }
}

#[tool_router]
impl WriteFileTool {
    #[tool(description = "Write content to a file on disk, creating parent directories as needed")]
    fn write_file(&self, Parameters(request): Parameters<WriteFileRequest>) -> String {
        if request.path.is_empty() {
            return "Path must not be empty".to_string();
        }

        if let Some(parent) = Path::new(&request.path).parent()
            && !parent.as_os_str().is_empty()
            && let Err(e) = fs::create_dir_all(parent)
        {
            return format!(
                "Failed to create parent directories for {}: {}",
                request.path, e
            );
        }

        if let Err(e) = fs::write(&request.path, &request.content) {
            return format!("Failed to write to {}: {}", request.path, e);
        }

        format!("Wrote {} bytes to {}", request.content.len(), request.path)
    }
}

#[tool_handler]
impl ServerHandler for WriteFileTool {
    fn get_info(&self) -> ServerInfo {
        ServerInfo::new(ServerCapabilities::builder().enable_tools().build())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;
    use std::fs;

    fn call_write_file(tool: &WriteFileTool, path: &str, content: &str) -> String {
        tool.write_file(Parameters(WriteFileRequest {
            path: path.to_string(),
            content: content.to_string(),
        }))
    }

    fn unique_temp_dir(test_name: &str) -> std::path::PathBuf {
        let dir = env::temp_dir()
            .join("write_file_tests")
            .join(test_name)
            .join(format!("{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).expect("failed to create temp dir");
        dir
    }

    #[tokio::test]
    async fn write_file_creates_file_with_content() {
        let dir = unique_temp_dir("creates_file_with_content");
        let tool = WriteFileTool::new();
        let file_path = dir.join("output.txt");

        call_write_file(&tool, file_path.to_str().unwrap(), "hello world");

        let read_back = fs::read_to_string(&file_path).unwrap();
        assert_eq!(read_back, "hello world");
        let _ = fs::remove_dir_all(&dir);
    }

    #[tokio::test]
    async fn write_file_creates_parent_directories() {
        let dir = unique_temp_dir("creates_parent_dirs");
        let tool = WriteFileTool::new();
        let file_path = dir.join("a").join("b").join("c").join("file.txt");

        call_write_file(&tool, file_path.to_str().unwrap(), "nested content");

        let read_back = fs::read_to_string(&file_path).unwrap();
        assert_eq!(read_back, "nested content");
        let _ = fs::remove_dir_all(&dir);
    }

    #[tokio::test]
    async fn write_file_empty_path() {
        let tool = WriteFileTool::new();
        let result = call_write_file(&tool, "", "some content");
        assert_eq!(result, "Path must not be empty");
    }

    #[tokio::test]
    async fn write_file_returns_byte_count() {
        let dir = unique_temp_dir("returns_byte_count");
        let tool = WriteFileTool::new();
        let file_path = dir.join("count.txt");
        let content = "twelve chars";

        let result = call_write_file(&tool, file_path.to_str().unwrap(), content);

        let expected = format!("{}", content.len());
        assert!(
            result.contains(&expected),
            "Expected result to contain byte count {}, got: {}",
            expected,
            result
        );
        let _ = fs::remove_dir_all(&dir);
    }

    #[tokio::test]
    async fn write_file_overwrites_existing() {
        let dir = unique_temp_dir("overwrites_existing");
        let tool = WriteFileTool::new();
        let file_path = dir.join("overwrite.txt");

        call_write_file(&tool, file_path.to_str().unwrap(), "first content");
        call_write_file(&tool, file_path.to_str().unwrap(), "second content");

        let read_back = fs::read_to_string(&file_path).unwrap();
        assert_eq!(read_back, "second content");
        let _ = fs::remove_dir_all(&dir);
    }

    #[tokio::test]
    async fn write_file_preserves_unicode() {
        let dir = unique_temp_dir("preserves_unicode");
        let tool = WriteFileTool::new();
        let file_path = dir.join("unicode.txt");
        let content = "\u{1F600} hello \u{00E9}\u{00E8}\u{00EA} \u{4E16}\u{754C}";

        call_write_file(&tool, file_path.to_str().unwrap(), content);

        let read_back = fs::read_to_string(&file_path).unwrap();
        assert_eq!(read_back, content);
        let _ = fs::remove_dir_all(&dir);
    }
}
