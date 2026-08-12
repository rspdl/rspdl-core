//! Deterministic controlled-Korean scanner, parser, formatter, and lowering.

#![forbid(unsafe_code)]

mod ast;
mod diagnostic;
mod formatter;
#[cfg(test)]
mod generated_parser;
mod lowering;
mod parser;
mod scanner;

pub use ast::{
    ActionAst, ConstraintAst, ConstraintExpressionAst, DataModelAst, DeclarationAst, DocumentAst,
    EnumAst, EnumValueAst, FieldAst, FieldIntentAst, FieldIntentKindAst, LiteralAst, ModuleAst,
    NamedIdAst, OperandAst, PolicyAst, PolicyEffectAst, RecalculationAst, RelationOperatorAst,
    RoleAst, ScreenAst, ScreenOperationKindAst, SumDerivationAst, TypeReferenceAst,
};
pub use diagnostic::render_diagnostic;
pub use formatter::{FormatError, format_document};
pub use lowering::{KoreanFrontend, LowerOutput, lower};
pub use parser::{ParseOutput, parse};
pub use rspdl_domain::{Diagnostic, Severity, TextRange as Span};
pub use scanner::{ScanOutput, Token, TokenKind, scan};
