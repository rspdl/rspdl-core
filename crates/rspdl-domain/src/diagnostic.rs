use std::cmp::Ordering;
use std::collections::BTreeMap;

use serde::Serialize;

use crate::TextRange;

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Severity {
    Error,
    Warning,
}

/// A locale-neutral diagnostic envelope shared by frontends and semantic phases.
///
/// Human-readable sentences are rendered at a locale boundary. The compiler
/// contract contains only a stable message key and deterministic arguments.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct Diagnostic {
    pub rule_id: String,
    pub severity: Severity,
    pub message_key: String,
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    pub arguments: BTreeMap<String, String>,
    pub span: TextRange,
}

impl Diagnostic {
    pub fn new(
        rule_id: impl Into<String>,
        severity: Severity,
        message_key: impl Into<String>,
        span: TextRange,
    ) -> Self {
        Self {
            rule_id: rule_id.into(),
            severity,
            message_key: message_key.into(),
            arguments: BTreeMap::new(),
            span,
        }
    }

    pub fn error(
        rule_id: impl Into<String>,
        message_key: impl Into<String>,
        span: TextRange,
    ) -> Self {
        Self::new(rule_id, Severity::Error, message_key, span)
    }

    pub fn warning(
        rule_id: impl Into<String>,
        message_key: impl Into<String>,
        span: TextRange,
    ) -> Self {
        Self::new(rule_id, Severity::Warning, message_key, span)
    }

    pub fn with_argument(mut self, key: impl Into<String>, value: impl ToString) -> Self {
        self.arguments.insert(key.into(), value.to_string());
        self
    }

    pub fn argument(&self, key: &str) -> Option<&str> {
        self.arguments.get(key).map(String::as_str)
    }

    pub fn is_error(&self) -> bool {
        self.severity == Severity::Error
    }

    /// Canonical ordering shared by every compiler phase.
    pub fn stable_cmp(left: &Self, right: &Self) -> Ordering {
        (
            left.span.start,
            left.span.end,
            &left.rule_id,
            &left.message_key,
            &left.arguments,
        )
            .cmp(&(
                right.span.start,
                right.span.end,
                &right.rule_id,
                &right.message_key,
                &right.arguments,
            ))
    }
}
