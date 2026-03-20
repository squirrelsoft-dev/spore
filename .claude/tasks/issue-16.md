# Task Breakdown: Implement semantic routing

> Replace the orchestrator's placeholder substring-matching heuristic with a `SemanticRouter` that first tries exact intent matching from request context, then falls back to embedding-based cosine similarity against agent descriptions using rig-core's embedding API.

## Group 1 — Foundation types and error handling

_Tasks in this group can be done in parallel._

- [x] **Add `EmbeddingError` variant to `OrchestratorError`** `[S]`
      Add a new variant `EmbeddingError { reason: String }` to the `OrchestratorError` enum in `crates/orchestrator/src/error.rs` to represent failures from the embedding model (network errors, provider errors, etc.). Update the `Display` impl with a corresponding format arm. This variant wraps rig-core's `EmbeddingError` into the orchestrator's error hierarchy so the semantic router can propagate embedding failures cleanly.
      Files: `crates/orchestrator/src/error.rs`
      Blocking: "Implement `SemanticRouter` struct with two-phase routing"

- [x] **Add `rig-core` dependency to orchestrator `Cargo.toml`** `[S]`
      Add `rig-core = { version = "0.32" }` to `[dependencies]` in `crates/orchestrator/Cargo.toml`. rig-core 0.32 is already in the workspace lockfile (used by `agent-runtime`), so this does not introduce a new crate to the dependency tree. The orchestrator needs rig-core for: `EmbeddingModel` trait, `Embedding` struct, `VectorDistance` trait (cosine similarity), and `EmbeddingError`. Also add `tracing = "0.1"` for logging (also already in the lockfile). Add `tokio = { version = "1", features = ["macros", "rt"] }` to dev-dependencies for async tests.
      Files: `crates/orchestrator/Cargo.toml`
      Blocking: "Implement `SemanticRouter` struct with two-phase routing"

## Group 2 — SemanticRouter implementation

_Depends on: Group 1._

