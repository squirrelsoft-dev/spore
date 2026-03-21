# Spec: Write unit tests for `SemanticRouter`

> From: .claude/tasks/issue-16.md

## Objective

Create an integration test file that exercises all public methods of `SemanticRouter` (`new`, `route`, `register`) using a deterministic mock `EmbeddingModel`. These tests verify the two-phase routing logic (exact intent match, then cosine similarity fallback), error conditions, and edge cases without requiring a real embedding API provider. This is the primary correctness validation for the core semantic routing algorithm.

## Current State

- **`SemanticRouter` does not exist yet.** It will be implemented in `crates/orchestrator/src/semantic_router.rs` by the prerequisite task "Implement `SemanticRouter` struct with two-phase routing." The spec for that task (`implement-semantic-router-struct-with-two-phase-routing.md`) defines the public API:
  - `async fn new<M: EmbeddingModel>(model: &M, agents: Vec<(String, String)>, threshold: f64) -> Result<Self, OrchestratorError>`
  - `async fn route<M: EmbeddingModel>(&self, model: &M, request: &AgentRequest) -> Result<String, OrchestratorError>`
  - `async fn register<M: EmbeddingModel>(&mut self, model: &M, name: String, description: String) -> Result<(), OrchestratorError>`

- **`OrchestratorError`** (in `crates/orchestrator/src/error.rs`): Currently has `NoRoute { input: String }`, `AgentUnavailable`, `EscalationFailed`, `HttpError`, and `Config` variants. A prerequisite task will add `EmbeddingError { reason: String }`.

- **`AgentRequest`** (in `crates/agent-sdk/src/agent_request.rs`): Has fields `id: Uuid`, `input: String`, `context: Option<serde_json::Value>`, `caller: Option<String>`. Has a `new(input: String)` constructor.

- **rig-core 0.32 `EmbeddingModel` trait:**
  ```rust
  pub trait EmbeddingModel: Send + Sync {
      const MAX_DOCUMENTS: usize;
      type Client;
      fn make(client: &Self::Client, model: impl Into<String>, dims: Option<usize>) -> Self;
      fn ndims(&self) -> usize;
      fn embed_texts(
          &self,
          texts: impl IntoIterator<Item = String> + Send,
      ) -> impl Future<Output = Result<Vec<Embedding>, EmbeddingError>> + Send;
      fn embed_text(
          &self,
          text: &str,
      ) -> impl Future<Output = Result<Embedding, EmbeddingError>> + Send { /* default impl */ }
  }
  ```
  `Embedding` struct: `{ document: String, vec: Vec<f64> }`.

- **rig-core `VectorDistance` trait** (implemented for `Embedding`): Provides `cosine_similarity(&self, other: &Self, normalized: bool) -> f64`. With `normalized: false`, computes `dot_product / (magnitude1 * magnitude2)`.

- **Existing test patterns** (from `orchestrator_test.rs`, `error_test.rs`, `config_test.rs`):
  - Integration tests live in `crates/orchestrator/tests/`.
  - Async tests use `#[tokio::test]` with `tokio` in dev-dependencies (features: `macros`, `rt`, `net`).
  - Tests construct types directly and assert on return values.
  - Error matching uses `match` or `matches!()` with pattern destructuring.
  - No mocking framework -- mocks are hand-written structs.
  - Assertion messages provide context via custom `panic!` messages or `assert!` with format strings.

- **Orchestrator `Cargo.toml` dev-dependencies:**
  ```toml
  [dev-dependencies]
  tokio = { version = "1", features = ["macros", "rt", "net"] }
  axum = "0.8"
  uuid = { version = "1", features = ["v4"] }
  ```
  After the prerequisite tasks complete, `rig-core` will be in `[dependencies]`, making it available in integration tests.

## Requirements

### 1. `MockEmbeddingModel` struct

Create a `MockEmbeddingModel` struct in the test file that implements rig-core's `EmbeddingModel` trait. It must:

