use rmcp::{
    handler::server::{router::tool::ToolRouter, wrapper::Parameters},
    model::{ServerCapabilities, ServerInfo},
    schemars, tool, tool_handler, tool_router, ServerHandler,
};

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct CargoBuildRequest {
    /// Package name to build (passed as -p <package>)
    pub package: String,
    /// Whether to build in release mode
    pub release: Option<bool>,
}

#[derive(Debug, Clone)]
pub struct CargoBuildTool {
    tool_router: ToolRouter<Self>,
}

impl CargoBuildTool {
    pub fn new() -> Self {
        Self {
            tool_router: Self::tool_router(),
        }
    }
}

fn validate_package_name(name: &str) -> bool {
    !name.is_empty()
        && name
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || ch == '-' || ch == '_')
}

#[tool_router]
impl CargoBuildTool {
    #[tool(description = "Run cargo build on a specified package and return the result")]
    fn cargo_build(&self, Parameters(request): Parameters<CargoBuildRequest>) -> String {
        if !validate_package_name(&request.package) {
            return serde_json::json!({
                "success": false,
                "stderr": format!("Invalid package name: {}", request.package)
            })
            .to_string();
        }

        let mut cmd = std::process::Command::new("cargo");
        cmd.args(["build", "-p", &request.package]);

        if request.release == Some(true) {
            cmd.arg("--release");
        }

        match cmd.output() {
            Ok(output) => serde_json::json!({
                "success": output.status.success(),
                "stdout": String::from_utf8_lossy(&output.stdout),
                "stderr": String::from_utf8_lossy(&output.stderr),
                "exit_code": output.status.code(),
            })
            .to_string(),
            Err(e) => serde_json::json!({
                "success": false,
                "stderr": format!("Failed to execute cargo: {e}"),
            })
            .to_string(),
        }
    }
}

#[tool_handler]
impl ServerHandler for CargoBuildTool {
    fn get_info(&self) -> ServerInfo {
        ServerInfo::new(ServerCapabilities::builder().enable_tools().build())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn call_cargo_build(tool: &CargoBuildTool, package: &str, release: Option<bool>) -> String {
        tool.cargo_build(Parameters(CargoBuildRequest {
            package: package.to_string(),
            release,
        }))
    }

    #[tokio::test]
    async fn rejects_invalid_package_name() {
        let tool = CargoBuildTool::new();
        let result = call_cargo_build(&tool, "foo; rm -rf /", None);
        let json: serde_json::Value = serde_json::from_str(&result).unwrap();
        assert_eq!(json["success"], false);
        assert!(json["stderr"].as_str().unwrap().contains("Invalid package name"));
    }

    #[tokio::test]
    async fn rejects_package_with_path_separator() {
        let tool = CargoBuildTool::new();
        let result = call_cargo_build(&tool, "../evil", None);
        let json: serde_json::Value = serde_json::from_str(&result).unwrap();
        assert_eq!(json["success"], false);
        assert!(json["stderr"].as_str().unwrap().contains("Invalid package name"));
    }

    #[tokio::test]
    async fn validates_clean_package_name() {
        let tool = CargoBuildTool::new();
        let result = call_cargo_build(&tool, "echo-tool", None);
        let json: serde_json::Value = serde_json::from_str(&result).unwrap();
        assert!(json.get("success").is_some());
        assert!(json.get("stdout").is_some());
        assert!(json.get("stderr").is_some());
        assert!(json.get("exit_code").is_some());
    }
}
