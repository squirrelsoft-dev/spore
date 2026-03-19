use agent_sdk::AgentError;
use orchestrator::error::OrchestratorError;

// ---------------------------------------------------------------------------
// Display tests -- one per variant
// ---------------------------------------------------------------------------

#[test]
fn display_no_route_contains_input() {
    let err = OrchestratorError::NoRoute {
        input: "hello".into(),
    };
    let display = format!("{}", err);
    assert!(
        display.contains("No route found for input: hello"),
        "expected 'No route found for input: hello' in: {display}"
    );
}

#[test]
fn display_agent_unavailable_contains_name_and_reason() {
    let err = OrchestratorError::AgentUnavailable {
        name: "agent-1".into(),
        reason: "connection refused".into(),
    };
    let display = format!("{}", err);
    assert!(
        display.contains("Agent 'agent-1' unavailable: connection refused"),
        "expected \"Agent 'agent-1' unavailable: connection refused\" in: {display}"
    );
}

#[test]
fn display_escalation_failed_contains_chain_and_reason() {
    let err = OrchestratorError::EscalationFailed {
        chain: vec!["a".into(), "b".into()],
        reason: "max depth".into(),
    };
    let display = format!("{}", err);
    assert!(
        display.contains("Escalation failed through chain [a -> b]: max depth"),
        "expected 'Escalation failed through chain [a -> b]: max depth' in: {display}"
    );
}

#[test]
fn display_http_error_contains_url_and_reason() {
    let err = OrchestratorError::HttpError {
        url: "http://localhost/invoke".into(),
        reason: "timeout".into(),
    };
    let display = format!("{}", err);
    assert!(
        display.contains("HTTP error calling http://localhost/invoke: timeout"),
        "expected 'HTTP error calling http://localhost/invoke: timeout' in: {display}"
    );
}

// ---------------------------------------------------------------------------
// From conversion tests -- OrchestratorError -> AgentError
// ---------------------------------------------------------------------------

#[test]
fn from_no_route_to_agent_error_internal() {
    let err = OrchestratorError::NoRoute {
        input: "hello".into(),
    };
    let agent_err: AgentError = AgentError::from(err);
    assert!(
        matches!(&agent_err, AgentError::Internal(msg) if msg.contains("No route found for input: hello")),
        "expected AgentError::Internal containing 'No route found for input: hello', got: {agent_err:?}"
    );
}

#[test]
fn from_agent_unavailable_to_agent_error_internal() {
    let err = OrchestratorError::AgentUnavailable {
        name: "agent-1".into(),
        reason: "connection refused".into(),
    };
    let agent_err: AgentError = err.into();
    assert!(
        matches!(&agent_err, AgentError::Internal(msg) if msg.contains("Agent 'agent-1' unavailable: connection refused")),
        "expected AgentError::Internal containing agent unavailable message, got: {agent_err:?}"
    );
}

#[test]
fn from_escalation_failed_to_agent_error_internal() {
    let err = OrchestratorError::EscalationFailed {
        chain: vec!["a".into(), "b".into()],
        reason: "max depth".into(),
    };
    let agent_err: AgentError = AgentError::from(err);
    assert!(
        matches!(&agent_err, AgentError::Internal(msg) if msg.contains("Escalation failed through chain [a -> b]: max depth")),
        "expected AgentError::Internal containing escalation failed message, got: {agent_err:?}"
    );
}

#[test]
fn from_http_error_to_agent_error_internal() {
    let err = OrchestratorError::HttpError {
        url: "http://localhost/invoke".into(),
        reason: "timeout".into(),
    };
    let agent_err: AgentError = err.into();
    assert!(
        matches!(&agent_err, AgentError::Internal(msg) if msg.contains("HTTP error calling http://localhost/invoke: timeout")),
        "expected AgentError::Internal containing http error message, got: {agent_err:?}"
    );
}

// ---------------------------------------------------------------------------
// EscalationFailed edge cases
// ---------------------------------------------------------------------------

#[test]
fn display_escalation_failed_empty_chain() {
    let err = OrchestratorError::EscalationFailed {
        chain: vec![],
        reason: "no agents".into(),
    };
    let display = format!("{}", err);
    assert!(display.contains("[]"), "expected '[]' in: {display}");
    assert!(
        display.contains("no agents"),
        "expected 'no agents' in: {display}"
    );
}

#[test]
fn display_escalation_failed_single_chain() {
    let err = OrchestratorError::EscalationFailed {
        chain: vec!["a".into()],
        reason: "fail".into(),
    };
    let display = format!("{}", err);
    assert!(display.contains("[a]"), "expected '[a]' in: {display}");
    assert!(display.contains("fail"), "expected 'fail' in: {display}");
}

// ---------------------------------------------------------------------------
// Error trait verification
// ---------------------------------------------------------------------------

#[test]
fn orchestrator_error_implements_std_error_trait() {
    let err = OrchestratorError::NoRoute {
        input: "test".into(),
    };
    let std_err: &dyn std::error::Error = &err;
    assert!(
        std_err.source().is_none(),
        "expected source() to return None"
    );
}
