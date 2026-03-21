# Spec: Implement `SemanticRouter` struct with two-phase routing

> From: .claude/tasks/issue-16.md

## Objective

Create the `SemanticRouter` struct that replaces the orchestrator's placeholder substring-matching heuristic (`route_by_description_match`) with embedding-based cosine similarity routing. The router implements a two-phase strategy: (1) fast exact-match on an `"intent"` field in the request context, and (2) fallback to semantic similarity by embedding the request input and comparing it against pre-computed agent description embeddings using rig-core's `EmbeddingModel` trait.

This is the core component of issue-16 (semantic routing). All other tasks in the issue either prepare for it (error variants, dependencies) or build on top of it (orchestrator integration, tests).

## Current State

- **`crates/orchestrator/src/orchestrator.rs`**: The `Orchestrator` currently routes in two phases:
  - Phase 1 (`route_by_target_agent`): Checks `request.context["target_agent"]` for an exact agent name match.
  - Phase 2 (`route_by_description_match`): A substring heuristic that lowercases the input and agent descriptions, then checks if any description word (3+ chars) appears in the input. This is nondeterministic (HashMap iteration order) and low quality. The SemanticRouter will replace this phase.

- **`crates/orchestrator/src/error.rs`**: Defines `OrchestratorError` with variants `NoRoute`, `AgentUnavailable`, `EscalationFailed`, `HttpError`, and `Config`. Does NOT yet have an `EmbeddingError` variant (prerequisite task will add it).

- **`crates/orchestrator/src/lib.rs`**: Exports modules `agent_endpoint`, `config`, `error`, and `orchestrator`. Does NOT yet declare `semantic_router` (the integration task will add it).

- **`crates/orchestrator/Cargo.toml`**: Does NOT yet depend on `rig-core` (prerequisite task will add it).

- **`agent_sdk::AgentRequest`** (in `crates/agent-sdk/src/agent_request.rs`):
  ```rust
  pub struct AgentRequest {
      pub id: Uuid,
      pub input: String,
      pub context: Option<Value>,  // serde_json::Value
      pub caller: Option<String>,
  }
  ```
  The `context` field is an `Option<serde_json::Value>` -- a JSON object that may contain `"target_agent"` (used by the existing orchestrator) and `"intent"` (used by the new SemanticRouter).

- **`crates/orchestrator/src/agent_endpoint.rs`**: Defines `AgentEndpoint` with fields `name: String`, `description: String`, `url: String`, and a private `client: reqwest::Client`. The SemanticRouter does NOT use `AgentEndpoint` directly -- it works with `(name, description)` pairs and returns agent names as strings. The orchestrator integration task will bridge between the router's string output and the registry's `AgentEndpoint` lookup.

- **rig-core 0.32 embedding API** (from `/usr/local/cargo/registry/src/.../rig-core-0.32.0/src/embeddings/`):
  - `EmbeddingModel` trait (in `embedding.rs`): Has `embed_text(&self, text: &str) -> impl Future<Output = Result<Embedding, EmbeddingError>>` and `embed_texts(...)`. The trait requires `WasmCompatSend + WasmCompatSync` (which resolves to `Send + Sync` on non-WASM targets). It is NOT dyn-compatible because methods return `impl Future`.
  - `Embedding` struct (in `embedding.rs`): `{ document: String, vec: Vec<f64> }`. Derives `Clone`, `Default`, `Deserialize`, `Serialize`, `Debug`.
  - `EmbeddingError` enum (in `embedding.rs`): Variants include `HttpError`, `JsonError`, `UrlError`, `DocumentError`, `ResponseError`, `ProviderError`.
  - `VectorDistance` trait (in `distance.rs`): Implemented for `Embedding`. Provides `cosine_similarity(&self, other: &Self, normalized: bool) -> f64` where `normalized: false` computes the full formula (dot product / product of magnitudes).

## Requirements

1. Create a new file `crates/orchestrator/src/semantic_router.rs`.

2. Define a private `AgentProfile` struct with fields:
   - `name: String` -- the agent's unique name.
   - `description: String` -- the agent's human-readable description (kept for debugging/logging).
   - `description_embedding: Embedding` -- the pre-computed embedding of the description text.

3. Define a public `SemanticRouter` struct with fields:
   - `agents: Vec<AgentProfile>` -- the list of registered agents with their pre-computed embeddings.
   - `similarity_threshold: f64` -- the minimum cosine similarity score required for a semantic match.

4. Implement `SemanticRouter::new`:
   - Signature: `pub async fn new<M: EmbeddingModel>(model: &M, agents: Vec<(String, String)>, threshold: f64) -> Result<Self, OrchestratorError>`
   - Takes `(name, description)` pairs in the `agents` parameter.
   - Calls `model.embed_text(&description)` for each agent's description to pre-compute embeddings at construction time.
   - Maps rig-core's `EmbeddingError` to `OrchestratorError::EmbeddingError { reason }`.
   - Returns the constructed `SemanticRouter` with all description embeddings cached.

