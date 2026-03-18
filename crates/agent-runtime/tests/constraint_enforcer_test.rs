use std::collections::HashMap;
use std::sync::Arc;

use agent_runtime::constraint_enforcer::ConstraintEnforcer;
use agent_sdk::{
    async_trait, AgentError, AgentRequest, AgentResponse, Constraints, HealthStatus, MicroAgent,
    ModelConfig, OutputSchema, SkillManifest,
};
use serde_json::json;

struct MockAgent {
    manifest: SkillManifest,
    response_confidence: f32,
    error_mode: Option<AgentError>,
    health_status: HealthStatus,
}

#[async_trait]
impl MicroAgent for MockAgent {
    fn manifest(&self) -> &SkillManifest {
        &self.manifest
    }

    async fn invoke(&self, request: AgentRequest) -> Result<AgentResponse, AgentError> {
        if let Some(err) = &self.error_mode {
            return Err(err.clone());
        }
        Ok(AgentResponse {
            id: request.id,
            output: json!({"result": "ok"}),
            confidence: self.response_confidence,
            escalated: false,
            escalate_to: None,
            tool_calls: vec![],
        })
    }

    async fn health(&self) -> HealthStatus {
        self.health_status.clone()
    }
}

fn make_manifest_with_threshold(threshold: f64, escalate_to: Option<String>) -> SkillManifest {
    SkillManifest {
        name: "test-agent".to_string(),
        version: "1.0.0".to_string(),
        description: "A mock agent for constraint enforcer tests".to_string(),
        model: ModelConfig {
            provider: "test-provider".to_string(),
            name: "test-model".to_string(),
            temperature: 0.7,
        },
        preamble: "You are a test agent.".to_string(),
        tools: vec![],
        constraints: Constraints {
            max_turns: 10,
            confidence_threshold: threshold,
            escalate_to,
            allowed_actions: vec![],
        },
        output: OutputSchema {
            format: "json".to_string(),
            schema: HashMap::new(),
        },
    }
}

#[tokio::test]
async fn confidence_above_threshold_passes_through_unchanged() {
    let mock = MockAgent {
        manifest: make_manifest_with_threshold(0.85, None),
        response_confidence: 0.95,
        error_mode: None,
        health_status: HealthStatus::Healthy,
    };
    let enforcer = ConstraintEnforcer::new(Arc::new(mock));

    let request = AgentRequest::new("test input".to_string());
    let response = enforcer.invoke(request).await.unwrap();

    assert!(!response.escalated);
    assert!(response.escalate_to.is_none());
    assert!((response.confidence - 0.95).abs() < f32::EPSILON);
}

#[tokio::test]
async fn confidence_below_threshold_triggers_escalation() {
    let mock = MockAgent {
        manifest: make_manifest_with_threshold(0.85, Some("fallback-agent".to_string())),
        response_confidence: 0.50,
        error_mode: None,
        health_status: HealthStatus::Healthy,
    };
    let enforcer = ConstraintEnforcer::new(Arc::new(mock));

    let request = AgentRequest::new("test input".to_string());
    let response = enforcer.invoke(request).await.unwrap();

    assert!(response.escalated);
    assert_eq!(response.escalate_to, Some("fallback-agent".to_string()));
    assert!((response.confidence - 0.50).abs() < f32::EPSILON);
}

#[tokio::test]
async fn confidence_below_threshold_without_escalate_to() {
    let mock = MockAgent {
        manifest: make_manifest_with_threshold(0.85, None),
        response_confidence: 0.50,
        error_mode: None,
        health_status: HealthStatus::Healthy,
    };
    let enforcer = ConstraintEnforcer::new(Arc::new(mock));

    let request = AgentRequest::new("test input".to_string());
    let response = enforcer.invoke(request).await.unwrap();

    assert!(response.escalated);
    assert!(response.escalate_to.is_none());
    assert!((response.confidence - 0.50).abs() < f32::EPSILON);
}

#[tokio::test]
async fn manifest_delegates_to_inner_agent() {
    let mock = MockAgent {
        manifest: make_manifest_with_threshold(0.85, Some("human".to_string())),
        response_confidence: 0.95,
        error_mode: None,
        health_status: HealthStatus::Healthy,
    };
    let enforcer = ConstraintEnforcer::new(Arc::new(mock));

    let manifest = enforcer.manifest();
    assert_eq!(manifest.name, "test-agent");
    assert_eq!(manifest.version, "1.0.0");
    assert!((manifest.constraints.confidence_threshold - 0.85).abs() < f64::EPSILON);
    assert_eq!(
        manifest.constraints.escalate_to,
        Some("human".to_string())
    );
}

#[tokio::test]
async fn health_delegates_to_inner_agent_healthy() {
    let mock = MockAgent {
        manifest: make_manifest_with_threshold(0.85, None),
        response_confidence: 0.95,
        error_mode: None,
        health_status: HealthStatus::Healthy,
    };
    let enforcer = ConstraintEnforcer::new(Arc::new(mock));

    assert_eq!(enforcer.health().await, HealthStatus::Healthy);
}

#[tokio::test]
async fn health_delegates_to_inner_agent_degraded() {
    let mock = MockAgent {
        manifest: make_manifest_with_threshold(0.85, None),
        response_confidence: 0.95,
        error_mode: None,
        health_status: HealthStatus::Degraded("high latency".to_string()),
    };
    let enforcer = ConstraintEnforcer::new(Arc::new(mock));

    assert_eq!(
        enforcer.health().await,
        HealthStatus::Degraded("high latency".to_string())
    );
}

#[tokio::test]
async fn inner_agent_error_propagates_unchanged() {
    let mock = MockAgent {
        manifest: make_manifest_with_threshold(0.85, None),
        response_confidence: 0.95,
        error_mode: Some(AgentError::Internal("mock failure".to_string())),
        health_status: HealthStatus::Healthy,
    };
    let enforcer = ConstraintEnforcer::new(Arc::new(mock));

    let request = AgentRequest::new("test input".to_string());
    let result = enforcer.invoke(request).await;

    assert!(result.is_err());
    assert_eq!(
        result.unwrap_err(),
        AgentError::Internal("mock failure".to_string())
    );
}
