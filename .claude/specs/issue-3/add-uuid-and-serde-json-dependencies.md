# Spec: Add `uuid` and `serde_json` dependencies to `agent-sdk/Cargo.toml`

> From: .claude/tasks/issue-3.md

## Objective

Add the `uuid` and `serde_json` crates as dependencies to the `agent-sdk` crate so that the issue #4 envelope types can use `uuid::Uuid` for request identifiers and `serde_json::Value` for unstructured JSON payloads. Without these dependencies, the Group 2 type definitions (`AgentRequest`, `AgentResponse`, `ToolCallRecord`) cannot compile.

## Current State

- `crates/agent-sdk/Cargo.toml` declares edition 2024 and has two production dependencies (`serde` with `derive` feature, `schemars` with `derive` feature) and one dev-dependency (`serde_yaml`). These were added as part of issue #2.
- `crates/agent-sdk/src/lib.rs` contains the `SkillManifest` and related config types from issue #2, all of which use `serde` and `schemars` derive macros.
- The workspace root `Cargo.toml` uses resolver 2 and lists five member crates. It does not define any `[workspace.dependencies]`.
- No code in the crate currently references `uuid` or `serde_json`.

## Requirements

- Add `uuid` as a dependency with version `"1"` and features `["v4", "serde"]`.
  - The `v4` feature is required so `AgentRequest::new()` can generate random UUIDs via `Uuid::new_v4()`.
  - The `serde` feature is required so `Uuid` fields can be serialized/deserialized when embedded in structs that derive `Serialize`/`Deserialize`.
- Add `serde_json` as a dependency with version `"1"`.
  - This provides `serde_json::Value`, which is used for unstructured JSON fields in `AgentRequest.context`, `AgentResponse.output`, `ToolCallRecord.input`, and `ToolCallRecord.output`.
- Do not add any other dependencies beyond these two.
- Do not modify any existing dependencies or their feature sets.
- Do not modify `[dev-dependencies]`.
- The crate must continue to compile after the changes (`cargo check -p agent-sdk` must succeed).

## Implementation Details

### File to modify

**`crates/agent-sdk/Cargo.toml`**

Add the following entries to the existing `[dependencies]` section:

```toml
[dependencies]
serde = { version = "1", features = ["derive"] }
schemars = { version = "0.8", features = ["derive"] }
uuid = { version = "1", features = ["v4", "serde"] }
serde_json = "1"
```

The `serde` and `schemars` lines already exist and must not be modified. Only the `uuid` and `serde_json` lines are new.

### Version selection rationale

| Crate | Version | Reason |
|---|---|---|
| `uuid` | `1` | Stable major version; the `1.x` line has been API-stable since its release. The `v4` feature enables random UUID generation (`Uuid::new_v4()`). The `serde` feature enables `Serialize`/`Deserialize` implementations on `Uuid`, which is required because `AgentRequest` derives both traits and contains a `Uuid` field. |
| `serde_json` | `1` | Stable major version; the de facto standard for JSON serialization in Rust. Provides `serde_json::Value` for representing arbitrary JSON, used in multiple envelope type fields where the schema is intentionally dynamic. |

### Feature selection rationale for `uuid`

| Feature | Why needed |
|---|---|
| `v4` | `AgentRequest::new(input)` constructor must generate a random UUID v4 as the request identifier. Without this feature, `Uuid::new_v4()` is not available. |
| `serde` | `AgentRequest` derives `Serialize` and `Deserialize`. Any field type within the struct must also implement these traits. Without the `serde` feature on `uuid`, the `id: Uuid` field would cause a compile error. |

### What does NOT change

- `src/lib.rs` is not modified in this task. No `use` statements are added until the Group 2 tasks create the types that consume these dependencies.
- The `[dev-dependencies]` section is not modified. `serde_json` is placed in `[dependencies]` (not dev-dependencies) because production types use `serde_json::Value` in their field definitions.
- No workspace-level `[workspace.dependencies]` are added. This could be done as a future refactoring once other crates also depend on these crates, but is out of scope here.
- No feature flags are exposed from `agent-sdk` itself.
- Existing dependencies (`serde`, `schemars`) and dev-dependencies (`serde_yaml`) are unchanged.

## Dependencies

- **Blocked by:** Nothing. This is a Group 1 task and can be done in parallel with the "Add `async-trait` dependency" task.
- **Blocking:** All issue #4 type definitions in Group 2:
  - `AgentRequest` struct (uses `uuid::Uuid` for the `id` field, `serde_json::Value` for the `context` field)
  - `AgentResponse` struct (uses `serde_json::Value` for the `output` field)
  - `ToolCallRecord` struct (uses `serde_json::Value` for the `input` and `output` fields)

## Risks & Edge Cases

- **`uuid` v1 vs v2:** The `uuid` crate is at major version 1.x and is API-stable. There is no v2 on the horizon. Pinning to `"1"` is the correct choice.
- **`serde_json` and `schemars` interop:** `schemars` 0.8 uses `serde_json` internally for its `Schema` type. Adding `serde_json` as a direct dependency will not cause version conflicts because `schemars` 0.8 already depends on `serde_json` 1.x, so Cargo will unify to a single version.
- **No code uses the dependencies yet:** After this task, `cargo check` will succeed but `cargo clippy` may emit "unused dependency" warnings depending on the clippy version. This is expected and will resolve once Group 2 tasks add types that reference `uuid::Uuid` and `serde_json::Value`.
- **Edition 2024 compatibility:** Both `uuid` 1.x and `serde_json` 1.x are compatible with Rust edition 2024. The edition primarily affects language-level features, not library compatibility.

## Verification

1. Run `cargo check -p agent-sdk` -- must compile without errors.
2. Run `cargo build -p agent-sdk` -- must build without errors.
3. Inspect `Cargo.lock` -- must contain resolved entries for `uuid` (with `v4` and `serde` features) and `serde_json` (and their transitive dependencies).
4. Confirm the `[dependencies]` section contains exactly four entries: `serde`, `schemars`, `uuid`, and `serde_json`.
5. Confirm the `[dev-dependencies]` section is unchanged (still contains only `serde_yaml`).
6. Run `cargo test -p agent-sdk` -- all existing tests must still pass.
