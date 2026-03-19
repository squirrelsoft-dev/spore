use agent_sdk::AgentRequest;
use rig::embeddings::distance::VectorDistance;
use rig::embeddings::{Embedding, EmbeddingModel};

use crate::error::OrchestratorError;

struct AgentProfile {
    name: String,
    #[allow(dead_code)]
    description: String,
    description_embedding: Embedding,
}

pub struct SemanticRouter {
    agents: Vec<AgentProfile>,
    similarity_threshold: f64,
}

impl SemanticRouter {
    /// Creates a new `SemanticRouter` by pre-computing embeddings for each agent description.
    pub async fn new<M: EmbeddingModel>(
        model: &M,
        agents: Vec<(String, String)>,
        threshold: f64,
    ) -> Result<Self, OrchestratorError> {
        let mut profiles = Vec::with_capacity(agents.len());
        for (name, description) in agents {
            let embedding = embed_description(model, &description).await?;
            profiles.push(AgentProfile {
                name,
                description,
                description_embedding: embedding,
            });
        }
        Ok(Self {
            agents: profiles,
            similarity_threshold: threshold,
        })
    }

    /// Routes a request to the best matching agent using two-phase routing.
    ///
    /// Phase 1: Check `request.context` for an `"intent"` field and exact-match
    /// agent names case-insensitively.
    ///
    /// Phase 2: Embed `request.input` and compute cosine similarity against
    /// pre-computed description embeddings. Return the highest-scoring agent
    /// above `similarity_threshold`, or `OrchestratorError::NoRoute`.
    pub async fn route<M: EmbeddingModel>(
        &self,
        model: &M,
        request: &AgentRequest,
    ) -> Result<String, OrchestratorError> {
        if let Some(agent_name) = match_intent(&self.agents, &request.context) {
            tracing::debug!(agent = %agent_name, "routed via intent match");
            return Ok(agent_name);
        }

        let input_embedding = model
            .embed_text(&request.input)
            .await
            .map_err(|e| OrchestratorError::EmbeddingError {
                reason: e.to_string(),
            })?;

        if let Some(agent_name) =
            find_best_match(&input_embedding, &self.agents, self.similarity_threshold)
        {
            tracing::debug!(agent = %agent_name, "routed via semantic similarity");
            return Ok(agent_name);
        }

        Err(OrchestratorError::NoRoute {
            input: request.input.clone(),
        })
    }

    /// Registers a new agent by computing its description embedding.
    pub async fn register<M: EmbeddingModel>(
        &mut self,
        model: &M,
        name: String,
        description: String,
    ) -> Result<(), OrchestratorError> {
        let embedding = embed_description(model, &description).await?;
        self.agents.push(AgentProfile {
            name,
            description,
            description_embedding: embedding,
        });
        Ok(())
    }
}

/// Embeds a description string using the provided model.
async fn embed_description<M: EmbeddingModel>(
    model: &M,
    description: &str,
) -> Result<Embedding, OrchestratorError> {
    model
        .embed_text(description)
        .await
        .map_err(|e| OrchestratorError::EmbeddingError {
            reason: e.to_string(),
        })
}

/// Finds the agent with the highest cosine similarity to the input embedding,
/// returning its name only if the score strictly exceeds `threshold`.
fn find_best_match(
    input_embedding: &Embedding,
    agents: &[AgentProfile],
    threshold: f64,
) -> Option<String> {
    let mut best_score = threshold;
    let mut best_agent: Option<&str> = None;

    for agent in agents {
        let score =
            input_embedding.cosine_similarity(&agent.description_embedding, false);
        if score > best_score {
            best_score = score;
            best_agent = Some(&agent.name);
        }
    }

    best_agent.map(|name| name.to_string())
}

/// Extracts `context["intent"]` as a string and matches it case-insensitively
/// against registered agent names.
fn match_intent(
    agents: &[AgentProfile],
    context: &Option<serde_json::Value>,
) -> Option<String> {
    let ctx = context.as_ref()?;
    let intent = ctx.get("intent")?.as_str()?;

    agents
        .iter()
        .find(|agent| agent.name.eq_ignore_ascii_case(intent))
        .map(|agent| agent.name.clone())
}
