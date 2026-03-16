use std::path::PathBuf;

use serde::Deserialize;

use crate::error::SkillError;
use agent_sdk::{Constraints, ModelConfig, OutputSchema};

const BOM: char = '\u{FEFF}';
const DELIMITER: &str = "---";

#[derive(Deserialize)]
pub(crate) struct SkillFrontmatter {
    pub(crate) name: String,
    pub(crate) version: String,
    pub(crate) description: String,
    pub(crate) model: ModelConfig,
    pub(crate) tools: Vec<String>,
    pub(crate) constraints: Constraints,
    pub(crate) output: OutputSchema,
}

pub(crate) fn extract_frontmatter(content: &str) -> Result<(&str, &str), SkillError> {
    let trimmed = content.trim_start_matches(BOM).trim_start();

    if !trimmed.starts_with(DELIMITER) {
        return Err(missing_opening_error());
    }

    let opening_end = find_opening_delimiter_end(trimmed);
    let yaml_start = &trimmed[opening_end..];
    let closing_offset = find_closing_delimiter_offset(yaml_start)?;

    let yaml = &yaml_start[..closing_offset];
    let body = extract_body(yaml_start, closing_offset);

    Ok((yaml, body))
}

fn missing_opening_error() -> SkillError {
    SkillError::ParseError {
        path: PathBuf::from("<unknown>"),
        source: "missing opening frontmatter delimiter '---'".to_string(),
    }
}

fn find_opening_delimiter_end(trimmed: &str) -> usize {
    match trimmed.find('\n') {
        Some(pos) => pos + 1,
        None => trimmed.len(),
    }
}

fn find_closing_delimiter_offset(content: &str) -> Result<usize, SkillError> {
    let mut pos = 0;
    while pos < content.len() {
        let line_end = content[pos..].find('\n');
        let line = match line_end {
            Some(end) => &content[pos..pos + end],
            None => &content[pos..],
        };

        if line.trim() == DELIMITER {
            return Ok(pos);
        }

        pos += match line_end {
            Some(end) => end + 1,
            None => content.len() - pos,
        };
    }

    Err(SkillError::ParseError {
        path: PathBuf::from("<unknown>"),
        source: "missing closing frontmatter delimiter '---'".to_string(),
    })
}

