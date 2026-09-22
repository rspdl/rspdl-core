//! Deterministic controlled-Korean scanner, parser, formatter, and lowering.

#![forbid(unsafe_code)]

mod ast;
mod diagnostic;
mod formatter;
mod frontmatter;
#[cfg(test)]
mod generated;
mod lowering;
mod parser;
mod scanner;

pub use ast::{
    ActionAst, ActionDataMutationAst, ActionInputAst, ActionInputKindAst, CategoryAst,
    ConstraintAst, ConstraintExpressionAst, CreationBranchAst, CreationDecisionAst, DataModelAst,
    DataMutationKindAst, DeclarationAst, DocumentAst, EnumAst, EnumValueAst, EventAst,
    EventInputAst, FieldAst, FieldIntentAst, FieldIntentKindAst, FieldProducerAst,
    FieldProducerConditionAst, FieldProducerSourceAst, FrontmatterAst, FrontmatterRefAst,
    LayoutElementAst, LiteralAst, ModuleAst, NamedIdAst, OperandAst, PolicyAst, PolicyEffectAst,
    RecalculationAst, RelationOperatorAst, RelationProducerAst, RoleAst, ScreenAst,
    ScreenLayoutAst, ScreenLayoutKindAst, ScreenOperationKindAst, ScreenPathAst, SumDerivationAst,
    TypeReferenceAst, WorkflowAcquisitionAst, WorkflowAst, WorkflowCompletionAst, WorkflowDataAst,
};
pub use diagnostic::render_diagnostic;
pub use formatter::{FormatError, FormatOutput, format_document, format_source};
pub use lowering::{KoreanFrontend, LowerOutput, lower};
pub use parser::{ParseOutput, parse};
pub use rspdl_domain::{Diagnostic, Severity, TextRange as Span};
pub use scanner::{ScanOutput, Token, TokenKind, scan};
