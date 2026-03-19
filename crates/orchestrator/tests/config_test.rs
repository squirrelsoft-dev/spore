use std::env;
use std::sync::Mutex;

use orchestrator::config::OrchestratorConfig;

/// Mutex to serialize tests that modify environment variables,
/// preventing parallel test conflicts.
static ENV_LOCK: Mutex<()> = Mutex::new(());

/// Runs `f` with the given environment variables set, then restores originals.
fn with_env_vars<F, R>(vars: &[(&str, Option<&str>)], f: F) -> R
where
    F: FnOnce() -> R,
{
    let _guard = ENV_LOCK.lock().unwrap();
    let originals: Vec<(&str, Option<String>)> =
        vars.iter().map(|(k, _)| (*k, env::var(k).ok())).collect();

    for (key, val) in vars {
        // SAFETY: tests are serialized via ENV_LOCK so no concurrent access.
        match val {
            Some(v) => unsafe { env::set_var(key, v) },
            None => unsafe { env::remove_var(key) },
        }
    }

    let result = f();

    for (key, original) in &originals {
        // SAFETY: tests are serialized via ENV_LOCK so no concurrent access.
        match original {
            Some(v) => unsafe { env::set_var(key, v) },
            None => unsafe { env::remove_var(key) },
        }
    }

    result
}