5. Implement `SemanticRouter::route`:
   - Signature: `pub async fn route<M: EmbeddingModel>(&self, model: &M, request: &AgentRequest) -> Result<String, OrchestratorError>`
   - **Phase 1 (exact intent match):** Check `request.context` for an `"intent"` key. If present and its string value (via `as_str()`) matches an agent name case-insensitively, return that agent's name immediately. If the value is not a string (e.g., number, object), skip to Phase 2.
   - **Phase 2 (semantic similarity):** Call `model.embed_text(&request.input)` to embed the request input. Call a helper function `find_best_match` to compute cosine similarity against all pre-computed description embeddings. If the highest similarity score exceeds `self.similarity_threshold`, return that agent's name. Otherwise, return `OrchestratorError::NoRoute { input: request.input.clone() }`.
   - The embedding model is passed by reference (not stored in the struct) to avoid making the struct generic and to sidestep rig-core's non-dyn-compatible trait.

6. Implement `SemanticRouter::register`:
   - Signature: `pub async fn register<M: EmbeddingModel>(&mut self, model: &M, name: String, description: String) -> Result<(), OrchestratorError>`
   - Embeds the description text using `model.embed_text(&description)`.
   - Pushes a new `AgentProfile` onto `self.agents`.
   - Maps `EmbeddingError` to `OrchestratorError::EmbeddingError { reason }`.

7. Implement a private helper function `find_best_match`:
   - Signature: `fn find_best_match(input_embedding: &Embedding, agents: &[AgentProfile], threshold: f64) -> Option<String>`
   - Iterates all agents, computing `input_embedding.cosine_similarity(&agent.description_embedding, false)` for each (using the `VectorDistance` trait).
   - Tracks the agent with the highest similarity score.
   - Returns `Some(name)` if the highest score exceeds `threshold`, otherwise `None`.
   - Must be under 50 lines per project rules.

8. Implement a private helper function for Phase 1 intent extraction:
   - Signature: `fn match_intent(agents: &[AgentProfile], context: &Option<serde_json::Value>) -> Option<String>`
   - Extracts `context["intent"]` as a string, compares case-insensitively against agent names.
   - Returns `Some(agent_name)` on match, `None` otherwise.
   - Must be under 50 lines per project rules.

9. All functions must be under 50 lines each.

10. No test module in this file. Tests are a separate task.

11. No commented-out code or debug statements.

12. Use `tracing::debug!` for logging routing decisions (intent match found, semantic match score, no route).

## Implementation Details

### File to create

**`crates/orchestrator/src/semantic_router.rs`**

**Imports needed:**
```rust
use agent_sdk::AgentRequest;
use rig::embeddings::{Embedding, EmbeddingModel};
use rig::embeddings::distance::VectorDistance;
use serde_json::Value;

use crate::error::OrchestratorError;
```
Note: The exact import paths for rig-core types must be verified at implementation time. `rig::embeddings::Embedding` is the most likely path based on `rig-core`'s module structure (`src/embeddings/embedding.rs` re-exported through `src/embeddings/mod.rs`). The crate is imported as `rig-core` in `Cargo.toml` but used as `rig_core` or `rig` in Rust code -- check how `agent-runtime` imports it (the `Cargo.toml` uses `rig-core = { version = "0.32" }` which becomes `rig_core` in Rust, but rig-core may re-export as `rig`).

**Struct definitions:**
```rust
struct AgentProfile {
    name: String,
    description: String,
    description_embedding: Embedding,
}

pub struct SemanticRouter {
    agents: Vec<AgentProfile>,
    similarity_threshold: f64,
}
```
`AgentProfile` is private to the module. `SemanticRouter` is public. Neither struct needs `Clone` or `Serialize`/`Deserialize` -- the `Embedding` struct inside `AgentProfile` is `Clone` but the router itself is not expected to be cloned.

**`new` implementation sketch:**
```rust
pub async fn new<M: EmbeddingModel>(
    model: &M,
    agents: Vec<(String, String)>,
    threshold: f64,
) -> Result<Self, OrchestratorError> {
    let mut profiles = Vec::with_capacity(agents.len());
    for (name, description) in agents {
        let embedding = model
            .embed_text(&description)
            .await
            .map_err(|e| OrchestratorError::EmbeddingError {
                reason: e.to_string(),
            })?;
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
```