- Store a `HashMap<String, Vec<f64>>` mapping known input strings to fixed embedding vectors.
- In `embed_text`/`embed_texts`, look up the input string in the map. If found, return the corresponding vector as an `Embedding`. If not found, return a default "unknown" vector (e.g., `[0.0, 0.0, 0.0]`) rather than an error -- this keeps test setup simpler.
- Be `Send + Sync` (required by `EmbeddingModel`). Using `HashMap` is fine since the mock is immutable after construction.
- Implement all required associated items: `MAX_DOCUMENTS`, `type Client`, `make()`, `ndims()`, and `embed_texts()`. The `make()` and `Client` type are not used in tests but must exist for trait satisfaction.

### 2. Deterministic embedding vectors

Use 3-dimensional embedding vectors for simplicity. The mock maps these known strings to fixed vectors:

| String | Vector | Purpose |
|--------|--------|---------|
| `"Handles financial queries"` | `[1.0, 0.0, 0.0]` | Finance agent description |
| `"Handles weather forecasts"` | `[0.0, 1.0, 0.0]` | Weather agent description |
| `"Handles travel bookings"` | `[0.0, 0.0, 1.0]` | Travel agent description (for multi-agent test) |
| `"What are my expenses?"` | `[0.9, 0.1, 0.0]` | Input with high similarity to finance |
| `"Will it rain tomorrow?"` | `[0.1, 0.9, 0.0]` | Input with high similarity to weather |
| `"random gibberish"` | `[0.33, 0.33, 0.33]` | Input with low similarity to all agents |
| `"Book a flight to Paris"` | `[0.05, 0.05, 0.95]` | Input with high similarity to travel |
| `"Handles sports news"` | `[0.5, 0.5, 0.0]` | New agent description (for register test) |
| `"Latest soccer scores"` | `[0.45, 0.55, 0.0]` | Input with high similarity to sports (for register test) |

**Cosine similarity verification** (to confirm test vectors produce expected routing):
- `cos([0.9, 0.1, 0.0], [1.0, 0.0, 0.0])` = 0.9 / (sqrt(0.82) * 1.0) = 0.9 / 0.9055 ~ 0.9939 (well above 0.7 threshold)
- `cos([0.33, 0.33, 0.33], [1.0, 0.0, 0.0])` = 0.33 / (sqrt(0.3267) * 1.0) = 0.33 / 0.5716 ~ 0.5774 (below 0.7 threshold)
- `cos([0.33, 0.33, 0.33], [0.0, 1.0, 0.0])` = 0.33 / 0.5716 ~ 0.5774 (below 0.7 threshold)
- `cos([0.9, 0.1, 0.0], [0.0, 1.0, 0.0])` = 0.1 / 0.9055 ~ 0.1104 (correctly low)

### 3. Test case: exact intent match

Test name: `route_by_exact_intent_match`
- Create a `SemanticRouter` with two agents: `("finance-agent", "Handles financial queries")` and `("weather-agent", "Handles weather forecasts")`.
- Build an `AgentRequest` with `context: Some(json!({"intent": "finance-agent"}))` and any input text.
- Call `router.route(&mock_model, &request).await`.
- Assert the result is `Ok("finance-agent")`.
- This exercises Phase 1 (intent match). The input text does not matter because intent match short-circuits.

### 4. Test case: semantic fallback via cosine similarity

Test name: `route_by_semantic_similarity`
- Create a `SemanticRouter` with the same two agents.
- Build an `AgentRequest` with no `context` (or context without `"intent"`) and input `"What are my expenses?"`.
- Call `router.route(&mock_model, &request).await`.
- Assert the result is `Ok("finance-agent")`.
- This exercises Phase 2. The input embedding `[0.9, 0.1, 0.0]` has highest cosine similarity to the finance agent's `[1.0, 0.0, 0.0]`.

### 5. Test case: no match below threshold

