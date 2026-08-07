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
/// Message keys and structured evidence will extend this envelope in a later
/// vertical slice. Keeping the type in the domain crate already prevents
/// semantic phases from depending on a locale frontend.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct Diagnostic {
    pub rule_id: String,
    pub severity: Severity,
    pub message: String,
    pub span: TextRange,
}

impl Diagnostic {
    pub fn new(
        rule_id: impl Into<String>,
        severity: Severity,
        message: impl Into<String>,
        span: TextRange,
    ) -> Self {
        Self {
            rule_id: rule_id.into(),
            severity,
            message: message.into(),
            span,
        }
    }

    pub fn error(rule_id: impl Into<String>, message: impl Into<String>, span: TextRange) -> Self {
        Self::new(rule_id, Severity::Error, message, span)
    }

    pub fn warning(
        rule_id: impl Into<String>,
        message: impl Into<String>,
        span: TextRange,
    ) -> Self {
        Self::new(rule_id, Severity::Warning, message, span)
    }

    pub fn is_error(&self) -> bool {
        self.severity == Severity::Error
    }
}
