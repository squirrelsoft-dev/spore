use agent_sdk::{AgentRequest, AgentResponse, HealthStatus};
use serde::Deserialize;

use crate::error::OrchestratorError;

pub struct AgentEndpoint {
    pub name: String,
    pub description: String,
    pub url: String,
    client: reqwest::Client,
}

#[derive(Deserialize)]
struct HealthResponseDto {
    status: HealthStatus,
}

impl AgentEndpoint {
    pub fn new(
        name: impl Into<String>,
        description: impl Into<String>,
        url: impl Into<String>,
        client: reqwest::Client,
    ) -> Self {
        let url_string: String = url.into();
        Self {
            name: name.into(),
            description: description.into(),
            url: url_string.trim_end_matches('/').to_string(),
            client,
        }
    }

    pub async fn invoke(
        &self,
        request: &AgentRequest,
    ) -> Result<AgentResponse, OrchestratorError> {
        let invoke_url = format!("{}/invoke", self.url);
        let make_err = |reason: String| OrchestratorError::HttpError {
            url: invoke_url.clone(),
            reason,
        };

        let response = self
            .client
            .post(&invoke_url)
            .json(request)
            .send()
            .await
            .map_err(|e| make_err(e.to_string()))?;

        let response = response
            .error_for_status()
            .map_err(|e| make_err(e.to_string()))?;

        response
            .json::<AgentResponse>()
            .await
            .map_err(|e| make_err(e.to_string()))
    }

    pub async fn health(&self) -> Result<HealthStatus, OrchestratorError> {
        let health_url = format!("{}/health", self.url);
        let response = self
            .client
            .get(&health_url)
            .send()
            .await
            .map_err(|e| OrchestratorError::AgentUnavailable {
                name: self.name.clone(),
                reason: e.to_string(),
            })?;

        let response =
            response.error_for_status().map_err(|e| {
                OrchestratorError::AgentUnavailable {
                    name: self.name.clone(),
                    reason: e.to_string(),
                }
            })?;

        let dto: HealthResponseDto =
            response.json().await.map_err(|e| {
                OrchestratorError::AgentUnavailable {
                    name: self.name.clone(),
                    reason: e.to_string(),
                }
            })?;

        Ok(dto.status)
    }
}
