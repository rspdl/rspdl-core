use rspdl_domain::TextRange as Span;
use serde::Serialize;

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct NamedIdAst {
    #[serde(skip_serializing_if = "String::is_empty")]
    pub name: String,
    pub id: String,
    pub span: Span,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct ModuleAst {
    pub declaration: NamedIdAst,
    pub span: Span,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct EnumValueAst {
    pub declaration: NamedIdAst,
    pub span: Span,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct EnumAst {
    pub declaration: NamedIdAst,
    pub values: Vec<EnumValueAst>,
    pub span: Span,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(tag = "kind", content = "name", rename_all = "snake_case")]
pub enum TypeReferenceAst {
    String,
    Integer,
    Boolean,
    Named(String),
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct FieldAst {
    pub declaration: NamedIdAst,
    pub required: bool,
    pub value_type: TypeReferenceAst,
    pub span: Span,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct DataModelAst {
    pub declaration: NamedIdAst,
    pub fields: Vec<FieldAst>,
    pub span: Span,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct RelationAst {
    pub declaration: NamedIdAst,
    /// Ordered model parameters. `required` and `unique` use the first model
    /// as their anchor.
    pub parameter_models: Vec<String>,
    pub span: Span,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum RelationalConstraintKindAst {
    NonEmpty { model: String },
    Required { model: String, relation: String },
    Unique { model: String, relation: String },
    Exclusive { relations: Vec<String> },
    Exhaustive { relations: Vec<String> },
    Coexistent { relations: Vec<String> },
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct RelationalConstraintAst {
    pub constraint: RelationalConstraintKindAst,
    pub span: Span,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ScreenOperationKindAst {
    Create,
    Read,
    Input,
    Update,
    Delete,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct ScreenAst {
    pub declaration: NamedIdAst,
    pub model: String,
    pub fields: Vec<String>,
    pub operation: ScreenOperationKindAst,
    pub span: Span,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum DataMutationKindAst {
    Create,
    Update,
    Delete,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct ActionDataMutationAst {
    pub action: String,
    pub model: String,
    pub mutation: DataMutationKindAst,
    pub span: Span,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct SumDerivationAst {
    pub target_model: String,
    pub target_field: String,
    pub source_model: String,
    pub source_field: String,
    pub span: Span,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct RecalculationAst {
    pub source_model: String,
    pub source_field: String,
    pub target_model: String,
    pub target_field: String,
    pub span: Span,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum FieldIntentKindAst {
    Internal,
    Hidden,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct FieldIntentAst {
    pub model: String,
    pub field: String,
    pub intent: FieldIntentKindAst,
    pub span: Span,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RelationOperatorAst {
    Equal,
    NotEqual,
    LessThan,
    LessThanOrEqual,
    GreaterThan,
    GreaterThanOrEqual,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(tag = "kind", content = "value", rename_all = "snake_case")]
pub enum LiteralAst {
    String(String),
    Integer(String),
    Boolean(bool),
    Named(String),
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(tag = "kind", content = "value", rename_all = "snake_case")]
pub enum OperandAst {
    Field(String),
    Literal(LiteralAst),
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct ConstraintExpressionAst {
    pub model: String,
    pub left: OperandAst,
    pub operator: RelationOperatorAst,
    pub right: OperandAst,
    pub span: Span,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct ConstraintAst {
    pub declaration: NamedIdAst,
    pub expression: ConstraintExpressionAst,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct RoleAst {
    pub declaration: NamedIdAst,
    pub span: Span,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct ActionAst {
    pub declaration: NamedIdAst,
    pub span: Span,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ActionInputKindAst {
    ExistingModel { model: String },
    Value { value_type: TypeReferenceAst },
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct ActionInputAst {
    pub action: String,
    pub declaration: NamedIdAst,
    pub kind: ActionInputKindAst,
    pub span: Span,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PolicyEffectAst {
    Allow,
    Deny,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct PolicyAst {
    pub declaration: NamedIdAst,
    pub role: String,
    pub model: String,
    pub field: String,
    pub action: String,
    pub effect: PolicyEffectAst,
    pub span: Span,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(tag = "kind", content = "definition", rename_all = "snake_case")]
pub enum DeclarationAst {
    Enum(EnumAst),
    DataModel(DataModelAst),
    Relation(RelationAst),
    RelationalConstraint(RelationalConstraintAst),
    Screen(ScreenAst),
    ActionDataMutation(ActionDataMutationAst),
    SumDerivation(SumDerivationAst),
    Recalculation(RecalculationAst),
    FieldIntent(FieldIntentAst),
    Constraint(ConstraintAst),
    Role(RoleAst),
    Action(ActionAst),
    ActionInput(ActionInputAst),
    Policy(PolicyAst),
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct DocumentAst {
    pub module: ModuleAst,
    pub declarations: Vec<DeclarationAst>,
}
