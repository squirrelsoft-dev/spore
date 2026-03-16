use agent_sdk::{SkillManifest, ALLOWED_OUTPUT_FORMATS};

use crate::SkillError;

/// Trait for checking whether a tool name is registered.
///
/// Used by the `validate` function to verify that all tool names
/// referenced in a `SkillManifest` actually exist in the runtime.
pub trait ToolExists {
    fn tool_exists(&self, name: &str) -> bool;
}

/// Stub implementation that always returns `true`.
///
/// Useful in tests that exercise validation rules unrelated to tool
/// name checking (e.g., confidence threshold, output format).
#[derive(Debug, Clone, Copy)]
pub struct AllToolsExist;

impl ToolExists for AllToolsExist {
    fn tool_exists(&self, _name: &str) -> bool {
        true
    }
}

pub fn validate(manifest: &SkillManifest, tool_checker: &dyn ToolExists) -> Result<(), SkillError> {
    let mut reasons: Vec<String> = Vec::new();

    check_required_strings(manifest, &mut reasons);
    check_preamble(&manifest.preamble, &mut reasons);
    check_confidence_threshold(manifest.constraints.confidence_threshold, &mut reasons);
    check_max_turns(manifest.constraints.max_turns, &mut reasons);
    check_tools_exist(&manifest.tools, tool_checker, &mut reasons);
    check_output_format(&manifest.output.format, &mut reasons);
    check_escalate_to(&manifest.constraints.escalate_to, &mut reasons);

    if reasons.is_empty() {
        Ok(())
    } else {
        Err(SkillError::ValidationError {
            skill_name: manifest.name.clone(),
            reasons,
        })
    }
}

fn check_required_strings(manifest: &SkillManifest, reasons: &mut Vec<String>) {
    let fields: [(&str, &str); 4] = [
        ("name", &manifest.name),
        ("version", &manifest.version),
        ("model.provider", &manifest.model.provider),
        ("model.name", &manifest.model.name),
    ];

    for (label, value) in &fields {
        if value.trim().is_empty() {
            reasons.push(format!("'{label}' must not be empty"));
        }
    }
}

fn check_preamble(preamble: &str, reasons: &mut Vec<String>) {
    if preamble.trim().is_empty() {
        reasons.push("'preamble' must not be empty".to_string());
    }
}

fn check_confidence_threshold(value: f64, reasons: &mut Vec<String>) {
    if !(0.0..=1.0).contains(&value) {
        reasons.push(format!(
            "'confidence_threshold' must be between 0.0 and 1.0, got {value}"
        ));
    }
}

fn check_max_turns(value: u32, reasons: &mut Vec<String>) {
    if value == 0 {
        reasons.push("'max_turns' must be greater than 0".to_string());
    }
}

fn check_tools_exist(tools: &[String], tool_checker: &dyn ToolExists, reasons: &mut Vec<String>) {
    let missing: Vec<&str> = tools
        .iter()
        .filter(|name| !tool_checker.tool_exists(name))
        .map(|s| s.as_str())
        .collect();

    if !missing.is_empty() {
        reasons.push(format!("tools not found: {}", missing.join(", ")));
    }
}

fn check_output_format(format: &str, reasons: &mut Vec<String>) {
    if !ALLOWED_OUTPUT_FORMATS.contains(&format) {
        reasons.push(format!("unrecognized output format '{format}'"));
    }
}

fn check_escalate_to(escalate_to: &Option<String>, reasons: &mut Vec<String>) {
    // TODO: full cross-agent escalation validation deferred to orchestrator
    if let Some(name) = escalate_to
        && name.trim().is_empty()
    {
        reasons.push("'escalate_to' must not be empty when provided".to_string());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_tools_exist_returns_true() {
        let checker = AllToolsExist;
        assert!(checker.tool_exists("any_tool_name"));
    }

    #[test]
    fn all_tools_exist_returns_true_for_empty_string() {
        let checker = AllToolsExist;
        assert!(checker.tool_exists(""));
    }

    #[test]
    fn trait_is_object_safe() {
        let checker = AllToolsExist;
        let dyn_checker: &dyn ToolExists = &checker;
        assert!(dyn_checker.tool_exists("some_tool"));
    }
}
