use std::collections::HashMap;

use rmcp::{
    handler::server::{router::tool::ToolRouter, wrapper::Parameters},
    model::{ServerCapabilities, ServerInfo},
    schemars, tool, tool_handler, tool_router, ServerHandler,
};

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct ListAgentsRequest {
    /// Optional filter string to narrow results by name or description (case-insensitive substring match)
    pub filter: Option<String>,
}

#[derive(Debug, Clone)]
pub struct ListAgentsTool {
    tool_router: ToolRouter<Self>,
}

impl ListAgentsTool {
    pub fn new() -> Self {
        Self {
            tool_router: Self::tool_router(),
        }
    }
}

#[derive(Debug, Clone, serde::Serialize)]
struct AgentInfo {
    name: String,
    url: String,
    description: String,
}

/// Parse a raw AGENT_ENDPOINTS string into a list of (name, url) pairs.
///
/// Format: "name1=url1,name2=url2". Trims whitespace, rejects empty keys/values.
fn parse_endpoints(raw: &str) -> Result<Vec<(String, String)>, String> {
    raw.split(',')
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(|entry| {
            let (key, value) = entry
                .split_once('=')
                .ok_or_else(|| format!("invalid pair '{entry}', expected 'key=value'"))?;
            let key = key.trim();
            let value = value.trim();
            if key.is_empty() || value.is_empty() {
                return Err(format!("empty key or value in pair '{entry}'"));
            }
            Ok((key.to_string(), value.to_string()))
        })
        .collect()
}

/// Parse a raw AGENT_DESCRIPTIONS string into a map of name -> description.
///
/// Format: "name1=desc1,name2=desc2". Lenient: skips malformed pairs.
fn parse_descriptions(raw: &str) -> HashMap<String, String> {
    raw.split(',')
        .filter_map(|entry| {
            let (key, value) = entry.split_once('=')?;
            let key = key.trim().to_string();
            let value = value.trim().to_string();
            if key.is_empty() || value.is_empty() {
                return None;
            }
            Some((key, value))
        })
        .collect()
}

/// Build a list of AgentInfo from parsed endpoints and descriptions.
fn build_agent_list(
    endpoints: &[(String, String)],
    descriptions: &HashMap<String, String>,
) -> Vec<AgentInfo> {
    endpoints
        .iter()
        .map(|(name, url)| AgentInfo {
            name: name.clone(),
            url: url.clone(),
            description: descriptions.get(name).cloned().unwrap_or_default(),
        })
        .collect()
}

/// Filter agents by case-insensitive substring match on name and description.
fn filter_agents(agents: &[AgentInfo], filter: &str) -> Vec<AgentInfo> {
    let lower_filter = filter.to_lowercase();
    agents
        .iter()
        .filter(|agent| {
            agent.name.to_lowercase().contains(&lower_filter)
                || agent.description.to_lowercase().contains(&lower_filter)
        })
        .cloned()
        .collect()
}

#[tool_router]
impl ListAgentsTool {
    #[tool(description = "List registered agents, optionally filtered by name or description")]
    fn list_agents(&self, Parameters(request): Parameters<ListAgentsRequest>) -> String {
        let endpoints_raw = match std::env::var("AGENT_ENDPOINTS") {
            Ok(val) if !val.trim().is_empty() => val,
            _ => return serde_json::json!({"agents": []}).to_string(),
        };

        let endpoints = match parse_endpoints(&endpoints_raw) {
            Ok(eps) => eps,
            Err(msg) => {
                return serde_json::json!({"agents": [], "error": msg}).to_string();
            }
        };

        let descriptions = match std::env::var("AGENT_DESCRIPTIONS") {
            Ok(val) if !val.trim().is_empty() => parse_descriptions(&val),
            _ => HashMap::new(),
        };

        let agents = build_agent_list(&endpoints, &descriptions);

        let agents = match request.filter.as_deref() {
            Some(f) if !f.is_empty() => filter_agents(&agents, f),
            _ => agents,
        };

        serde_json::json!({"agents": agents}).to_string()
    }
}

#[tool_handler]
impl ServerHandler for ListAgentsTool {
    fn get_info(&self) -> ServerInfo {
        ServerInfo::new(ServerCapabilities::builder().enable_tools().build())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_empty_endpoints() {
        let result = parse_endpoints("");
        assert_eq!(result.unwrap(), vec![]);
    }

    #[test]
    fn test_single_agent() {
        let endpoints = parse_endpoints("builder=http://localhost:8001").unwrap();
        let descriptions = parse_descriptions("builder=Builds things");
        let agents = build_agent_list(&endpoints, &descriptions);

        assert_eq!(agents.len(), 1);
        assert_eq!(agents[0].name, "builder");
        assert_eq!(agents[0].url, "http://localhost:8001");
        assert_eq!(agents[0].description, "Builds things");
    }

    #[test]
    fn test_multiple_agents() {
        let endpoints =
            parse_endpoints("builder=http://localhost:8001,runner=http://localhost:8002").unwrap();
        let descriptions = parse_descriptions("builder=Builds things");
        let agents = build_agent_list(&endpoints, &descriptions);

        assert_eq!(agents.len(), 2);
        assert_eq!(agents[0].name, "builder");
        assert_eq!(agents[0].description, "Builds things");
        assert_eq!(agents[1].name, "runner");
        assert_eq!(agents[1].url, "http://localhost:8002");
        assert_eq!(agents[1].description, "");
    }

    #[test]
    fn test_filter_matches_name() {
        let agents = vec![
            AgentInfo {
                name: "builder".to_string(),
                url: "http://localhost:8001".to_string(),
                description: "Builds things".to_string(),
            },
            AgentInfo {
                name: "runner".to_string(),
                url: "http://localhost:8002".to_string(),
                description: "Runs tasks".to_string(),
            },
        ];

        let filtered = filter_agents(&agents, "build");
        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].name, "builder");
    }

    #[test]
    fn test_filter_case_insensitive() {
        let agents = vec![AgentInfo {
            name: "Builder".to_string(),
            url: "http://localhost:8001".to_string(),
            description: "Builds Things".to_string(),
        }];

        let filtered = filter_agents(&agents, "BUILD");
        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].name, "Builder");

        let filtered = filter_agents(&agents, "things");
        assert_eq!(filtered.len(), 1);
    }

    #[test]
    fn test_missing_descriptions() {
        let endpoints = parse_endpoints("agent1=http://a:1,agent2=http://b:2").unwrap();
        let descriptions = HashMap::new();
        let agents = build_agent_list(&endpoints, &descriptions);

        assert_eq!(agents.len(), 2);
        assert_eq!(agents[0].description, "");
        assert_eq!(agents[1].description, "");
    }
}
