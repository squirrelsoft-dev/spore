use agent_sdk::{AgentError, AgentRequest, AgentResponse, HealthStatus, ToolCallRecord};
use serde_json::json;
use uuid::Uuid;

#[test]
fn agent_request_new_sets_uuid_and_defaults() {
    let request = AgentRequest::new("hello".to_string());

    assert_ne!(request.id, Uuid::nil());
    assert_eq!(request.input, "hello");
    assert!(request.context.is_none());
    assert!(request.caller.is_none());
}

#[test]
fn agent_request_json_round_trip() {
    let original = AgentRequest {
        id: Uuid::nil(),
        input: "analyze this data".to_string(),
        context: Some(json!({"key": "value"})),
        caller: Some("orchestrator".to_string()),
    };

    let json_str = serde_json::to_string(&original).unwrap();
    let deserialized: AgentRequest = serde_json::from_str(&json_str).unwrap();

    assert_eq!(original, deserialized);
}

#[test]
fn agent_response_success_sets_defaults() {
    let id = Uuid::new_v4();
    let response = AgentResponse::success(id, json!("result"));

    assert!((response.confidence - 1.0).abs() < f32::EPSILON);
    assert!(!response.escalated);
    assert!(response.tool_calls.is_empty());
    assert_eq!(response.output, json!("result"));
}

#[test]
fn agent_response_json_round_trip_with_tool_calls() {
    let original = AgentResponse {
        id: Uuid::nil(),
        output: json!({"answer": 42}),
        confidence: 0.875,
        escalated: true,
        escalate_to: None,
        tool_calls: vec![ToolCallRecord {
            tool_name: "search".to_string(),
            input: json!({"q": "test"}),
            output: json!({"results": []}),
        }],
    };

    let json_str = serde_json::to_string(&original).unwrap();
    let deserialized: AgentResponse = serde_json::from_str(&json_str).unwrap();

    assert_eq!(original, deserialized);
}

#[test]
fn agent_error_display_contains_expected_substrings() {
    let tool_call_failed = AgentError::ToolCallFailed {
        tool: "web_search".into(),
        reason: "timeout".into(),
    };
    let display = format!("{}", tool_call_failed);
    assert!(
        display.contains("web_search"),
        "expected 'web_search' in: {display}"
    );
    assert!(
        display.contains("timeout"),
        "expected 'timeout' in: {display}"
    );

    let confidence_low = AgentError::ConfidenceTooLow {
        confidence: 0.3,
        threshold: 0.75,
    };
    let display = format!("{}", confidence_low);
    assert!(display.contains("0.3"), "expected '0.3' in: {display}");
    assert!(display.contains("0.75"), "expected '0.75' in: {display}");

    let max_turns = AgentError::MaxTurnsExceeded { turns: 10 };
    let display = format!("{}", max_turns);
    assert!(display.contains("10"), "expected '10' in: {display}");

    let internal = AgentError::Internal("something broke".into());
    let display = format!("{}", internal);
    assert!(
        display.contains("something broke"),
        "expected 'something broke' in: {display}"
    );
}

#[test]
fn health_status_serialize_deserialize_each_variant() {
    let variants = vec![
        HealthStatus::Healthy,
        HealthStatus::Degraded("slow db".into()),
        HealthStatus::Unhealthy("disk full".into()),
    ];

    for original in variants {
        let json_str = serde_json::to_string(&original).unwrap();
        let deserialized: HealthStatus = serde_json::from_str(&json_str).unwrap();
        assert_eq!(original, deserialized);
    }
}

#[test]
fn tool_call_record_json_round_trip_with_nested_values() {
    let original = ToolCallRecord {
        tool_name: "execute_sql".to_string(),
        input: json!({
            "query": "SELECT * FROM orders",
            "params": [1, 2, 3]
        }),
        output: json!({
            "rows": [{"id": 1, "name": "Widget"}],
            "count": 1
        }),
    };

    let json_str = serde_json::to_string(&original).unwrap();
    let deserialized: ToolCallRecord = serde_json::from_str(&json_str).unwrap();

    assert_eq!(original, deserialized);
}

#[test]
fn agent_request_none_fields_round_trip() {
    let original = AgentRequest {
        id: Uuid::nil(),
        input: "test input".to_string(),
        context: None,
        caller: None,
    };

    let json_str = serde_json::to_string(&original).unwrap();
    let deserialized: AgentRequest = serde_json::from_str(&json_str).unwrap();

    assert_eq!(original, deserialized);
}

#[test]
fn agent_response_empty_tool_calls_round_trip() {
    let original = AgentResponse {
        id: Uuid::nil(),
        output: json!("done"),
        confidence: 0.75,
        escalated: true,
        escalate_to: None,
        tool_calls: vec![],
    };

    let json_str = serde_json::to_string(&original).unwrap();
    let deserialized: AgentResponse = serde_json::from_str(&json_str).unwrap();

    assert_eq!(original, deserialized);
}

#[test]
fn agent_error_equality() {
    assert_eq!(
        AgentError::MaxTurnsExceeded { turns: 5 },
        AgentError::MaxTurnsExceeded { turns: 5 }
    );
    assert_ne!(
        AgentError::Internal("a".to_string()),
        AgentError::Internal("b".to_string())
    );
    assert_ne!(
        AgentError::Internal("x".to_string()),
        AgentError::MaxTurnsExceeded { turns: 1 }
    );
}
