//! Deterministic controlled-Korean scanner, parser, formatter, and lowering.

#![forbid(unsafe_code)]

mod ast;
mod diagnostic;
mod formatter;
#[cfg(test)]
mod generated;
mod lowering;
mod parser;
mod scanner;

pub use ast::{
    ActionAst, ActionDataMutationAst, ActionInputAst, ActionInputKindAst, ConstraintAst,
    ConstraintExpressionAst, CreationBranchAst, CreationDecisionAst, DataModelAst,
    DataMutationKindAst, DeclarationAst, DocumentAst, EnumAst, EnumValueAst, FieldAst,
    FieldIntentAst, FieldIntentKindAst, FieldProducerAst, FieldProducerConditionAst,
    FieldProducerSourceAst, LiteralAst, ModuleAst, NamedIdAst, OperandAst, PolicyAst,
    PolicyEffectAst, RecalculationAst, RelationOperatorAst, RoleAst, ScreenAst,
    ScreenOperationKindAst, SumDerivationAst, TypeReferenceAst,
};
pub use diagnostic::render_diagnostic;
pub use formatter::{FormatError, format_document};
pub use lowering::{KoreanFrontend, LowerOutput, lower};
pub use parser::{ParseOutput, parse};
pub use rspdl_domain::{Diagnostic, Severity, TextRange as Span};
pub use scanner::{ScanOutput, Token, TokenKind, scan};
