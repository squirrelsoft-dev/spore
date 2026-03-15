# Spec: Add `serde`, `schemars` dependencies to `agent-sdk/Cargo.toml`

> From: .claude/tasks/issue-2.md

## Objective

Add the foundational serialization and schema-generation dependencies to the `agent-sdk` crate so that all subsequent type definitions (Group 2 and Group 3) can derive `Serialize`, `Deserialize`, and `JsonSchema`. Without these dependencies, none of the data-model work can proceed. This also adds `serde_yaml` as a dev-dependency so Group 4 tests can deserialize the README's canonical YAML skill file example.

## Current State

- `crates/agent-sdk/Cargo.toml` declares edition 2024 and has an empty `[dependencies]` section with no `[dev-dependencies]` section.
- `crates/agent-sdk/src/lib.rs` contains only a placeholder `add()` function and its test.
- The workspace root `Cargo.toml` uses resolver 2 and lists five member crates. It does not define any `[workspace.dependencies]`.
- The README tech stack table (line 111) explicitly names `serde` / `schemars` as the serialization layer for this project.

## Requirements

- Add `serde` as a dependency with the `derive` feature enabled, so structs can use `#[derive(Serialize, Deserialize)]` without a separate `serde_derive` crate.
- Add `schemars` as a dependency with the `derive` feature enabled, so structs can use `#[derive(JsonSchema)]`.
- Add a `[dev-dependencies]` section with `serde_yaml` for test-only YAML deserialization.
- Use specific version requirements (not wildcards) for all three crates to ensure reproducible builds.
- Do not add any other dependencies beyond these three.
- The crate must continue to compile after the changes (`cargo check -p agent-sdk` must succeed).

## Implementation Details

### File to modify

**`crates/agent-sdk/Cargo.toml`**

Add the following entries:

```toml
[dependencies]
serde = { version = "1", features = ["derive"] }
schemars = { version = "0.8", features = ["derive"] }

[dev-dependencies]
serde_yaml = "0.9"
```

### Version selection rationale

| Crate | Version | Reason |
|---|---|---|
| `serde` | `1` | Stable major version; the `1.x` line has been API-stable since 2017. The `derive` feature gates `#[derive(Serialize, Deserialize)]`. |
| `schemars` | `0.8` | Latest stable release line. The `derive` feature gates `#[derive(JsonSchema)]`. Note: `schemars` 1.0.0-alpha exists but is not yet stable. |
| `serde_yaml` | `0.9` | Latest stable release line for YAML serde support. Only needed in tests, so placed in `[dev-dependencies]`. |

### What does NOT change

- `src/lib.rs` is not modified in this task. The placeholder code remains until the Group 3 task "Update `lib.rs` module declarations and re-exports" runs.
- No workspace-level `[workspace.dependencies]` are added. This could be done as a future refactoring once other crates also depend on serde, but is out of scope here.
- No feature flags are exposed from `agent-sdk` itself.

## Dependencies

- **Blocked by:** Nothing. This is the first task in the issue-2 breakdown.
- **Blocking:** All tasks in Group 2 (`ModelConfig`, `Constraints`, `OutputSchema` struct definitions) and Group 3 (`SkillManifest` struct, `lib.rs` re-exports), since they all require `serde` and `schemars` derive macros to compile.

## Risks & Edge Cases

- **`schemars` 0.8 vs 1.0-alpha:** The `schemars` 1.0 alpha series changes the derive API. Pinning to `0.8` avoids breakage. If the project later needs JSON Schema draft-2020-12 support (only in 1.0), the migration path is to update the version and adjust derive attributes.
- **`serde_yaml` maintenance status:** `serde_yaml` is in maintenance mode; the author recommends considering alternatives. For this project's scope (test-only YAML parsing of small skill files), it remains the simplest choice. If it becomes unmaintained, `serde_yml` (a community fork) is a drop-in replacement.
- **Edition 2024 compatibility:** All three crates are compatible with Rust edition 2024. The edition primarily affects language-level features (e.g., `gen` keyword reservation), not library compatibility.
- **No code uses the dependencies yet:** After this task, `cargo check` will succeed but `cargo clippy` may emit "unused dependency" warnings depending on the clippy version. This is expected and will resolve once Group 2 tasks add `use` statements.

## Verification

1. Run `cargo check -p agent-sdk` -- must compile without errors.
2. Run `cargo build -p agent-sdk` -- must build without errors.
3. Inspect `Cargo.lock` -- must contain resolved entries for `serde`, `schemars`, and `serde_yaml` (and their transitive dependencies).
4. Confirm the `[dependencies]` section has exactly `serde` (with `derive` feature) and `schemars` (with `derive` feature).
5. Confirm the `[dev-dependencies]` section has exactly `serde_yaml`.
6. Run `cargo test -p agent-sdk` -- existing placeholder test must still pass.