Test name: `route_returns_no_route_below_threshold`
- Create a `SemanticRouter` with the two agents and threshold `0.7`.
- Build an `AgentRequest` with input `"random gibberish"` and no intent context.
- Call `router.route(&mock_model, &request).await`.
- Assert the result is `Err(OrchestratorError::NoRoute { .. })`.
- The embedding `[0.33, 0.33, 0.33]` has cosine similarity ~0.577 to all agents, below 0.7.

### 6. Test case: multiple agents routes to highest scorer

Test name: `route_selects_highest_scoring_agent`
- Create a `SemanticRouter` with three agents: `finance-agent`, `weather-agent`, and `travel-agent`.
- Build an `AgentRequest` with input `"Book a flight to Paris"` and no intent context.
- Call `router.route(&mock_model, &request).await`.
- Assert the result is `Ok("travel-agent")`.
- The embedding `[0.05, 0.05, 0.95]` has highest similarity to travel's `[0.0, 0.0, 1.0]`, not to finance or weather.

### 7. Test case: empty agents list returns NoRoute

Test name: `route_returns_no_route_for_empty_agents`
- Create a `SemanticRouter` with an empty agents vec.
- Build an `AgentRequest` with any input and no intent context.
- Call `router.route(&mock_model, &request).await`.
- Assert the result is `Err(OrchestratorError::NoRoute { .. })`.

### 8. Test case: register adds a routable agent

Test name: `register_makes_agent_routable`
- Create a `SemanticRouter` with an empty agents vec.
- Call `router.register(&mock_model, "sports-agent".into(), "Handles sports news".into()).await`.
- Assert the result is `Ok(())`.
- Build an `AgentRequest` with input `"Latest soccer scores"` and no intent.
- Call `router.route(&mock_model, &request).await`.
- Assert the result is `Ok("sports-agent")`.

### 9. Test case: case-insensitive intent matching

Test name: `route_intent_matching_is_case_insensitive`
- Create a `SemanticRouter` with `("finance-agent", "Handles financial queries")`.
- Build an `AgentRequest` with `context: Some(json!({"intent": "Finance-Agent"}))` (mixed case).
- Call `router.route(&mock_model, &request).await`.
- Assert the result is `Ok("finance-agent")` (returns the original-case name from the profile, not the lowercased intent).

## Implementation Details

### File to create

**`crates/orchestrator/tests/semantic_router_test.rs`**

### Imports

```rust
use std::collections::HashMap;

use agent_sdk::AgentRequest;
use orchestrator::error::OrchestratorError;
use orchestrator::semantic_router::SemanticRouter;
use rig::embeddings::embedding::{Embedding, EmbeddingError, EmbeddingModel};
use serde_json::json;
```

Note on rig-core import paths: The exact paths must be verified at implementation time. rig-core is imported as `rig-core` in `Cargo.toml` but the crate name in Rust is `rig` (based on `agent-runtime/src/provider.rs` using `use rig::...`). The `Embedding`, `EmbeddingModel`, and `EmbeddingError` types are in `rig::embeddings::embedding`. The `VectorDistance` trait (used internally by `SemanticRouter`, not by the tests) is in `rig::embeddings::distance`.

### `MockEmbeddingModel` definition

```rust
struct MockEmbeddingModel {
    embeddings: HashMap<String, Vec<f64>>,
    ndims: usize,
}

impl MockEmbeddingModel {
    fn new(embeddings: HashMap<String, Vec<f64>>, ndims: usize) -> Self {
        Self { embeddings, ndims }
    }
}

impl EmbeddingModel for MockEmbeddingModel {
    const MAX_DOCUMENTS: usize = 100;
    type Client = ();

    fn make(_client: &Self::Client, _model: impl Into<String>, _dims: Option<usize>) -> Self {
        panic!("MockEmbeddingModel::make is not used in tests")
    }

    fn ndims(&self) -> usize {
        self.ndims
    }

    async fn embed_texts(
        &self,
        texts: impl IntoIterator<Item = String> + Send,
    ) -> Result<Vec<Embedding>, EmbeddingError> {
        let results = texts
            .into_iter()
            .map(|text| {
                let vec = self
                    .embeddings
                    .get(&text)
                    .cloned()
                    .unwrap_or_else(|| vec![0.0; self.ndims]);
                Embedding {
                    document: text,
                    vec,
                }
            })
            .collect();
        Ok(results)
    }
}
```

