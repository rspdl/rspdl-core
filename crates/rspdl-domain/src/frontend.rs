use serde::Serialize;

use crate::TextRange;
use crate::{
    CreationDecision, DataMutationKind, Diagnostic, FieldIntentKind, PolicyEffect,
    RelationOperator, ScreenOperationKind,
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
    pub span: TextRange,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct UnlinkedEnum {
    pub declaration: UnlinkedDeclaration,
    pub variants: Vec<UnlinkedEnumVariant>,
    pub span: TextRange,
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
    pub span: TextRange,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct UnlinkedDataModel {
    pub declaration: UnlinkedDeclaration,
    pub fields: Vec<UnlinkedField>,
    pub span: TextRange,
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
    pub span: TextRange,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum UnlinkedActionInputKind {
    ExistingModel { model: SurfaceRef },
    Value { value_type: UnlinkedTypeReference },
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(tag = "kind", content = "reference", rename_all = "snake_case")]
pub enum UnlinkedEventInputKind {
    ExistingModel { model: SurfaceRef },
    Value { value_type: UnlinkedTypeReference },
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct UnlinkedActionInput {
    pub declaration: UnlinkedDeclaration,
    pub kind: UnlinkedActionInputKind,
    pub span: TextRange,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct UnlinkedAction {
    pub declaration: UnlinkedDeclaration,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub inputs: Vec<UnlinkedActionInput>,
    pub span: TextRange,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct UnlinkedEventInput {
    pub declaration: UnlinkedDeclaration,
    pub kind: UnlinkedEventInputKind,
    pub span: TextRange,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct UnlinkedEvent {
    pub declaration: UnlinkedDeclaration,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub inputs: Vec<UnlinkedEventInput>,
    pub span: TextRange,
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ProductionTriggerKind {
    Action,
    Event,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct UnlinkedProductionTrigger {
    pub kind: ProductionTriggerKind,
    pub reference: SurfaceRef,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct UnlinkedCreationBranch {
    pub declaration: UnlinkedDeclaration,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub action: Option<SurfaceRef>,
    pub trigger: UnlinkedProductionTrigger,
    pub input: SurfaceRef,
    pub variant: SurfaceRef,
    pub output_model: SurfaceRef,
    pub decision: CreationDecision,
    pub span: TextRange,
}

/// A payload binding owned by either an action invocation or an immutable
/// event payload. A field producer is deliberately separate from a creation
/// branch: this slice attaches it to every `Create` branch of its already
/// declared trigger/output production rather than inferring a payload rule.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(tag = "kind", content = "value", rename_all = "snake_case")]
pub enum UnlinkedFieldProducerSource {
    ActionInput {
        input: SurfaceRef,
    },
    InputField {
        input: SurfaceRef,
        field: SurfaceRef,
    },
    EventInput {
        input: SurfaceRef,
    },
    EventInputField {
        input: SurfaceRef,
        field: SurfaceRef,
    },
    Constant {
        literal: UnlinkedLiteral,
    },
    /// A message template may interpolate only fields of its own output
    /// model.  It deliberately carries no input/model path.
    Template {
        parts: Vec<UnlinkedTemplatePart>,
    },
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(tag = "kind", content = "value", rename_all = "snake_case")]
pub enum UnlinkedTemplatePart {
    Text { value: String },
    OutputField { field: SurfaceRef },
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(tag = "kind", content = "definition", rename_all = "snake_case")]
pub enum UnlinkedFieldProducerCondition {
    EnumVariant {
        input: SurfaceRef,
        variant: SurfaceRef,
    },
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct UnlinkedFieldProducer {
    pub declaration: UnlinkedDeclaration,
    /// Compatibility projection for Action-owned producers. Event producers
    /// intentionally omit this action-shaped field.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub action: Option<SurfaceRef>,
    pub trigger: UnlinkedProductionTrigger,
    pub output_model: SurfaceRef,
    pub output_field: SurfaceRef,
    pub source: UnlinkedFieldProducerSource,
    pub condition: Option<UnlinkedFieldProducerCondition>,
    pub span: TextRange,
}

/// A direct trigger input binding to an output-owned relation slot. Relation
/// slot identity itself is derived from an existing binary relation with both
/// Required and Unique cardinality constraints.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct UnlinkedRelationProducer {
    pub declaration: UnlinkedDeclaration,
    /// Compatibility projection for Action-owned producers. Event producers
    /// intentionally omit this action-shaped field.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub action: Option<SurfaceRef>,
    pub trigger: UnlinkedProductionTrigger,
    pub input: SurfaceRef,
    pub output_model: SurfaceRef,
    pub relation: SurfaceRef,
    pub span: TextRange,
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
    pub span: TextRange,
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
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub events: Vec<UnlinkedEvent>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub creation_branches: Vec<UnlinkedCreationBranch>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub field_producers: Vec<UnlinkedFieldProducer>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub relation_producers: Vec<UnlinkedRelationProducer>,
    pub policies: Vec<UnlinkedPolicy>,
}
