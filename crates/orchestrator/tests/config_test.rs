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
        ],
        || OrchestratorConfig::from_env(),
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
        &[("AGENT_ENDPOINTS", None), ("AGENT_DESCRIPTIONS", None)],
        || OrchestratorConfig::from_env(),
    );

    assert!(
        result.is_err(),
        "Expected error when AGENT_ENDPOINTS is not set"
    );
}
