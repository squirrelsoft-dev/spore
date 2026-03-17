use std::collections::HashMap;
use std::sync::Arc;

use agent_runtime::provider;
use agent_runtime::runtime_agent::RuntimeAgent;
use agent_sdk::{
    AgentRequest, Constraints, HealthStatus, MicroAgent, ModelConfig, OutputSchema, SkillManifest,
};
use tool_registry::ToolRegistry;

/// Build a test `SkillManifest` with sensible defaults.
fn test_manifest() -> SkillManifest {
    SkillManifest {
        name: "test-agent".to_string(),
        version: "0.1.0".to_string(),
        description: "A test agent for integration tests".to_string(),
        model: ModelConfig {
            provider: "openai".to_string(),
            name: "gpt-4o-mini".to_string(),
            temperature: 0.0,
        },
        preamble: "You are a test agent.".to_string(),
        tools: vec![],
        constraints: Constraints {
            max_turns: 1,
            confidence_threshold: 0.5,
            escalate_to: None,
            allowed_actions: vec![],
        },
        output: OutputSchema {
            format: "text".to_string(),
            schema: HashMap::new(),
        },
    }
}

/// Build a `RuntimeAgent` using a fake OpenAI API key (no network calls).
///
/// The OpenAI client accepts any string as an API key at construction time;
/// validation only happens when a request is actually sent.
async fn build_test_runtime_agent() -> RuntimeAgent {
    // SAFETY: tests that call this helper are serialized by `#[tokio::test]`
    // default single-threaded runtime, so concurrent env mutation is avoided.
    unsafe {
        std::env::set_var("OPENAI_API_KEY", "sk-fake-test-key-for-integration-tests");
    }

    let manifest = test_manifest();
    let registry = Arc::new(ToolRegistry::new());
    let agent = provider::build_agent(&manifest, &registry)
        .await
        .expect("build_agent should succeed with a fake API key and no tools");

    RuntimeAgent::new(manifest, agent, registry)
}

#[tokio::test]
async fn test_manifest_returns_correct_values() {
    let runtime_agent = build_test_runtime_agent().await;
    let manifest = runtime_agent.manifest();

    assert_eq!(manifest.name, "test-agent");
    assert_eq!(manifest.version, "0.1.0");
    assert_eq!(manifest.description, "A test agent for integration tests");
    assert_eq!(manifest.model.provider, "openai");
    assert_eq!(manifest.model.name, "gpt-4o-mini");
    assert!((manifest.model.temperature - 0.0).abs() < f64::EPSILON);
    assert_eq!(manifest.preamble, "You are a test agent.");
    assert!(manifest.tools.is_empty());
    assert_eq!(manifest.constraints.max_turns, 1);
    assert!((manifest.constraints.confidence_threshold - 0.5).abs() < f64::EPSILON);
    assert!(manifest.constraints.escalate_to.is_none());
    assert!(manifest.constraints.allowed_actions.is_empty());
    assert_eq!(manifest.output.format, "text");
    assert!(manifest.output.schema.is_empty());
}

#[tokio::test]
async fn test_health_returns_healthy() {
    let runtime_agent = build_test_runtime_agent().await;
    let status = runtime_agent.health().await;

    assert_eq!(status, HealthStatus::Healthy);
}

#[tokio::test]
async fn test_runtime_agent_is_dyn_compatible() {
    let runtime_agent = build_test_runtime_agent().await;

    // This is primarily a compile-time check: RuntimeAgent must implement
    // MicroAgent in a way that is object-safe (dyn-compatible).
    let dyn_agent: Arc<dyn MicroAgent> = Arc::new(runtime_agent);

    // Verify the trait object is functional.
    assert_eq!(dyn_agent.manifest().name, "test-agent");
    assert_eq!(dyn_agent.health().await, HealthStatus::Healthy);
}

#[tokio::test]
#[ignore] // Requires a valid OPENAI_API_KEY environment variable.
async fn test_invoke_with_real_llm() {
    let api_key = std::env::var("OPENAI_API_KEY")
        .expect("OPENAI_API_KEY must be set to run this test");
    assert!(
        !api_key.is_empty(),
        "OPENAI_API_KEY must not be empty"
    );

    let manifest = test_manifest();
    let registry = Arc::new(ToolRegistry::new());
    let agent = provider::build_agent(&manifest, &registry)
        .await
        .expect("build_agent should succeed with a real API key");

    let runtime_agent = RuntimeAgent::new(manifest, agent, registry);
    let request = AgentRequest::new("Say hello in exactly one word.".to_string());
    let response = runtime_agent
        .invoke(request)
        .await
        .expect("invoke should succeed with a real LLM");

    assert!(
        !response.output.is_null(),
        "response output should not be null"
    );
}
