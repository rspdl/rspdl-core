//! Locale-independent product concepts lowered by surface-language frontends.

use serde::Serialize;

use crate::{CanonicalId, CanonicalType, CanonicalValue, EnumType};

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct EnumVariantDefinition {
    pub id: CanonicalId,
    pub local_id: CanonicalId,
    pub name: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct EnumDefinition {
    pub id: CanonicalId,
    pub name: String,
    pub enum_type: EnumType,
    pub variants: Vec<EnumVariantDefinition>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct FieldDefinition {
    pub id: CanonicalId,
    pub local_id: CanonicalId,
    pub name: String,
    pub required: bool,
    pub value_type: CanonicalType,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct DataModelDefinition {
    pub id: CanonicalId,
    pub name: String,
    pub fields: Vec<FieldDefinition>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct RelationDefinition {
    pub id: CanonicalId,
    pub name: String,
    /// Ordered entity sorts. The first parameter is the anchor used by
    /// relation cardinality constraints.
    pub parameter_model_ids: Vec<CanonicalId>,
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
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct ScreenDefinition {
    pub id: CanonicalId,
    pub name: String,
    pub operations: Vec<ScreenOperationDefinition>,
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
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct RoleDefinition {
    pub id: CanonicalId,
    pub name: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct ActionDefinition {
    pub id: CanonicalId,
    pub name: String,
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
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct SemanticModule {
    pub id: CanonicalId,
    pub name: String,
    pub enums: Vec<EnumDefinition>,
    pub models: Vec<DataModelDefinition>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub relations: Vec<RelationDefinition>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub relational_constraints: Vec<RelationalConstraintDefinition>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub screens: Vec<ScreenDefinition>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub derivations: Vec<DerivationDefinition>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub field_intents: Vec<FieldIntentDefinition>,
    pub constraints: Vec<ConstraintDefinition>,
    pub roles: Vec<RoleDefinition>,
    pub actions: Vec<ActionDefinition>,
    pub policies: Vec<PolicyDefinition>,
}