**`route` implementation sketch:**
```rust
pub async fn route<M: EmbeddingModel>(
    &self,
    model: &M,
    request: &AgentRequest,
) -> Result<String, OrchestratorError> {
    if let Some(name) = match_intent(&self.agents, &request.context) {
        tracing::debug!(agent = %name, "routed via intent match");
        return Ok(name);
    }

    let input_embedding = model
        .embed_text(&request.input)
        .await
        .map_err(|e| OrchestratorError::EmbeddingError {
            reason: e.to_string(),
        })?;

    match find_best_match(&input_embedding, &self.agents, self.similarity_threshold) {
        Some(name) => {
            tracing::debug!(agent = %name, "routed via semantic similarity");
            Ok(name)
        }
        None => Err(OrchestratorError::NoRoute {
            input: request.input.clone(),
        }),
    }
}
```

**`find_best_match` implementation sketch:**
```rust
fn find_best_match(
    input_embedding: &Embedding,
    agents: &[AgentProfile],
    threshold: f64,
) -> Option<String> {
    let mut best_name: Option<&str> = None;
    let mut best_score = threshold;

    for agent in agents {
        let score = input_embedding
            .cosine_similarity(&agent.description_embedding, false);
        if score > best_score {
            best_score = score;
            best_name = Some(&agent.name);
        }
    }

    best_name.map(|n| n.to_string())
}
```
Note: `best_score` is initialized to `threshold` so that only agents exceeding the threshold are considered. If multiple agents tie, the first one encountered wins (deterministic because `Vec` has stable iteration order, unlike `HashMap`).

**`match_intent` implementation sketch:**
```rust
fn match_intent(
    agents: &[AgentProfile],
    context: &Option<Value>,
) -> Option<String> {
    let context = context.as_ref()?;
    let intent = context.get("intent")?.as_str()?;
    let intent_lower = intent.to_lowercase();

    agents
        .iter()
        .find(|a| a.name.to_lowercase() == intent_lower)
        .map(|a| a.name.clone())
}
```

### Key design decisions

1. **Model passed by reference, not stored:** The `EmbeddingModel` trait in rig-core 0.32 returns `impl Future` from its methods, making it NOT dyn-compatible (`Box<dyn EmbeddingModel>` is impossible). Storing the model as a generic type parameter on the struct (`SemanticRouter<M: EmbeddingModel>`) would propagate the generic through the entire `Orchestrator` type, complicating integration. Instead, the model is passed by reference to `new()`, `route()`, and `register()`. The tradeoff: the caller must keep the model alive and pass it to every call.

2. **Pre-computed description embeddings:** Agent descriptions are embedded once at construction/registration time and cached in `AgentProfile.description_embedding`. Only the request input needs embedding at routing time, keeping per-request latency to a single embedding API call.

3. **`Vec<AgentProfile>` not `HashMap`:** Using a `Vec` instead of a `HashMap` because: (a) the number of agents is small (dozens, not thousands), (b) we need to iterate all agents for similarity comparison anyway, (c) `Vec` has deterministic iteration order (unlike `HashMap`), making tie-breaking predictable.

4. **Cosine similarity with `normalized: false`:** rig-core's `VectorDistance::cosine_similarity` takes a `normalized` flag. We pass `false` because we cannot assume embeddings from the model are unit-normalized. With `false`, the full formula is used: dot product / (magnitude1 * magnitude2).

5. **Threshold comparison uses strict `>` not `>=`:** `find_best_match` uses `score > best_score` where `best_score` starts at `threshold`. This means an exact threshold match does NOT qualify -- the score must strictly exceed it. This avoids edge cases where floating-point imprecision at the boundary causes inconsistent routing.

6. **Error mapping:** rig-core's `EmbeddingError` is mapped to `OrchestratorError::EmbeddingError { reason: e.to_string() }`. This is a lossy conversion (the original error type is not preserved), but it avoids adding a rig-core dependency to the error type's public API. The string representation from rig-core's `Display` impl provides sufficient diagnostic information.

### Integration points

- **Upstream (rig-core):** Consumes `EmbeddingModel` trait, `Embedding` struct, `VectorDistance` trait, and `EmbeddingError` enum from rig-core 0.32.
- **Upstream (agent-sdk):** Uses `AgentRequest` to access `input` and `context` fields.
- **Upstream (error):** Uses `OrchestratorError::NoRoute` and `OrchestratorError::EmbeddingError` (the latter added by the prerequisite task).
- **Downstream (orchestrator):** The "Integrate `SemanticRouter` into `Orchestrator`" task will add `pub mod semantic_router;` to `lib.rs`, store an `Option<SemanticRouter>` in the `Orchestrator` struct, and call `router.route(model, request)` from `dispatch()`.

## Dependencies

- **Blocked by:**
  - "Add `EmbeddingError` variant to `OrchestratorError`" -- the `route()` and `new()` methods return `OrchestratorError::EmbeddingError`, which must exist first.
  - "Add `rig-core` dependency to orchestrator `Cargo.toml`" -- the file imports `rig_core` types (`Embedding`, `EmbeddingModel`, `VectorDistance`), which require the dependency to be present.
