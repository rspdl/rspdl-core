//! Locale-independent product concepts lowered by surface-language frontends.

use serde::Serialize;

use crate::{CanonicalId, CanonicalType, CanonicalValue, EnumType, SourceId, TextRange};

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct EnumVariantDefinition {
    pub id: CanonicalId,
    pub local_id: CanonicalId,
    pub name: String,
    pub span: TextRange,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct EnumDefinition {
    pub id: CanonicalId,
    pub name: String,
    pub enum_type: EnumType,
    pub variants: Vec<EnumVariantDefinition>,
    pub span: TextRange,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct FieldDefinition {
    pub id: CanonicalId,
    pub local_id: CanonicalId,
    pub name: String,
    pub required: bool,
    pub value_type: CanonicalType,
    pub span: TextRange,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct DataModelDefinition {
    pub id: CanonicalId,
    pub name: String,
    pub fields: Vec<FieldDefinition>,
    pub span: TextRange,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct RelationDefinition {
    pub id: CanonicalId,
    pub name: String,
    /// Ordered entity sorts. The first parameter is the anchor used by
    /// relation cardinality constraints.
    pub parameter_model_ids: Vec<CanonicalId>,
    pub span: TextRange,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum RelationalConstraintKind {
    NonEmpty {
        model_id: CanonicalId,
    },
    Required {
        relation_id: CanonicalId,
    },
    Unique {
        relation_id: CanonicalId,
    },
    Exclusive {
        relation_ids: Vec<CanonicalId>,
    },
    Exhaustive {
        relation_ids: Vec<CanonicalId>,
    },
    /// Declares compatible overlap without asserting that overlap exists.
    Coexistent {
        relation_ids: Vec<CanonicalId>,
    },
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct RelationalConstraintDefinition {
    pub id: CanonicalId,
    pub constraint: RelationalConstraintKind,
    pub span: TextRange,
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ScreenOperationKind {
    Create,
    Read,
    Input,
    Update,
    Delete,
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize)]
pub struct ScreenOperationDefinition {
    pub kind: ScreenOperationKind,
    pub model_id: CanonicalId,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub field_ids: Vec<CanonicalId>,
    pub span: TextRange,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct ScreenDefinition {
    pub id: CanonicalId,
    pub name: String,
    pub operations: Vec<ScreenOperationDefinition>,
    pub span: TextRange,
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum DataMutationKind {
    Create,
    Update,
    Delete,
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize)]
pub struct ActionDataMutationDefinition {
    pub action_id: CanonicalId,
    pub model_id: CanonicalId,
    pub mutation: DataMutationKind,
    pub span: TextRange,
}

/// Source sidecar for one resolved action data mutation.
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize)]
pub struct ActionDataMutationProvenance {
    pub action_id: CanonicalId,
    pub model_id: CanonicalId,
    pub mutation: DataMutationKind,
    pub source_id: SourceId,
    pub span: TextRange,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum DerivationExpression {
    Sum { source_field_id: CanonicalId },
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct DerivationDefinition {
    pub target_field_id: CanonicalId,
    pub expression: DerivationExpression,
    pub recalculate_when_changed_field_ids: Vec<CanonicalId>,
    pub span: TextRange,
}

/// One explicit recalculation declaration retained for source navigation.
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize)]
pub struct RecalculationDefinition {
    pub source_field_id: CanonicalId,
    pub target_field_id: CanonicalId,
    pub span: TextRange,
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum FieldIntentKind {
    Internal,
    Hidden,
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize)]
pub struct FieldIntentDefinition {
    pub field_id: CanonicalId,
    pub intent: FieldIntentKind,
    pub span: TextRange,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RelationOperator {
    Equal,
    NotEqual,
    LessThan,
    LessThanOrEqual,
    GreaterThan,
    GreaterThanOrEqual,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(tag = "kind", content = "value", rename_all = "snake_case")]
pub enum ConstraintOperand {
    Field(CanonicalId),
    Constant(CanonicalValue),
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct ConstraintDefinition {
    pub id: CanonicalId,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub name: String,
    pub model_id: CanonicalId,
    pub left: ConstraintOperand,
    pub operator: RelationOperator,
    pub right: ConstraintOperand,
    pub span: TextRange,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct RoleDefinition {
    pub id: CanonicalId,
    pub name: String,
    pub span: TextRange,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct ActionDefinition {
    pub id: CanonicalId,
    pub name: String,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub inputs: Vec<ActionInputDefinition>,
    pub span: TextRange,
}

/// One typed, explicitly named input declared by an action.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct ActionInputDefinition {
    pub id: CanonicalId,
    pub local_id: CanonicalId,
    pub name: String,
    pub kind: ActionInputKind,
    pub span: TextRange,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(tag = "kind", content = "definition", rename_all = "snake_case")]
pub enum ActionInputKind {
    ExistingModel { model_id: CanonicalId },
    Value { value_type: CanonicalType },
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CreationDecision {
    Create,
    Skip,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ProductionCardinality {
    ExactlyOne,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct CreationBranchDefinition {
    pub id: CanonicalId,
    pub variant_id: CanonicalId,
    pub decision: CreationDecision,
    pub span: TextRange,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct ConditionalProductionDefinition {
    pub id: CanonicalId,
    pub action_id: CanonicalId,
    pub output_model_id: CanonicalId,
    pub instance_cardinality: ProductionCardinality,
    pub decision_input_id: CanonicalId,
    pub branches: Vec<CreationBranchDefinition>,
    pub span: TextRange,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PolicyEffect {
    Allow,
    Deny,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct PolicyDefinition {
    pub id: CanonicalId,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub name: String,
    pub role_id: CanonicalId,
    pub model_id: CanonicalId,
    pub field_id: CanonicalId,
    pub action_id: CanonicalId,
    pub effect: PolicyEffect,
    pub span: TextRange,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct SemanticModule {
    pub id: CanonicalId,
    pub name: String,
    pub span: TextRange,
    pub enums: Vec<EnumDefinition>,
    pub models: Vec<DataModelDefinition>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub relations: Vec<RelationDefinition>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub relational_constraints: Vec<RelationalConstraintDefinition>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub screens: Vec<ScreenDefinition>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub action_data_mutations: Vec<ActionDataMutationDefinition>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub derivations: Vec<DerivationDefinition>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub recalculations: Vec<RecalculationDefinition>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub field_intents: Vec<FieldIntentDefinition>,
    pub constraints: Vec<ConstraintDefinition>,
    pub roles: Vec<RoleDefinition>,
    pub actions: Vec<ActionDefinition>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub conditional_productions: Vec<ConditionalProductionDefinition>,
    pub policies: Vec<PolicyDefinition>,
}
