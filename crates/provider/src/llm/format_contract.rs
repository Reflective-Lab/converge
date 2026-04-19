// Copyright 2024-2026 Reflective Labs
// SPDX-License-Identifier: MIT

use converge_provider_api::{ChatResponse, LlmError, ResponseFormat};

pub(super) fn finalize_chat_response(
    requested_format: ResponseFormat,
    mut response: ChatResponse,
) -> Result<ChatResponse, LlmError> {
    if response.tool_calls.is_empty() {
        response.content = normalize_content(&response.content, requested_format)?;
    }
    Ok(response)
}

fn normalize_content(content: &str, requested_format: ResponseFormat) -> Result<String, LlmError> {
    match requested_format {
        ResponseFormat::Text | ResponseFormat::Markdown => Ok(content.to_string()),
        ResponseFormat::Json => validate_json(content),
        ResponseFormat::Yaml => validate_yaml(content),
        ResponseFormat::Toml => validate_toml(content),
    }
}

fn validate_json(content: &str) -> Result<String, LlmError> {
    let normalized = normalized_candidate(content);
    let value = serde_json::from_str::<serde_json::Value>(normalized).map_err(|error| {
        format_mismatch(
            ResponseFormat::Json,
            format!("expected JSON object or array: {error}"),
            normalized,
        )
    })?;

    if value.is_object() || value.is_array() {
        return Ok(normalized.to_string());
    }

    Err(format_mismatch(
        ResponseFormat::Json,
        "expected JSON object or array".to_string(),
        normalized,
    ))
}

fn validate_yaml(content: &str) -> Result<String, LlmError> {
    let normalized = normalized_candidate(content);
    let value = serde_yaml::from_str::<serde_yaml::Value>(normalized).map_err(|error| {
        format_mismatch(
            ResponseFormat::Yaml,
            format!("expected YAML mapping or sequence: {error}"),
            normalized,
        )
    })?;

    if matches!(
        value,
        serde_yaml::Value::Mapping(_) | serde_yaml::Value::Sequence(_)
    ) {
        return Ok(normalized.to_string());
    }

    Err(format_mismatch(
        ResponseFormat::Yaml,
        "expected YAML mapping or sequence".to_string(),
        normalized,
    ))
}

fn validate_toml(content: &str) -> Result<String, LlmError> {
    let normalized = normalized_candidate(content);
    let value = toml::from_str::<toml::Value>(normalized).map_err(|error| {
        format_mismatch(
            ResponseFormat::Toml,
            format!("expected TOML document: {error}"),
            normalized,
        )
    })?;

    if matches!(value, toml::Value::Table(_) | toml::Value::Array(_)) {
        return Ok(normalized.to_string());
    }

    Err(format_mismatch(
        ResponseFormat::Toml,
        "expected TOML table or array".to_string(),
        normalized,
    ))
}

fn normalized_candidate(content: &str) -> &str {
    strip_code_fences(content).trim()
}

fn strip_code_fences(content: &str) -> &str {
    let trimmed = content.trim();
    if let Some(rest) = trimmed.strip_prefix("```") {
        if let Some(after_tag) = rest.find('\n') {
            let inner = &rest[after_tag + 1..];
            if let Some(end) = inner.rfind("```") {
                return inner[..end].trim();
            }
        }
    }
    trimmed
}

fn format_mismatch(expected: ResponseFormat, detail: String, content: &str) -> LlmError {
    let preview = preview(content);
    LlmError::ResponseFormatMismatch {
        expected,
        message: if preview.is_empty() {
            detail
        } else {
            format!("{detail}; response preview: {preview}")
        },
    }
}

fn preview(content: &str) -> String {
    const MAX_PREVIEW_CHARS: usize = 120;

    let trimmed = content.trim();
    if trimmed.is_empty() {
        return String::new();
    }

    let mut preview = String::new();
    for ch in trimmed.chars().take(MAX_PREVIEW_CHARS) {
        preview.push(ch);
    }
    if trimmed.chars().count() > MAX_PREVIEW_CHARS {
        preview.push_str("...");
    }
    preview
}

#[cfg(test)]
mod tests {
    use converge_core::traits::{ChatResponse, LlmError, ResponseFormat, ToolCall};

    use super::finalize_chat_response;

    fn response(content: &str) -> ChatResponse {
        ChatResponse {
            content: content.to_string(),
            tool_calls: Vec::new(),
            usage: None,
            model: None,
            finish_reason: None,
            metadata: Default::default(),
        }
    }

    #[test]
    fn strips_json_code_fences() {
        let response = finalize_chat_response(
            ResponseFormat::Json,
            response("```json\n{\"facts\":[\"a\"]}\n```"),
        )
        .unwrap();

        assert_eq!(response.content, "{\"facts\":[\"a\"]}");
    }

    #[test]
    fn rejects_chatty_json_wrapper() {
        let error = finalize_chat_response(
            ResponseFormat::Json,
            response("Here is the JSON you asked for:\n{\"facts\":[\"a\"]}"),
        )
        .unwrap_err();

        assert!(matches!(
            error,
            LlmError::ResponseFormatMismatch {
                expected: ResponseFormat::Json,
                ..
            }
        ));
    }

    #[test]
    fn rejects_yaml_scalar() {
        let error =
            finalize_chat_response(ResponseFormat::Yaml, response("plain text reply")).unwrap_err();

        assert!(matches!(
            error,
            LlmError::ResponseFormatMismatch {
                expected: ResponseFormat::Yaml,
                ..
            }
        ));
    }

    #[test]
    fn skips_validation_for_tool_calls() {
        let mut response = response("not json");
        response.tool_calls = vec![ToolCall {
            id: "call-1".to_string(),
            name: "lookup".to_string(),
            arguments: "{}".to_string(),
        }];

        let finalized = finalize_chat_response(ResponseFormat::Json, response).unwrap();
        assert_eq!(finalized.content, "not json");
    }
}