fn extract_body(yaml_start: &str, closing_offset: usize) -> &str {
    let after_closing = &yaml_start[closing_offset..];
    let body_start = match after_closing.find('\n') {
        Some(pos) => pos + 1,
        None => after_closing.len(),
    };
    yaml_start[closing_offset + body_start..].trim()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_standard_frontmatter() {
        let input = "---\nname: test\n---\nbody content";
        let (yaml, body) = extract_frontmatter(input).unwrap();
        assert_eq!(yaml, "name: test\n");
        assert_eq!(body, "body content");
    }

    #[test]
    fn handles_bom_prefix() {
        let input = "\u{FEFF}---\nname: test\n---\nbody";
        let (yaml, body) = extract_frontmatter(input).unwrap();
        assert_eq!(yaml, "name: test\n");
        assert_eq!(body, "body");
    }

    #[test]
    fn handles_empty_yaml() {
        let input = "---\n---\nbody";
        let (yaml, body) = extract_frontmatter(input).unwrap();
        assert_eq!(yaml, "");
        assert_eq!(body, "body");
    }

    #[test]
    fn handles_empty_body() {
        let input = "---\nname: test\n---";
        let (yaml, body) = extract_frontmatter(input).unwrap();
        assert_eq!(yaml, "name: test\n");
        assert_eq!(body, "");
    }

    #[test]
    fn handles_no_trailing_newline_after_closing() {
        let input = "---\nkey: value\n---";
        let (yaml, body) = extract_frontmatter(input).unwrap();
        assert_eq!(yaml, "key: value\n");
        assert_eq!(body, "");
    }

    #[test]
    fn handles_windows_line_endings() {
        let input = "---\r\nname: test\r\n---\r\nbody";
        let (yaml, body) = extract_frontmatter(input).unwrap();
        assert_eq!(yaml, "name: test\r\n");
        assert_eq!(body, "body");
    }

    #[test]
    fn body_containing_horizontal_rules() {
        let input = "---\nname: test\n---\nsome text\n---\nmore text";
        let (yaml, body) = extract_frontmatter(input).unwrap();
        assert_eq!(yaml, "name: test\n");
        assert_eq!(body, "some text\n---\nmore text");
    }

    #[test]
    fn returns_error_for_missing_opening_delimiter() {
        let result = extract_frontmatter("no frontmatter here");
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert_eq!(
            err.to_string(),
            "parse error in <unknown>: missing opening frontmatter delimiter '---'"
        );
    }

    #[test]
    fn returns_error_for_missing_closing_delimiter() {
        let result = extract_frontmatter("---\nname: test\nno closing");
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert_eq!(
            err.to_string(),
            "parse error in <unknown>: missing closing frontmatter delimiter '---'"
        );
    }

    #[test]
    fn handles_leading_whitespace() {
        let input = "  \n  ---\nname: test\n---\nbody";
        let (yaml, body) = extract_frontmatter(input).unwrap();
        assert_eq!(yaml, "name: test\n");
        assert_eq!(body, "body");
    }

    #[test]
    fn handles_multiline_yaml() {
        let input = "---\nname: test\ndescription: a skill\ntags:\n  - one\n  - two\n---\nbody";
        let (yaml, body) = extract_frontmatter(input).unwrap();
        assert_eq!(
            yaml,
            "name: test\ndescription: a skill\ntags:\n  - one\n  - two\n"
        );
        assert_eq!(body, "body");
    }

    #[test]
    fn extract_valid_frontmatter_with_body() {
        let input = "---\nname: test\nversion: \"1.0\"\n---\n# Hello World\n\nThis is the body.";
        let (yaml, body) = extract_frontmatter(input).unwrap();

        assert!(yaml.contains("name: test"));
        assert!(yaml.contains("version: \"1.0\""));
        assert!(!yaml.contains("---"), "YAML should not include delimiters");

        assert!(body.contains("# Hello World"));
        assert!(body.contains("This is the body."));
        assert_eq!(body, body.trim(), "body should be trimmed");
    }

    #[test]
    fn extract_valid_frontmatter_with_empty_body() {
        let input = "---\nname: minimal\nversion: \"1.0\"\n---\n  \n";
        let (yaml, body) = extract_frontmatter(input).unwrap();

        assert!(!yaml.is_empty());
        assert!(yaml.contains("name: minimal"));
        assert!(yaml.contains("version: \"1.0\""));
        assert_eq!(body, "");
    }

    #[test]
    fn extract_missing_opening_delimiter() {
        let input = "name: test\nversion: \"1.0\"\n---\n# Body text";
        let result = extract_frontmatter(input);

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(matches!(err, SkillError::ParseError { .. }));
        if let SkillError::ParseError { source, .. } = &err {
            assert!(
                source.contains("delimiter") || source.contains("---"),
                "expected mention of delimiter in: {source}"
            );
        }
    }

    #[test]
    fn extract_missing_closing_delimiter() {
        let input = "---\nname: test\nversion: \"1.0\"\n# This never closes";
        let result = extract_frontmatter(input);

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(matches!(err, SkillError::ParseError { .. }));
    }

    #[test]
    fn extract_body_with_horizontal_rules() {
        let input = "---\nname: test\nversion: \"1.0\"\n---\n# Section One\n\nSome text.\n\n---\n\n# Section Two\n\nMore text.";
        let (yaml, body) = extract_frontmatter(input).unwrap();

        assert!(yaml.contains("name: test"));
        assert!(yaml.contains("version: \"1.0\""));
        assert!(
            !yaml.contains("Section"),
            "YAML should not contain body content"
        );

        assert!(
            body.contains("---"),
            "body should contain the horizontal rule"
        );
        assert!(body.contains("# Section One"));
        assert!(body.contains("# Section Two"));
        assert!(body.contains("Some text."));
        assert!(body.contains("More text."));
    }

    #[test]
    fn extract_frontmatter_with_leading_whitespace() {
        let input = "   ---\nname: test\nversion: \"1.0\"\n---\nBody text here.";
        let (yaml, body) = extract_frontmatter(input).unwrap();

        assert!(yaml.contains("name: test"));
        assert!(yaml.contains("version: \"1.0\""));
        assert_eq!(body, "Body text here.");
    }
}
