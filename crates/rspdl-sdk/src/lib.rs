//! Versioned JSON facade shared by RSPDL language bindings.

#![forbid(unsafe_code)]

use std::time::Duration;

use rspdl_compiler::{
    CheckOptions, MAX_MODEL_SCOPE_PER_MODEL, ModelFindingOptions, Source, check_ko_files,
    compile_ko_files, find_ko_model,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;

pub const WIRE_SCHEMA_VERSION: u32 = 1;
pub const SUPPORTED_LOCALE: &str = "ko-KR";
const DEFAULT_SOLVER_TIMEOUT_MS: u64 = 5_000;
const DEFAULT_MODEL_SCOPE_PER_MODEL: usize = 3;

#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
pub enum SdkError {
    #[error("invalid request JSON: {reason}")]
    InvalidRequestJson { reason: String },
    #[error("unsupported wire schema version `{requested}`; supported version is `{supported}`")]
    UnsupportedSchemaVersion { requested: u32, supported: u32 },
    #[error("unsupported locale `{requested}`; supported locale is `{supported}`")]
    UnsupportedLocale {
        requested: String,
        supported: &'static str,
    },
    #[error("invalid SDK option `{option}`: {reason}")]
    InvalidOption {
        option: &'static str,
        reason: String,
    },
    #[error("failed to serialize SDK response: {reason}")]
    ResponseSerialization { reason: String },
}

impl SdkError {
    pub const fn code(&self) -> &'static str {
        match self {
            Self::InvalidRequestJson { .. } => "RSPDL-SDK-001",
            Self::UnsupportedSchemaVersion { .. } => "RSPDL-SDK-002",
            Self::UnsupportedLocale { .. } => "RSPDL-SDK-003",
            Self::InvalidOption { .. } => "RSPDL-SDK-004",
            Self::ResponseSerialization { .. } => "RSPDL-SDK-005",
        }
    }
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct SdkSource {
    path: String,
    text: String,
}

impl From<SdkSource> for Source {
    fn from(source: SdkSource) -> Self {
        Self::new(source.path, source.text)
    }
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct CompileRequest {
    schema_version: u32,
    locale: String,
    sources: Vec<SdkSource>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct CheckRequest {
    schema_version: u32,
    locale: String,
    sources: Vec<SdkSource>,
    data: Value,
    #[serde(default = "default_solver_timeout_ms")]
    timeout_ms: u64,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct FindModelRequest {
    schema_version: u32,
    locale: String,
    source: SdkSource,
    #[serde(default = "default_model_scope_per_model")]
    scope_per_model: usize,
    #[serde(default = "default_solver_timeout_ms")]
    timeout_ms: u64,
}

#[derive(Debug, Serialize)]
struct SdkResponse<T> {
    schema_version: u32,
    result: T,
}

const fn default_solver_timeout_ms() -> u64 {
    DEFAULT_SOLVER_TIMEOUT_MS
}

const fn default_model_scope_per_model() -> usize {
    DEFAULT_MODEL_SCOPE_PER_MODEL
}

pub fn compile_json(request_json: &str) -> Result<String, SdkError> {
    let request = parse_request::<CompileRequest>(request_json)?;
    validate_common(request.schema_version, &request.locale, &request.sources)?;
    serialize_response(compile_ko_files(
        request.sources.into_iter().map(Source::from).collect(),
    ))
}

pub fn check_json(request_json: &str) -> Result<String, SdkError> {
    let request = parse_request::<CheckRequest>(request_json)?;
    validate_common(request.schema_version, &request.locale, &request.sources)?;
    let solver_timeout = validate_timeout(request.timeout_ms)?;
    let runtime_json =
        serde_json::to_string(&request.data).map_err(|error| SdkError::InvalidRequestJson {
            reason: error.to_string(),
        })?;
    serialize_response(check_ko_files(
        request.sources.into_iter().map(Source::from).collect(),
        &runtime_json,
        CheckOptions { solver_timeout },
    ))
}

pub fn find_model_json(request_json: &str) -> Result<String, SdkError> {
    let request = parse_request::<FindModelRequest>(request_json)?;
    validate_schema_and_locale(request.schema_version, &request.locale)?;
    if request.source.path.trim().is_empty() {
        return Err(SdkError::InvalidOption {
            option: "source.path",
            reason: "must not be empty".into(),
        });
    }
    if !(1..=MAX_MODEL_SCOPE_PER_MODEL).contains(&request.scope_per_model) {
        return Err(SdkError::InvalidOption {
            option: "scope_per_model",
            reason: format!("must be between 1 and {MAX_MODEL_SCOPE_PER_MODEL}"),
        });
    }
    let solver_timeout = validate_timeout(request.timeout_ms)?;
    serialize_response(find_ko_model(
        &request.source.text,
        ModelFindingOptions {
            scope_per_model: request.scope_per_model,
            solver_timeout,
        },
    ))
}

fn parse_request<T: for<'de> Deserialize<'de>>(request_json: &str) -> Result<T, SdkError> {
    serde_json::from_str(request_json).map_err(|error| SdkError::InvalidRequestJson {
        reason: error.to_string(),
    })
}

fn validate_common(
    schema_version: u32,
    locale: &str,
    sources: &[SdkSource],
) -> Result<(), SdkError> {
    validate_schema_and_locale(schema_version, locale)?;
    if sources.is_empty() {
        return Err(SdkError::InvalidOption {
            option: "sources",
            reason: "must contain at least one source".into(),
        });
    }
    if sources.iter().any(|source| source.path.trim().is_empty()) {
        return Err(SdkError::InvalidOption {
            option: "sources[].path",
            reason: "must not be empty".into(),
        });
    }
    Ok(())
}

fn validate_schema_and_locale(schema_version: u32, locale: &str) -> Result<(), SdkError> {
    if schema_version != WIRE_SCHEMA_VERSION {
        return Err(SdkError::UnsupportedSchemaVersion {
            requested: schema_version,
            supported: WIRE_SCHEMA_VERSION,
        });
    }
    if locale != SUPPORTED_LOCALE {
        return Err(SdkError::UnsupportedLocale {
            requested: locale.into(),
            supported: SUPPORTED_LOCALE,
        });
    }
    Ok(())
}

fn validate_timeout(timeout_ms: u64) -> Result<Duration, SdkError> {
    if timeout_ms == 0 {
        return Err(SdkError::InvalidOption {
            option: "timeout_ms",
            reason: "must be greater than zero".into(),
        });
    }
    Ok(Duration::from_millis(timeout_ms))
}

fn serialize_response<T: Serialize>(result: T) -> Result<String, SdkError> {
    serde_json::to_string(&SdkResponse {
        schema_version: WIRE_SCHEMA_VERSION,
        result,
    })
    .map_err(|error| SdkError::ResponseSerialization {
        reason: error.to_string(),
    })
}

#[cfg(test)]
mod tests {
    use serde_json::{Value, json};

    use super::*;

    const VALID_SOURCE: &str = r#"@모듈 재고(inventory)

재고 항목(item)은 다음 필드들로 구성되어 있다.
    이름(name): 필수 문자열
"#;

    fn source(path: &str, text: &str) -> Value {
        json!({ "path": path, "text": text })
    }

    #[test]
    fn compile_preserves_compiler_diagnostics_in_a_successful_response() {
        let request = json!({
            "schema_version": WIRE_SCHEMA_VERSION,
            "locale": SUPPORTED_LOCALE,
            "sources": [source("invalid.rspdl", "not rspdl")],
        });
        let response = compile_json(&request.to_string()).unwrap();
        let response: Value = serde_json::from_str(&response).unwrap();

        assert_eq!(response["schema_version"], WIRE_SCHEMA_VERSION);
        assert!(response["result"]["files"][0]["module"].is_null());
        assert!(response["result"]["files"][0]["diagnostics"].is_array());
    }

    #[test]
    fn schema_one_compile_response_includes_semantic_ir_spans() {
        let request = json!({
            "schema_version": WIRE_SCHEMA_VERSION,
            "locale": SUPPORTED_LOCALE,
            "sources": [source("inventory.rspdl", VALID_SOURCE)],
        });
        let response = compile_json(&request.to_string()).unwrap();
        let response: Value = serde_json::from_str(&response).unwrap();
        let module_end = VALID_SOURCE.find('\n').unwrap();

        assert_eq!(response["schema_version"], 1);
        assert_eq!(
            response["result"]["files"][0]["module"]["span"],
            json!({ "start": 0, "end": module_end })
        );
        let model_span = &response["result"]["files"][0]["module"]["models"][0]["span"];
        assert!(model_span["start"].as_u64().unwrap() < model_span["end"].as_u64().unwrap());
    }

    #[test]
    fn compile_is_independent_of_source_input_order() {
        let first = json!({
            "schema_version": WIRE_SCHEMA_VERSION,
            "locale": SUPPORTED_LOCALE,
            "sources": [
                source("b.rspdl", &VALID_SOURCE.replace("inventory", "second")),
                source("a.rspdl", VALID_SOURCE),
            ],
        });
        let second = json!({
            "schema_version": WIRE_SCHEMA_VERSION,
            "locale": SUPPORTED_LOCALE,
            "sources": [
                source("a.rspdl", VALID_SOURCE),
                source("b.rspdl", &VALID_SOURCE.replace("inventory", "second")),
            ],
        });

        assert_eq!(
            compile_json(&first.to_string()).unwrap(),
            compile_json(&second.to_string()).unwrap()
        );
    }

    #[test]
    fn check_returns_runtime_diagnostics_in_the_report() {
        let request = json!({
            "schema_version": WIRE_SCHEMA_VERSION,
            "locale": SUPPORTED_LOCALE,
            "sources": [source("inventory.rspdl", VALID_SOURCE)],
            "data": { "unexpected": true },
        });
        let response: Value =
            serde_json::from_str(&check_json(&request.to_string()).unwrap()).unwrap();

        assert_eq!(
            response["result"]["runtime_diagnostics"][0]["rule_id"],
            "RSPDL-INPUT-001"
        );
    }

    #[test]
    fn model_finding_preserves_a_non_success_result() {
        let request = json!({
            "schema_version": WIRE_SCHEMA_VERSION,
            "locale": SUPPORTED_LOCALE,
            "source": source("inventory.rspdl", VALID_SOURCE),
            "scope_per_model": 1,
        });
        let response: Value =
            serde_json::from_str(&find_model_json(&request.to_string()).unwrap()).unwrap();

        assert_eq!(response["schema_version"], WIRE_SCHEMA_VERSION);
        assert!(response["result"].get("result").is_some());
    }

    #[test]
    fn rejects_binding_configuration_without_turning_it_into_a_compiler_diagnostic() {
        let invalid_schema = json!({
            "schema_version": 999,
            "locale": SUPPORTED_LOCALE,
            "sources": [source("inventory.rspdl", VALID_SOURCE)],
        });
        let unsupported_locale = json!({
            "schema_version": WIRE_SCHEMA_VERSION,
            "locale": "en-US",
            "sources": [source("inventory.rspdl", VALID_SOURCE)],
        });
        let empty_sources = json!({
            "schema_version": WIRE_SCHEMA_VERSION,
            "locale": SUPPORTED_LOCALE,
            "sources": [],
        });

        assert_eq!(
            compile_json(&invalid_schema.to_string())
                .unwrap_err()
                .code(),
            "RSPDL-SDK-002"
        );
        assert_eq!(
            compile_json(&unsupported_locale.to_string())
                .unwrap_err()
                .code(),
            "RSPDL-SDK-003"
        );
        assert_eq!(
            compile_json(&empty_sources.to_string()).unwrap_err().code(),
            "RSPDL-SDK-004"
        );
    }

    #[test]
    fn rejects_malformed_json_and_invalid_solver_options() {
        assert_eq!(compile_json("{").unwrap_err().code(), "RSPDL-SDK-001");

        let request = json!({
            "schema_version": WIRE_SCHEMA_VERSION,
            "locale": SUPPORTED_LOCALE,
            "source": source("inventory.rspdl", VALID_SOURCE),
            "scope_per_model": 0,
        });
        assert_eq!(
            find_model_json(&request.to_string()).unwrap_err().code(),
            "RSPDL-SDK-004"
        );
    }
}