- [x] **Implement `SemanticRouter` struct with two-phase routing** `[L]`
      Create `crates/orchestrator/src/semantic_router.rs` containing the `SemanticRouter` struct. This is the core of the issue.

      **Struct design:**
      ```rust
      pub struct SemanticRouter {
          agents: Vec<AgentProfile>,
          similarity_threshold: f64,
      }
      struct AgentProfile {
          name: String,
          description: String,
          description_embedding: Embedding,
      }
      ```
      The router stores pre-computed `Embedding` vectors so it does not need to be generic at the struct level. The embedding model is only needed at construction time (to embed agent descriptions) and at routing time (to embed the request input).

      **Public API:**
      - `async fn new<M: EmbeddingModel>(model: &M, agents: Vec<(String, String)>, threshold: f64) -> Result<Self, OrchestratorError>` — Pre-computes embeddings for each agent's description at construction time. The `agents` parameter is `(name, description)` pairs.
      - `async fn route<M: EmbeddingModel>(&self, model: &M, request: &AgentRequest) -> Result<String, OrchestratorError>` — Two-phase routing: (1) check `request.context` for `"intent"` field and exact-match against agent names, (2) embed `request.input` and compute cosine similarity against pre-computed description embeddings. Return the highest-scoring agent above `similarity_threshold`, or `OrchestratorError::NoRoute`.
      - `async fn register<M: EmbeddingModel>(&mut self, model: &M, name: String, description: String) -> Result<(), OrchestratorError>` — Add a new agent, computing its description embedding on the fly.

      **Phase 1 (exact match):** Check `request.context` for an `"intent"` key. If present and its string value matches an agent name (case-insensitive), return that agent immediately. This preserves the existing `target_agent` context key behavior from the orchestrator AND adds support for the `intent` key specified in the issue.

      **Phase 2 (semantic similarity):** Use rig-core's `EmbeddingModel::embed_text` to embed `request.input`. Compute cosine similarity (via `VectorDistance::cosine_similarity` with `normalized: false`) against each agent's pre-computed description embedding. Return the agent with the highest similarity if it exceeds `similarity_threshold`. Default threshold: 0.7 (configurable).

      **Cosine similarity helper:** Write a helper function `fn find_best_match(input_embedding: &Embedding, agents: &[AgentProfile], threshold: f64) -> Option<String>` that iterates agents, computes cosine similarity, and returns the best match. Keep this under 50 lines per project rules.

      **Key design decisions:**
      - The embedding model is passed by reference to `route()` rather than stored in the struct. This avoids the struct being generic (which would complicate the `Orchestrator` integration) and avoids the non-dyn-compatible `EmbeddingModel` trait issue. The tradeoff is that the caller must keep the model alive and pass it.
      - Pre-compute description embeddings at construction time to avoid re-embedding on every request.
      - Use `f64` vectors (rig-core's `Embedding.vec` is `Vec<f64>`).
      Files: `crates/orchestrator/src/semantic_router.rs`
      Blocked by: "Add `EmbeddingError` variant to `OrchestratorError`", "Add `rig-core` dependency to orchestrator `Cargo.toml`"
      Blocking: "Integrate `SemanticRouter` into `Orchestrator`", "Write unit tests for `SemanticRouter`"

## Group 3 — Orchestrator integration

_Depends on: Group 2._

- [x] **Integrate `SemanticRouter` into `Orchestrator`** `[M]`
      Modify `crates/orchestrator/src/orchestrator.rs` to use the `SemanticRouter` for routing instead of the current inline heuristic methods.

      **Changes to `Orchestrator` struct:**
      - Add a `semantic_router: Option<SemanticRouter>` field. It is `Option` because the router requires an embedding model to construct, and the orchestrator may be used without semantic routing (e.g., in tests or when no embedding model is configured).

      **Recommended integration pattern:**
      - Keep `route()` synchronous for backward compatibility. Add a new `async fn route_semantic<M: EmbeddingModel>(&self, model: &M, request: &AgentRequest) -> Result<&AgentEndpoint, OrchestratorError>` method.
      - Modify `dispatch()` to: first try `route_by_target_agent()` (keep this fast path), then try the `SemanticRouter` if present, then fall through to `NoRoute`. Remove `route_by_description_match()` (the substring heuristic it replaces).
      - Update `Orchestrator::new()` to optionally accept a `SemanticRouter`.
      - Update `Orchestrator::from_config()` to build the `SemanticRouter` if an embedding model provider is configured.

      **Key constraint:** The `route()` method signature change must be backward-compatible. Existing tests use `route()` which is synchronous. One approach: keep the synchronous `route()` as a convenience that only does exact matching, and use the semantic path only in `dispatch()` which is already async.

      Also update `crates/orchestrator/src/lib.rs` to add `pub mod semantic_router;`.
      Files: `crates/orchestrator/src/orchestrator.rs`, `crates/orchestrator/src/lib.rs`
      Blocked by: "Implement `SemanticRouter` struct with two-phase routing"
      Blocking: "Write integration tests for semantic routing in `Orchestrator`"

- [x] **Add embedding model configuration to `OrchestratorConfig`** `[S]`
      Extend `OrchestratorConfig` in `crates/orchestrator/src/config.rs` to include optional embedding model settings: `embedding_provider: Option<String>` (e.g., "openai"), `embedding_model: Option<String>` (e.g., "text-embedding-3-small"), and `similarity_threshold: Option<f64>` (defaults to 0.7). For env-based config, read from `EMBEDDING_PROVIDER`, `EMBEDDING_MODEL`, and `SIMILARITY_THRESHOLD` environment variables. These are all optional — if not set, the orchestrator falls back to exact-match-only routing (no semantic fallback).
      Files: `crates/orchestrator/src/config.rs`
      Blocked by: "Implement `SemanticRouter` struct with two-phase routing"
      Blocking: "Write integration tests for semantic routing in `Orchestrator`"

## Group 4 — Tests

_Depends on: Group 3._

- [x] **Write unit tests for `SemanticRouter`** `[M]`
      Create `crates/orchestrator/tests/semantic_router_test.rs`. Use a mock `EmbeddingModel` that returns deterministic embeddings to avoid real API calls.

      **Mock strategy:** Create a `MockEmbeddingModel` that maps known strings to fixed embedding vectors. For example, "financial queries" maps to `[1.0, 0.0, 0.0]` and "weather forecasts" maps to `[0.0, 1.0, 0.0]`. Input "What are my expenses?" maps to `[0.9, 0.1, 0.0]` (high similarity to financial). Input "random gibberish" maps to `[0.3, 0.3, 0.3]` (low similarity to everything).

      **Test cases:**
      1. Exact intent match: request with `context.intent = "finance-agent"` routes to the agent named `finance-agent`
      2. Semantic fallback: request input "What are my expenses?" routes to agent with description "Handles financial queries" (via cosine similarity)
      3. No match below threshold: request input with low similarity to all agents returns `OrchestratorError::NoRoute`
      4. Multiple agents: routes to the highest-scoring agent, not just the first above threshold
      5. Empty agents list returns `NoRoute`
      6. `register()` adds a new agent that becomes routable
      7. Case-insensitive intent matching
      Files: `crates/orchestrator/tests/semantic_router_test.rs`
      Blocked by: "Implement `SemanticRouter` struct with two-phase routing"
      Non-blocking

- [x] **Write integration tests for semantic routing in `Orchestrator`** `[M]`
      Update `crates/orchestrator/tests/orchestrator_test.rs` to add tests that exercise the semantic routing path through `dispatch()`.

      **Test cases:**
      1. `dispatch()` with `SemanticRouter` routes by intent context
      2. `dispatch()` with `SemanticRouter` routes by semantic similarity when no intent is provided
      3. `dispatch()` with `SemanticRouter` returns `NoRoute` when no agent matches semantically
      4. `dispatch()` with `SemanticRouter` still prioritizes `target_agent` context over intent-based routing
      5. Existing tests continue to pass (backward compatibility — orchestrator without `SemanticRouter` still works with exact target_agent matching)

      Use the same `MockEmbeddingModel` from the semantic router tests. Use the existing mock HTTP server pattern from the current `orchestrator_test.rs` for downstream agents.
      Files: `crates/orchestrator/tests/orchestrator_test.rs`
      Blocked by: "Integrate `SemanticRouter` into `Orchestrator`", "Add embedding model configuration to `OrchestratorConfig`"
      Non-blocking

- [x] **Write unit tests for config embedding fields** `[S]`
      Update `crates/orchestrator/tests/config_test.rs` to test the new embedding configuration fields: (1) YAML config with embedding settings parses correctly, (2) YAML config without embedding settings still parses (optional fields), (3) env-based config reads `EMBEDDING_PROVIDER` and `EMBEDDING_MODEL`, (4) `SIMILARITY_THRESHOLD` env var parses as f64, (5) missing embedding env vars result in `None` (not an error).
      Files: `crates/orchestrator/tests/config_test.rs`
      Blocked by: "Add embedding model configuration to `OrchestratorConfig`"
      Non-blocking

## Group 5 — Verification

_Depends on: Group 4._

- [x] **Run verification suite** `[S]`
      Run `cargo check`, `cargo clippy`, and `cargo test` across the full workspace. Verify no regressions in existing crates (`agent-sdk`, `agent-runtime`, `skill-loader`, `tool-registry`). Verify all new and existing orchestrator tests pass. Confirm that the `semantic_router` module compiles without warnings.
      Files: (none — command-line verification only)
      Blocked by: All other tasks

## Implementation Notes

1. **rig-core's `EmbeddingModel` trait is not dyn-compatible**: The `embed_text` and `embed_texts` methods return `impl Future`, which makes `Box<dyn EmbeddingModel>` impossible. The `SemanticRouter` works around this by accepting the model as a generic parameter at method call sites rather than storing it as a field.

2. **Only OpenAI provides embedding models in rig-core 0.32**: Anthropic's rig-core provider does not implement `EmbeddingsClient`. If the project needs non-OpenAI embeddings in the future, a local embedding library like `fastembed` could be added, but that is out of scope for this issue.

3. **Pre-computed description embeddings**: Agent description embeddings are computed once at construction/registration time and cached in the `SemanticRouter`. Only the request input needs to be embedded at routing time, keeping per-request latency to a single embedding API call.

4. **Backward compatibility**: The `Orchestrator` must continue to work without a `SemanticRouter` (it is `Option`). Existing tests that use `target_agent` context routing will continue to pass unchanged. The `route_by_description_match` substring heuristic is removed since the semantic router subsumes it.

5. **No new external dependencies**: `rig-core` 0.32 and `tracing` 0.1 are already in the workspace lockfile. No new crates are added to the dependency tree.

6. **Embedding vector type**: rig-core uses `Vec<f64>` for embedding vectors (not `Vec<f32>`). The implementation must use `f64` to match rig-core's `Embedding` struct.

7. **Similarity threshold**: Default to 0.7, configurable via `OrchestratorConfig`. The threshold applies only to the semantic fallback path — exact intent matches bypass it.

8. **Routing priority order**: (1) `context.target_agent` exact match (existing fast path in orchestrator), (2) `context.intent` exact match (new, in SemanticRouter), (3) embedding cosine similarity fallback (new, in SemanticRouter).
