use serde::Serialize;

use crate::TextRange;
use crate::{
    DataMutationKind, Diagnostic, FieldIntentKind, PolicyEffect, RelationOperator,
    ScreenOperationKind,
};

/// Behavior contract implemented by every surface-language frontend.
///
/// A frontend owns scanning, parsing, surface linting, and desugaring. It must
/// not resolve symbols, type-check references, or run semantic rules.
pub trait Frontend {
    fn language_id(&self) -> &'static str;

    fn lower_source(&self, source: &str) -> FrontendOutput;
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct FrontendOutput {
    pub module: Option<UnlinkedModule>,
    pub diagnostics: Vec<Diagnostic>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct UnlinkedDeclaration {
    #[serde(skip_serializing_if = "String::is_empty")]
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    pub span: TextRange,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct SurfaceRef {
    /// Locale-independent declaration ID, either module-local or fully qualified.
    pub id: String,
    pub span: TextRange,
}

impl SurfaceRef {
    pub fn stable_id(id: impl Into<String>, span: TextRange) -> Self {
        Self {
            id: id.into(),
            span,
        }
    }

    pub fn id(&self) -> &str {
        &self.id
    }

    pub const fn span(&self) -> TextRange {
        self.span
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct UnlinkedEnumVariant {
    pub declaration: UnlinkedDeclaration,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct UnlinkedEnum {
    pub declaration: UnlinkedDeclaration,
    pub variants: Vec<UnlinkedEnumVariant>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(tag = "kind", content = "reference", rename_all = "snake_case")]
pub enum UnlinkedTypeReference {
    String,
    Integer,
    Boolean,
    Named(SurfaceRef),
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct UnlinkedField {
    pub declaration: UnlinkedDeclaration,
    pub required: bool,
    pub value_type: UnlinkedTypeReference,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct UnlinkedDataModel {
    pub declaration: UnlinkedDeclaration,
    pub fields: Vec<UnlinkedField>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct UnlinkedRelation {
    pub declaration: UnlinkedDeclaration,
    /// Ordered relation parameters. The first parameter is the anchor used by
    /// `required` and `unique` constraints.
    pub parameter_models: Vec<SurfaceRef>,
    pub span: TextRange,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum UnlinkedRelationalConstraintKind {
    NonEmpty {
        model: SurfaceRef,
    },
    Required {
        model: SurfaceRef,
        relation: SurfaceRef,
    },
    Unique {
        model: SurfaceRef,
        relation: SurfaceRef,
    },
    Exclusive {
        relations: Vec<SurfaceRef>,
    },
    Exhaustive {
        relations: Vec<SurfaceRef>,
    },
    /// Explicitly records that overlap is compatible. It does not require an
    /// overlapping tuple to exist.
    Coexistent {
        relations: Vec<SurfaceRef>,
    },
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct UnlinkedRelationalConstraint {
    pub declaration: UnlinkedDeclaration,
    pub constraint: UnlinkedRelationalConstraintKind,
    pub span: TextRange,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct UnlinkedScreen {
    pub declaration: UnlinkedDeclaration,
    pub model: SurfaceRef,
    pub fields: Vec<SurfaceRef>,
    pub operation: ScreenOperationKind,
    pub span: TextRange,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct UnlinkedActionDataMutation {
    pub action: SurfaceRef,
    pub model: SurfaceRef,
    pub mutation: DataMutationKind,
    pub span: TextRange,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct UnlinkedSumDerivation {
    pub target_model: SurfaceRef,
    pub target_field: SurfaceRef,
    pub source_model: SurfaceRef,
    pub source_field: SurfaceRef,
    pub span: TextRange,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct UnlinkedRecalculation {
    pub source_model: SurfaceRef,
    pub source_field: SurfaceRef,
    pub target_model: SurfaceRef,
    pub target_field: SurfaceRef,
    pub span: TextRange,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct UnlinkedFieldIntent {
    pub model: SurfaceRef,
    pub field: SurfaceRef,
    pub intent: FieldIntentKind,
    pub span: TextRange,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(tag = "kind", content = "value", rename_all = "snake_case")]
pub enum UnlinkedLiteral {
    String { value: String, span: TextRange },
    Integer { value: String, span: TextRange },
    Boolean { value: bool, span: TextRange },
    Named(SurfaceRef),
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(tag = "kind", content = "value", rename_all = "snake_case")]
pub enum UnlinkedOperand {
    Field(SurfaceRef),
    Literal(UnlinkedLiteral),
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct UnlinkedConstraint {
    pub declaration: UnlinkedDeclaration,
    pub model: SurfaceRef,
    pub left: UnlinkedOperand,
    pub operator: RelationOperator,
    pub right: UnlinkedOperand,
    pub span: TextRange,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct UnlinkedRole {
    pub declaration: UnlinkedDeclaration,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct UnlinkedAction {
    pub declaration: UnlinkedDeclaration,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct UnlinkedPolicy {
    pub declaration: UnlinkedDeclaration,
    pub role: SurfaceRef,
    pub model: SurfaceRef,
    pub field: SurfaceRef,
    pub action: SurfaceRef,
    pub effect: PolicyEffect,
    pub span: TextRange,
}

/// Locale-neutral, unresolved semantic intent produced by a frontend.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct UnlinkedModule {
    pub declaration: UnlinkedDeclaration,
    pub enums: Vec<UnlinkedEnum>,
    pub models: Vec<UnlinkedDataModel>,
    pub relations: Vec<UnlinkedRelation>,
    pub relational_constraints: Vec<UnlinkedRelationalConstraint>,
    pub screens: Vec<UnlinkedScreen>,
    pub action_data_mutations: Vec<UnlinkedActionDataMutation>,
    pub derivations: Vec<UnlinkedSumDerivation>,
    pub recalculations: Vec<UnlinkedRecalculation>,
    pub field_intents: Vec<UnlinkedFieldIntent>,
    pub constraints: Vec<UnlinkedConstraint>,
    pub roles: Vec<UnlinkedRole>,
    pub actions: Vec<UnlinkedAction>,
    pub policies: Vec<UnlinkedPolicy>,
}
