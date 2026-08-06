//! Deterministic controlled-Korean scanner, parser, formatter, and lowering.

#![forbid(unsafe_code)]

mod ast;
mod formatter;
mod lowering;
mod parser;
mod scanner;

pub use ast::{
    ActionAst, ConstraintAst, ConstraintExpressionAst, DataModelAst, DeclarationAst, DocumentAst,
    EnumAst, EnumValueAst, FieldAst, FieldIntentAst, FieldIntentKindAst, LiteralAst, ModuleAst,
    NamedIdAst, OperandAst, PolicyAst, PolicyEffectAst, RecalculationAst, RelationOperatorAst,
    RoleAst, ScreenAst, ScreenOperationKindAst, Span, SumDerivationAst, TypeReferenceAst,
};
pub use formatter::{FormatError, format_document};
pub use lowering::{LowerOutput, lower};
pub use parser::{ParseOutput, parse};
pub use scanner::{ScanOutput, Token, TokenKind, scan};

use serde::Serialize;

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Severity {
    Error,
    Warning,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct Diagnostic {
    pub rule_id: String,
    pub severity: Severity,
    pub message: String,
    pub span: Span,
}

impl Diagnostic {
    fn error(rule_id: &str, message: impl Into<String>, span: Span) -> Self {
        Self {
            rule_id: rule_id.into(),
            severity: Severity::Error,
            message: message.into(),
            span,
        }
    }

    pub fn is_error(&self) -> bool {
        self.severity == Severity::Error
    }
}