- **Blocking:**
  - "Integrate `SemanticRouter` into `Orchestrator`" -- the orchestrator integration task adds `pub mod semantic_router;` to `lib.rs` and uses the `SemanticRouter` inside `Orchestrator::dispatch()`.
  - "Write unit tests for `SemanticRouter`" -- the test task creates a mock `EmbeddingModel` and exercises `new()`, `route()`, and `register()`.

## Risks & Edge Cases

1. **rig-core import paths.** The exact Rust module paths for rig-core types (`Embedding`, `EmbeddingModel`, `VectorDistance`) may differ from the sketched imports. rig-core 0.32 uses `rig-core` as the crate name (becomes `rig_core` in Rust), but `agent-runtime` imports it as `rig-core` and the crate may re-export types at different paths. Mitigation: verify import paths by checking how `agent-runtime` uses rig-core, or by running `cargo doc -p rig-core --open` after the dependency is added.

2. **Empty agents list.** If `SemanticRouter::new` is called with an empty `agents` vec, the router will have no agents. Phase 1 and Phase 2 will both find nothing, and `route()` will always return `NoRoute`. This is valid behavior, not an error.

3. **Embedding API failures.** If the embedding model provider is unreachable or returns errors (rate limits, invalid API key), `new()` will fail at the first agent that cannot be embedded. This is fail-fast behavior -- if we cannot embed descriptions, the router is useless. Similarly, `route()` will fail if the input cannot be embedded. Both map to `OrchestratorError::EmbeddingError`.

4. **Zero-magnitude embeddings.** If an embedding model returns a zero vector for some input, `cosine_similarity` will produce `NaN` (division by zero). The `NaN > threshold` comparison will evaluate to `false`, so the agent will simply not match. This is safe but could be confusing if it happens silently. The `tracing::debug!` logging will help diagnose this.

5. **Case sensitivity of intent matching.** The spec requires case-insensitive matching for the `"intent"` field. The implementation lowercases both the intent value and agent names before comparison. This means an intent of `"Finance-Agent"` will match an agent named `"finance-agent"`. The returned name is the original (not lowercased) agent name from the profile.

6. **Intent field vs target_agent field.** The existing `Orchestrator.route_by_target_agent` checks `context["target_agent"]`. The `SemanticRouter` checks `context["intent"]`. These are different keys serving different purposes: `target_agent` is an explicit override (preserved in the orchestrator's fast path), while `intent` is a semantic hint that the router interprets. Both can coexist in the same request context without conflict.

7. **Deterministic tie-breaking.** When multiple agents have the same cosine similarity score above the threshold, `find_best_match` returns the first one encountered in the `Vec`. Since `Vec` iteration is stable and deterministic, this is reproducible. However, the "winner" depends on the order agents were registered. This is acceptable for the current scale (small number of agents).

8. **Threshold boundary.** The strict `>` comparison means a score of exactly 0.7 (with threshold 0.7) does NOT match. This is a deliberate design choice to avoid floating-point edge cases. If this proves too aggressive, the threshold can be lowered by the caller.

9. **Performance with many agents.** The `find_best_match` function iterates all agents linearly. For the expected scale (tens of agents), this is negligible. If the number of agents grows to thousands, a vector index (e.g., approximate nearest neighbor) would be needed, but that is out of scope.

## Verification

After implementation (and after both blocking prerequisite tasks are complete), run:

```bash
cargo check -p orchestrator
cargo clippy -p orchestrator
```

Both must pass with no errors and no warnings. (`cargo test -p orchestrator` may fail if the module is not yet wired into `lib.rs` -- that is done by the integration task.)

Additionally verify:

- The file `crates/orchestrator/src/semantic_router.rs` exists.
- `SemanticRouter` is `pub struct` with `agents: Vec<AgentProfile>` and `similarity_threshold: f64`.
- `AgentProfile` is a private struct (no `pub`) with `name`, `description`, and `description_embedding` fields.
- `new()` is `pub async fn` with generic `M: EmbeddingModel`, takes `(String, String)` pairs, and pre-computes embeddings.
- `route()` is `pub async fn` with generic `M: EmbeddingModel`, implements Phase 1 (intent match) then Phase 2 (semantic similarity).
- `register()` is `pub async fn` with generic `M: EmbeddingModel`, embeds the description and pushes to the agents list.
- `find_best_match` is a private function that computes cosine similarity and returns the best match above threshold.
- `match_intent` is a private function that extracts `context["intent"]` and compares case-insensitively.
- All functions are under 50 lines each.
- No test module, no commented-out code, no debug statements in the file.
- Error mapping from `EmbeddingError` to `OrchestratorError::EmbeddingError` is present in `new()`, `route()`, and `register()`.
- `tracing::debug!` is used for logging routing decisions.
