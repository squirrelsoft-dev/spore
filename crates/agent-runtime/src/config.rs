use std::fmt;
use std::net::SocketAddr;
use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct RuntimeConfig {
    pub skill_name: String,
    pub skill_dir: PathBuf,
    pub bind_addr: SocketAddr,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ConfigError {
    MissingVar {
        name: String,
    },
    InvalidValue {
        name: String,
        value: String,
        reason: String,
    },
}

impl fmt::Display for ConfigError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ConfigError::MissingVar { name } => {
                write!(f, "missing required environment variable: '{name}'")
            }
            ConfigError::InvalidValue {
                name,
                value,
                reason,
            } => {
                write!(
                    f,
                    "invalid value for environment variable '{name}': '{value}' ({reason})"
                )
            }
        }
    }
}

impl std::error::Error for ConfigError {}

impl RuntimeConfig {
    pub fn from_env() -> Result<Self, ConfigError> {
        let skill_name = read_required_var("SKILL_NAME")?;
        let skill_dir = read_optional_var_or("SKILL_DIR", "./skills").into();
        let bind_addr = parse_bind_addr()?;

        Ok(Self {
            skill_name,
            skill_dir,
            bind_addr,
        })
    }
}

fn read_required_var(name: &str) -> Result<String, ConfigError> {
    match std::env::var(name) {
        Ok(val) if !val.is_empty() => Ok(val),
        _ => Err(ConfigError::MissingVar { name: name.into() }),
    }
}

fn read_optional_var_or(name: &str, default: &str) -> String {
    std::env::var(name)
        .ok()
        .filter(|v| !v.is_empty())
        .unwrap_or_else(|| default.to_string())
}

fn parse_bind_addr() -> Result<SocketAddr, ConfigError> {
    let default_addr: SocketAddr = "0.0.0.0:8080".parse().expect("valid default address");

    match std::env::var("BIND_ADDR") {
        Ok(val) if !val.is_empty() => {
            val.parse()
                .map_err(|e: std::net::AddrParseError| ConfigError::InvalidValue {
                    name: "BIND_ADDR".into(),
                    value: val,
                    reason: e.to_string(),
                })
        }
        _ => Ok(default_addr),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;
    use std::sync::Mutex;

    // Mutex to serialize tests that modify environment variables.
    static ENV_LOCK: Mutex<()> = Mutex::new(());

    fn with_env_vars<F>(vars: &[(&str, Option<&str>)], f: F)
    where
        F: FnOnce(),
    {
        let _guard = ENV_LOCK.lock().unwrap();
        let originals: Vec<(&str, Option<String>)> =
            vars.iter().map(|(k, _)| (*k, env::var(k).ok())).collect();

        for (k, v) in vars {
            // SAFETY: tests are serialized via ENV_LOCK so no concurrent access.
            match v {
                Some(val) => unsafe { env::set_var(k, val) },
                None => unsafe { env::remove_var(k) },
            }
        }

        f();

        for (k, original) in &originals {
            // SAFETY: tests are serialized via ENV_LOCK so no concurrent access.
            match original {
                Some(val) => unsafe { env::set_var(k, val) },
                None => unsafe { env::remove_var(k) },
            }
        }
    }

    #[test]
    fn from_env_with_all_vars_set() {
        with_env_vars(
            &[
                ("SKILL_NAME", Some("my-skill")),
                ("SKILL_DIR", Some("/tmp/skills")),
                ("BIND_ADDR", Some("127.0.0.1:9090")),
            ],
            || {
                let config = RuntimeConfig::from_env().unwrap();
                assert_eq!(config.skill_name, "my-skill");
                assert_eq!(config.skill_dir, PathBuf::from("/tmp/skills"));
                assert_eq!(
                    config.bind_addr,
                    "127.0.0.1:9090".parse::<SocketAddr>().unwrap()
                );
            },
        );
    }

    #[test]
    fn from_env_uses_defaults_when_optional_vars_unset() {
        with_env_vars(
            &[
                ("SKILL_NAME", Some("echo")),
                ("SKILL_DIR", None),
                ("BIND_ADDR", None),
            ],
            || {
                let config = RuntimeConfig::from_env().unwrap();
                assert_eq!(config.skill_name, "echo");
                assert_eq!(config.skill_dir, PathBuf::from("./skills"));
                assert_eq!(
                    config.bind_addr,
                    "0.0.0.0:8080".parse::<SocketAddr>().unwrap()
                );
            },
        );
    }

    #[test]
    fn from_env_missing_skill_name() {
        with_env_vars(
            &[
                ("SKILL_NAME", None),
                ("SKILL_DIR", None),
                ("BIND_ADDR", None),
            ],
            || {
                let err = RuntimeConfig::from_env().unwrap_err();
                assert_eq!(
                    err,
                    ConfigError::MissingVar {
                        name: "SKILL_NAME".into()
                    }
                );
            },
        );
    }

    #[test]
    fn from_env_empty_skill_name_treated_as_missing() {
        with_env_vars(
            &[
                ("SKILL_NAME", Some("")),
                ("SKILL_DIR", None),
                ("BIND_ADDR", None),
            ],
            || {
                let err = RuntimeConfig::from_env().unwrap_err();
                assert_eq!(
                    err,
                    ConfigError::MissingVar {
                        name: "SKILL_NAME".into()
                    }
                );
            },
        );
    }

    #[test]
    fn from_env_invalid_bind_addr() {
        with_env_vars(
            &[
                ("SKILL_NAME", Some("echo")),
                ("SKILL_DIR", None),
                ("BIND_ADDR", Some("not-an-address")),
            ],
            || {
                let err = RuntimeConfig::from_env().unwrap_err();
                match err {
                    ConfigError::InvalidValue { name, value, .. } => {
                        assert_eq!(name, "BIND_ADDR");
                        assert_eq!(value, "not-an-address");
                    }
                    other => panic!("expected InvalidValue, got {:?}", other),
                }
            },
        );
    }

    #[test]
    fn display_missing_var() {
        let err = ConfigError::MissingVar { name: "FOO".into() };
        assert_eq!(
            err.to_string(),
            "missing required environment variable: 'FOO'"
        );
    }

    #[test]
    fn display_invalid_value() {
        let err = ConfigError::InvalidValue {
            name: "BAR".into(),
            value: "bad".into(),
            reason: "could not parse".into(),
        };
        assert_eq!(
            err.to_string(),
            "invalid value for environment variable 'BAR': 'bad' (could not parse)"
        );
    }

    #[test]
    fn config_error_is_std_error() {
        let err: Box<dyn std::error::Error> =
            Box::new(ConfigError::MissingVar { name: "X".into() });
        assert!(!err.to_string().is_empty());
    }
}
