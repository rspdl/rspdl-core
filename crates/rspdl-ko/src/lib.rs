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
    ActionAst, ActionDataMutationAst, ActionInputAst, ActionInputKindAst, ActionOutcomeAst,
    ActionOutcomesAst, CategoryAst, ConstraintAst, ConstraintExpressionAst, CreationBranchAst,
    CreationDecisionAst, DataModelAst, DataMutationKindAst, DeclarationAst, DocumentAst, EnumAst,
    EnumValueAst, EventAst, EventInputAst, FieldAst, FieldIntentAst, FieldIntentKindAst,
    FieldProducerAst, FieldProducerConditionAst, FieldProducerSourceAst, FrontmatterAst,
    FrontmatterRefAst, HandlerKindAst, LayoutElementAst, LiteralAst, LookupResultAst, ModuleAst,
    NamedIdAst, OperandAst, OutcomeDataAst, OutcomeDataSourceAst, OutcomeKindAst, PolicyAst,
    PolicyEffectAst, RecalculationAst, RecoveryAst, RecoveryKindAst, RelationOperatorAst,
    RelationProducerAst, RoleAst, SameScreenHandlerAst, ScreenAst, ScreenLayoutAst,
    ScreenLayoutKindAst, ScreenOperationKindAst, ScreenPathAst, ScreenPermissionAst,
    SumDerivationAst, TypeReferenceAst, WorkflowAcquisitionAst, WorkflowAst, WorkflowCompletionAst,
    WorkflowDataAst,
};
pub use diagnostic::render_diagnostic;
pub use formatter::{FormatError, FormatOutput, format_document, format_source};
pub use lowering::{KoreanFrontend, LowerOutput, lower};
pub use parser::{ParseOutput, parse};
pub use rspdl_domain::{Diagnostic, Severity, TextRange as Span};
pub use scanner::{ScanOutput, Token, TokenKind, scan};
