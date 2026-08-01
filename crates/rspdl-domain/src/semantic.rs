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
    pub constraints: Vec<ConstraintDefinition>,
    pub roles: Vec<RoleDefinition>,
    pub actions: Vec<ActionDefinition>,
    pub policies: Vec<PolicyDefinition>,
}
