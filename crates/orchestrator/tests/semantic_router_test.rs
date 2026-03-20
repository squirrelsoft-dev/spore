use std::collections::HashMap;

use agent_sdk::AgentRequest;
use orchestrator::error::OrchestratorError;
use orchestrator::semantic_router::SemanticRouter;
use rig::embeddings::{Embedding, EmbeddingError, EmbeddingModel};
use serde_json::json;

// ---------------------------------------------------------------------------
// MockEmbeddingModel
// ---------------------------------------------------------------------------

/// A deterministic embedding model that returns pre-configured vectors for
/// known strings and a zero vector for anything else.
struct MockEmbeddingModel {
    vectors: HashMap<String, Vec<f64>>,
}

impl MockEmbeddingModel {
    fn new(vectors: HashMap<String, Vec<f64>>) -> Self {
        Self { vectors }
    }
}

impl EmbeddingModel for MockEmbeddingModel {
    const MAX_DOCUMENTS: usize = 100;
    type Client = ();

    fn make(_client: &Self::Client, _model: impl Into<String>, _dims: Option<usize>) -> Self {
        panic!("MockEmbeddingModel::make should not be called in tests")
    }

    fn ndims(&self) -> usize {
        3
    }

    async fn embed_texts(
        &self,
        texts: impl IntoIterator<Item = String> + Send,
    ) -> Result<Vec<Embedding>, EmbeddingError> {
        let embeddings = texts
            .into_iter()
            .map(|text| {
                let vec = self
                    .vectors
                    .get(&text)
                    .cloned()
                    .unwrap_or(vec![0.0, 0.0, 0.0]);
                Embedding {
                    document: text,
                    vec,
                }
            })
            .collect();
        Ok(embeddings)
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Builds the standard vector mappings used by most tests.
fn standard_vectors() -> HashMap<String, Vec<f64>> {
    let mut map = HashMap::new();
    map.insert("Handles financial queries".into(), vec![1.0, 0.0, 0.0]);
    map.insert("Handles weather forecasts".into(), vec![0.0, 1.0, 0.0]);
    map.insert("Handles travel bookings".into(), vec![0.0, 0.0, 1.0]);
    map.insert("What are my expenses?".into(), vec![0.9, 0.1, 0.0]);
    map.insert("Will it rain tomorrow?".into(), vec![0.1, 0.9, 0.0]);
    map.insert("random gibberish".into(), vec![0.33, 0.33, 0.33]);
    map.insert("Book a flight to Paris".into(), vec![0.05, 0.05, 0.95]);
    map.insert("Handles sports news".into(), vec![0.5, 0.5, 0.0]);
    map.insert("Latest soccer scores".into(), vec![0.45, 0.55, 0.0]);
    map
}

/// Builds a `SemanticRouter` with the three core agents (finance, weather, travel).
async fn build_standard_router(model: &MockEmbeddingModel) -> SemanticRouter {
    let agents = vec![
        ("finance-agent".into(), "Handles financial queries".into()),
        ("weather-agent".into(), "Handles weather forecasts".into()),
        ("travel-agent".into(), "Handles travel bookings".into()),
    ];
    SemanticRouter::new(model, agents, 0.7)
        .await
        .expect("failed to build standard router")
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[tokio::test]
async fn route_by_exact_intent_match() {
    let model = MockEmbeddingModel::new(standard_vectors());
    let router = build_standard_router(&model).await;

    let request = AgentRequest {
        id: uuid::Uuid::new_v4(),
        input: "anything".into(),
        context: Some(json!({"intent": "finance-agent"})),
        caller: None,
    };

    let result = router.route(&model, &request).await.unwrap();
    assert_eq!(result, "finance-agent");
}

#[tokio::test]
async fn route_by_semantic_similarity() {
    let model = MockEmbeddingModel::new(standard_vectors());
    let router = build_standard_router(&model).await;

    let request = AgentRequest {
        id: uuid::Uuid::new_v4(),
        input: "What are my expenses?".into(),
        context: None,
        caller: None,
    };

    let result = router.route(&model, &request).await.unwrap();
    assert_eq!(result, "finance-agent");
}

#[tokio::test]
async fn route_returns_no_route_below_threshold() {
    let model = MockEmbeddingModel::new(standard_vectors());
    let router = build_standard_router(&model).await;

    let request = AgentRequest {
        id: uuid::Uuid::new_v4(),
        input: "random gibberish".into(),
        context: None,
        caller: None,
    };

    let result = router.route(&model, &request).await;
    assert!(result.is_err());
    match result.unwrap_err() {
        OrchestratorError::NoRoute { .. } => {}
        other => panic!("expected NoRoute, got: {:?}", other),
    }
}

#[tokio::test]
async fn route_selects_highest_scoring_agent() {
    let model = MockEmbeddingModel::new(standard_vectors());
    let router = build_standard_router(&model).await;

    let request = AgentRequest {
        id: uuid::Uuid::new_v4(),
        input: "Book a flight to Paris".into(),
        context: None,
        caller: None,
    };

    let result = router.route(&model, &request).await.unwrap();
    assert_eq!(result, "travel-agent");
}

#[tokio::test]
async fn route_returns_no_route_for_empty_agents() {
    let model = MockEmbeddingModel::new(standard_vectors());
    let router = SemanticRouter::new(&model, vec![], 0.7)
        .await
        .expect("failed to build empty router");

    let request = AgentRequest {
        id: uuid::Uuid::new_v4(),
        input: "What are my expenses?".into(),
        context: None,
        caller: None,
    };

    let result = router.route(&model, &request).await;
    assert!(result.is_err());
    match result.unwrap_err() {
        OrchestratorError::NoRoute { .. } => {}
        other => panic!("expected NoRoute, got: {:?}", other),
    }
}

#[tokio::test]
async fn register_makes_agent_routable() {
    let model = MockEmbeddingModel::new(standard_vectors());
    let mut router = build_standard_router(&model).await;

    // Before registration, "Latest soccer scores" should not route to sports-agent.
    let request = AgentRequest {
        id: uuid::Uuid::new_v4(),
        input: "Latest soccer scores".into(),
        context: None,
        caller: None,
    };
    let pre_result = router.route(&model, &request).await;
    assert!(
        pre_result.is_err() || pre_result.as_ref().unwrap() != "sports-agent",
        "sports-agent should not be routable before registration"
    );

    // Register sports-agent
    router
        .register(&model, "sports-agent".into(), "Handles sports news".into())
        .await
        .expect("failed to register sports-agent");

    // After registration, "Latest soccer scores" should route to sports-agent.
    let request = AgentRequest {
        id: uuid::Uuid::new_v4(),
        input: "Latest soccer scores".into(),
        context: None,
        caller: None,
    };
    let result = router.route(&model, &request).await.unwrap();
    assert_eq!(result, "sports-agent");
}

#[tokio::test]
async fn route_intent_matching_is_case_insensitive() {
    let model = MockEmbeddingModel::new(standard_vectors());
    let router = build_standard_router(&model).await;

    let request = AgentRequest {
        id: uuid::Uuid::new_v4(),
        input: "anything".into(),
        context: Some(json!({"intent": "Finance-Agent"})),
        caller: None,
    };

    let result = router.route(&model, &request).await.unwrap();
    assert_eq!(result, "finance-agent");
}