#[test]
fn yaml_config_parses_correctly() {
    let yaml = r#"
agents:
  - name: summarizer
    description: Summarizes text
    url: http://localhost:8081
  - name: translator
    description: Translates text
    url: http://localhost:8082
"#;

    let dir = std::env::temp_dir().join("orchestrator_test_yaml_parse");
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("config.yaml");
    std::fs::write(&path, yaml).unwrap();

    let config = OrchestratorConfig::from_file(path.to_str().unwrap()).unwrap();

    assert_eq!(config.agents.len(), 2);

    assert_eq!(config.agents[0].name, "summarizer");
    assert_eq!(config.agents[0].url, "http://localhost:8081");
    assert_eq!(config.agents[0].description, "Summarizes text");

    assert_eq!(config.agents[1].name, "translator");
    assert_eq!(config.agents[1].url, "http://localhost:8082");
    assert_eq!(config.agents[1].description, "Translates text");

    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn empty_agents_list_is_valid() {
    let yaml = "agents: []\n";

    let dir = std::env::temp_dir().join("orchestrator_test_empty_agents");
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("config.yaml");
    std::fs::write(&path, yaml).unwrap();

    let config = OrchestratorConfig::from_file(path.to_str().unwrap()).unwrap();

    assert!(config.agents.is_empty());

    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn malformed_yaml_returns_error() {
    let bad_yaml = "agents:\n  - name: [invalid\n    :\n";

    let dir = std::env::temp_dir().join("orchestrator_test_malformed_yaml");
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("config.yaml");
    std::fs::write(&path, bad_yaml).unwrap();

    let result = OrchestratorConfig::from_file(path.to_str().unwrap());

    assert!(result.is_err(), "Expected error for malformed YAML");
    assert!(
        matches!(
            result.unwrap_err(),
            orchestrator::error::OrchestratorError::Config { .. }
        ),
        "Expected OrchestratorError::Config variant"
    );

    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn env_config_parses_agent_endpoints() {
    let result = with_env_vars(
        &[
            (
                "AGENT_ENDPOINTS",
                Some("a=http://localhost:8081,b=http://localhost:8082"),
            ),
            ("AGENT_DESCRIPTIONS", Some("a=Desc A,b=Desc B")),
            ("EMBEDDING_PROVIDER", None),
            ("EMBEDDING_MODEL", None),
            ("SIMILARITY_THRESHOLD", None),
        ],
        OrchestratorConfig::from_env,
    );

    let config = result.unwrap();
    assert_eq!(config.agents.len(), 2);

    let agent_a = config.agents.iter().find(|a| a.name == "a").unwrap();
    assert_eq!(agent_a.url, "http://localhost:8081");
    assert_eq!(agent_a.description, "Desc A");

    let agent_b = config.agents.iter().find(|a| a.name == "b").unwrap();
    assert_eq!(agent_b.url, "http://localhost:8082");
    assert_eq!(agent_b.description, "Desc B");
}

#[test]
fn missing_env_var_returns_error() {
    let result = with_env_vars(
        &[
            ("AGENT_ENDPOINTS", None),
            ("AGENT_DESCRIPTIONS", None),
            ("EMBEDDING_PROVIDER", None),
            ("EMBEDDING_MODEL", None),
            ("SIMILARITY_THRESHOLD", None),
        ],
        OrchestratorConfig::from_env,
    );

    assert!(
        result.is_err(),
        "Expected error when AGENT_ENDPOINTS is not set"
    );
    assert!(
        matches!(
            result.unwrap_err(),
            orchestrator::error::OrchestratorError::Config { .. }
        ),
        "Expected OrchestratorError::Config variant"
    );
}

#[test]
fn yaml_config_with_embedding_settings_parses_correctly() {
    let yaml = r#"
agents:
  - name: summarizer
    description: Summarizes text
    url: http://localhost:8081
embedding_provider: openai
embedding_model: text-embedding-3-small
similarity_threshold: 0.75
"#;

    let dir = std::env::temp_dir().join("orchestrator_test_yaml_embedding");
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("config.yaml");
    std::fs::write(&path, yaml).unwrap();

    let config = OrchestratorConfig::from_file(path.to_str().unwrap()).unwrap();

    assert_eq!(config.agents.len(), 1);
    assert_eq!(config.agents[0].name, "summarizer");

    assert_eq!(config.embedding_provider.as_deref(), Some("openai"),);
    assert_eq!(
        config.embedding_model.as_deref(),
        Some("text-embedding-3-small"),
    );
    assert_eq!(config.similarity_threshold, Some(0.75));

    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn yaml_config_without_embedding_settings_parses() {
    let yaml = r#"
agents:
  - name: translator
    description: Translates text
    url: http://localhost:8082
"#;

    let dir = std::env::temp_dir().join("orchestrator_test_yaml_no_embedding");
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("config.yaml");
    std::fs::write(&path, yaml).unwrap();

    let config = OrchestratorConfig::from_file(path.to_str().unwrap()).unwrap();

    assert_eq!(config.agents.len(), 1);
    assert!(config.embedding_provider.is_none());
    assert!(config.embedding_model.is_none());
    assert!(config.similarity_threshold.is_none());

    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn env_config_reads_embedding_provider_and_model() {
    let result = with_env_vars(
        &[
            ("AGENT_ENDPOINTS", Some("svc=http://localhost:9000")),
            ("AGENT_DESCRIPTIONS", Some("svc=Test service")),
            ("EMBEDDING_PROVIDER", Some("openai")),
            ("EMBEDDING_MODEL", Some("text-embedding-3-small")),
            ("SIMILARITY_THRESHOLD", None),
        ],
        OrchestratorConfig::from_env,
    );

    let config = result.unwrap();
    assert_eq!(config.embedding_provider.as_deref(), Some("openai"));
    assert_eq!(
        config.embedding_model.as_deref(),
        Some("text-embedding-3-small"),
    );
    assert!(config.similarity_threshold.is_none());
}

#[test]
fn env_config_parses_similarity_threshold_as_f64() {
    let result = with_env_vars(
        &[
            ("AGENT_ENDPOINTS", Some("svc=http://localhost:9000")),
            ("AGENT_DESCRIPTIONS", Some("svc=Test service")),
            ("EMBEDDING_PROVIDER", None),
            ("EMBEDDING_MODEL", None),
            ("SIMILARITY_THRESHOLD", Some("0.85")),
        ],
        OrchestratorConfig::from_env,
    );

    let config = result.unwrap();
    assert_eq!(config.similarity_threshold, Some(0.85));
}

#[test]
fn missing_embedding_env_vars_result_in_none() {
    let result = with_env_vars(
        &[
            ("AGENT_ENDPOINTS", Some("svc=http://localhost:9000")),
            ("AGENT_DESCRIPTIONS", Some("svc=Test service")),
            ("EMBEDDING_PROVIDER", None),
            ("EMBEDDING_MODEL", None),
            ("SIMILARITY_THRESHOLD", None),
        ],
        OrchestratorConfig::from_env,
    );

    let config = result.unwrap();
    assert!(config.embedding_provider.is_none());
    assert!(config.embedding_model.is_none());
    assert!(config.similarity_threshold.is_none());
}

#[test]
fn invalid_similarity_threshold_returns_error() {
    let result = with_env_vars(
        &[
            ("AGENT_ENDPOINTS", Some("svc=http://localhost:9000")),
            ("AGENT_DESCRIPTIONS", Some("svc=Test service")),
            ("EMBEDDING_PROVIDER", None),
            ("EMBEDDING_MODEL", None),
            ("SIMILARITY_THRESHOLD", Some("not_a_number")),
        ],
        OrchestratorConfig::from_env,
    );

    assert!(
        result.is_err(),
        "Expected error for non-numeric SIMILARITY_THRESHOLD"
    );
    assert!(
        matches!(
            result.unwrap_err(),
            orchestrator::error::OrchestratorError::Config { .. }
        ),
        "Expected OrchestratorError::Config variant"
    );
}

#[test]
fn partial_embedding_env_vars_are_valid() {
    let result = with_env_vars(
        &[
            ("AGENT_ENDPOINTS", Some("svc=http://localhost:9000")),
            ("AGENT_DESCRIPTIONS", Some("svc=Test service")),
            ("EMBEDDING_PROVIDER", Some("openai")),
            ("EMBEDDING_MODEL", None),
            ("SIMILARITY_THRESHOLD", None),
        ],
        OrchestratorConfig::from_env,
    );

    let config = result.unwrap();
    assert_eq!(config.embedding_provider.as_deref(), Some("openai"));
    assert!(config.embedding_model.is_none());
    assert!(config.similarity_threshold.is_none());
}