Key design decisions for the mock:
- Unknown strings return a zero vector rather than an error. This avoids needing to pre-register every possible input string and makes test setup more forgiving. The zero vector has cosine similarity of `NaN` (0 / 0) with non-zero vectors, which evaluates to `false` in `>` comparisons, so it will never match. (Actually, rig-core's cosine_similarity with a zero-magnitude vector: magnitude is 0.0, so division by zero produces `NaN` or `inf`. The `>` comparison with `NaN` is always `false`, so no route will match, which is safe.)
- The `make()` method panics because it is never called in these tests (the mock is constructed directly).
- The mock is `Send + Sync` because `HashMap` is `Send + Sync` when keys and values are.

### Helper function for building the mock

```rust
fn build_test_model() -> MockEmbeddingModel {
    let mut embeddings = HashMap::new();
    embeddings.insert("Handles financial queries".into(), vec![1.0, 0.0, 0.0]);
    embeddings.insert("Handles weather forecasts".into(), vec![0.0, 1.0, 0.0]);
    embeddings.insert("Handles travel bookings".into(), vec![0.0, 0.0, 1.0]);
    embeddings.insert("What are my expenses?".into(), vec![0.9, 0.1, 0.0]);
    embeddings.insert("Will it rain tomorrow?".into(), vec![0.1, 0.9, 0.0]);
    embeddings.insert("random gibberish".into(), vec![0.33, 0.33, 0.33]);
    embeddings.insert("Book a flight to Paris".into(), vec![0.05, 0.05, 0.95]);
    embeddings.insert("Handles sports news".into(), vec![0.5, 0.5, 0.0]);
    embeddings.insert("Latest soccer scores".into(), vec![0.45, 0.55, 0.0]);
    MockEmbeddingModel::new(embeddings, 3)
}
```

### Helper function for building requests

```rust
fn build_request(input: &str, context: Option<serde_json::Value>) -> AgentRequest {
    AgentRequest {
        id: uuid::Uuid::new_v4(),
        input: input.to_string(),
        context,
        caller: None,
    }
}
```

### Test function signatures

All tests are `#[tokio::test] async fn`. Each test constructs the mock model via `build_test_model()`, builds a `SemanticRouter` via `SemanticRouter::new(&model, agents, threshold).await.unwrap()`, then exercises `route()` or `register()`.

Error assertions use `match` with pattern destructuring (consistent with `orchestrator_test.rs` style):
```rust
match result.unwrap_err() {
    OrchestratorError::NoRoute { .. } => {}
    other => panic!("expected NoRoute, got: {:?}", other),
}
```

### No changes to other files

This task only creates the new test file. It does not modify `Cargo.toml` (rig-core will already be in dependencies and the existing dev-dependencies are sufficient), `lib.rs`, or any source files.

## Dependencies

- **Blocked by:**
  - "Implement `SemanticRouter` struct with two-phase routing" -- the `SemanticRouter` type, its `new()`, `route()`, and `register()` methods must exist for the tests to compile.
  - "Add `rig-core` dependency to orchestrator `Cargo.toml`" -- the test file imports `rig::embeddings::embedding::{Embedding, EmbeddingError, EmbeddingModel}`, which requires rig-core in dependencies.
  - "Add `EmbeddingError` variant to `OrchestratorError`" -- the no-route tests assert against `OrchestratorError::NoRoute` and some tests may produce `OrchestratorError::EmbeddingError`.
  - The `semantic_router` module must be declared as `pub mod semantic_router;` in `crates/orchestrator/src/lib.rs` (done by the integration task) so that `orchestrator::semantic_router::SemanticRouter` is accessible from integration tests.
- **Blocking:**
  - "Run verification suite" -- the verification task depends on all tests existing and passing.

## Risks & Edge Cases

1. **rig-core import paths.** The exact Rust import paths for `Embedding`, `EmbeddingModel`, and `EmbeddingError` may differ from what is shown. rig-core re-exports types through its module hierarchy, and the crate name resolves to `rig` (not `rig_core`) based on how `agent-runtime` uses it. The implementer must verify paths by checking compilation or `cargo doc` output. Likely alternatives: `rig::embeddings::Embedding`, `rig::embeddings::EmbeddingModel`, `rig::embeddings::EmbeddingError`.

2. **`EmbeddingModel` trait associated types.** The `EmbeddingModel` trait has `type Client` and `fn make(client: &Self::Client, ...)`. The mock must provide both, but they are never called in tests. Using `type Client = ()` and a panicking `make()` is sufficient.

3. **`embed_texts` return type.** The `embed_texts` method receives `impl IntoIterator<Item = String> + Send`. The mock's implementation must handle this generic parameter. Using `.into_iter()` to iterate and collect into `Vec<Embedding>` is straightforward.

4. **`async fn` in trait impl.** rig-core 0.32's `EmbeddingModel` trait declares `embed_texts` as returning `impl Future<...>`. In Rust 2024 edition (which the orchestrator crate uses per `edition = "2024"` in Cargo.toml), `async fn` in trait impls is stabilized, so the mock can use `async fn embed_texts(...)` directly. If the rig-core trait uses `impl Future` return syntax instead of `async fn`, the implementer should verify that `async fn` in the impl block satisfies the trait bound.

5. **Zero-vector cosine similarity.** If an unknown input string is embedded (returning `[0.0, 0.0, 0.0]`), the cosine similarity computation will involve division by zero. In rig-core's implementation, `magnitude1` will be `0.0`, producing `NaN`. The comparison `NaN > threshold` is `false`, so no agent will match and `NoRoute` will be returned. This is safe behavior.

6. **Floating-point precision.** The test vectors are chosen so that similarity scores are clearly above or below the 0.7 threshold, avoiding boundary precision issues. The closest to the boundary is `cos([0.33, 0.33, 0.33], [1.0, 0.0, 0.0]) ~ 0.577`, which has a comfortable margin below 0.7.

7. **Threshold comparison strictness.** The `SemanticRouter` uses strict `>` (not `>=`) for threshold comparison. Test vectors are designed with this in mind -- no test relies on a score being exactly equal to the threshold.

8. **`pub mod semantic_router` visibility.** The test file imports `orchestrator::semantic_router::SemanticRouter`. This requires `semantic_router` to be declared as a `pub mod` in `crates/orchestrator/src/lib.rs`. If the integration task that adds this module declaration is not yet complete, the test file will fail to compile. The dependency graph ensures the integration task completes first, but the implementer should be aware of this ordering requirement.

9. **Mock returns original case for intent match.** The case-insensitive intent test asserts that the returned name is the original `"finance-agent"` (as stored in the `AgentProfile`), not the lowercased version from the request context. This verifies that the router preserves the canonical agent name.

## Verification

After implementation (and after all blocking prerequisite tasks are complete), run:

```bash
cargo test -p orchestrator --test semantic_router_test
cargo clippy -p orchestrator --tests
```

Specifically, confirm these 7 tests exist and pass:

- `semantic_router_test::route_by_exact_intent_match`
- `semantic_router_test::route_by_semantic_similarity`
- `semantic_router_test::route_returns_no_route_below_threshold`
- `semantic_router_test::route_selects_highest_scoring_agent`
- `semantic_router_test::route_returns_no_route_for_empty_agents`
- `semantic_router_test::register_makes_agent_routable`
- `semantic_router_test::route_intent_matching_is_case_insensitive`

Additionally verify:
- The `MockEmbeddingModel` compiles and satisfies the `EmbeddingModel` trait bound.
- No warnings from `cargo clippy`.
- No commented-out code or debug statements in the test file.
- All existing orchestrator tests (`orchestrator_test`, `error_test`, `config_test`, `agent_endpoint_test`) still pass -- no regressions.
