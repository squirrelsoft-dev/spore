use tool_registry::{RegistryError, ToolEntry};

fn make_tool_entry() -> ToolEntry {
    ToolEntry {
        name: "my-tool".to_string(),
        version: "1.0.0".to_string(),
        endpoint: "http://localhost:8080".to_string(),
    }
}

// ── ToolEntry tests ──────────────────────────────────────────────────

#[test]
fn json_round_trip() {
    let entry = make_tool_entry();
    let json = serde_json::to_string(&entry).expect("serialize");
    let deserialized: ToolEntry = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(entry, deserialized);
}

#[test]
fn json_round_trip_unix_socket() {
    let entry = ToolEntry {
        name: "unix-tool".to_string(),
        version: "2.3.1".to_string(),
        endpoint: "unix:///var/run/tool.sock".to_string(),
    };
    let json = serde_json::to_string(&entry).expect("serialize");
    let deserialized: ToolEntry = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(entry, deserialized);
}

#[test]
fn equality() {
    let a = make_tool_entry();
    let b = make_tool_entry();
    assert_eq!(a, b);
}

#[test]
fn inequality_by_name() {
    let a = make_tool_entry();
    let b = ToolEntry {
        name: "other-tool".to_string(),
        ..a.clone()
    };
    assert_ne!(a, b);
}

#[test]
fn inequality_by_version() {
    let a = make_tool_entry();
    let b = ToolEntry {
        version: "2.0.0".to_string(),
        ..a.clone()
    };
    assert_ne!(a, b);
}

#[test]
fn inequality_by_endpoint() {
    let a = make_tool_entry();
    let b = ToolEntry {
        endpoint: "http://localhost:9090".to_string(),
        ..a.clone()
    };
    assert_ne!(a, b);
}

#[test]
fn clone_is_equal() {
    let original = make_tool_entry();
    let cloned = original.clone();
    assert_eq!(original, cloned);
}

// ── RegistryError tests ─────────────────────────────────────────────

#[test]
fn tool_not_found_display() {
    let err = RegistryError::ToolNotFound {
        name: "missing-tool".to_string(),
    };
    assert_eq!(err.to_string(), "tool not found: 'missing-tool'");
}

#[test]
fn duplicate_entry_display() {
    let err = RegistryError::DuplicateEntry {
        name: "dup-tool".to_string(),
    };
    assert_eq!(err.to_string(), "duplicate tool entry: 'dup-tool'");
}

#[test]
fn connection_failed_display() {
    let err = RegistryError::ConnectionFailed {
        endpoint: "http://bad-host:1234".to_string(),
        reason: "timeout".to_string(),
    };
    assert_eq!(
        err.to_string(),
        "connection to 'http://bad-host:1234' failed: timeout"
    );
}

#[test]
fn error_trait_impl() {
    let err = RegistryError::ToolNotFound {
        name: "some-tool".to_string(),
    };
    let dyn_err: &dyn std::error::Error = &err;
    let msg = dyn_err.to_string();
    assert!(msg.contains("some-tool"), "expected tool name in: {msg}");
}

#[test]
fn registry_error_equality() {
    let a = RegistryError::ToolNotFound {
        name: "x".to_string(),
    };
    let b = RegistryError::ToolNotFound {
        name: "x".to_string(),
    };
    assert_eq!(a, b);
}

#[test]
fn inequality_between_variants() {
    let a = RegistryError::ToolNotFound {
        name: "x".to_string(),
    };
    let b = RegistryError::DuplicateEntry {
        name: "x".to_string(),
    };
    assert_ne!(a, b);
}
