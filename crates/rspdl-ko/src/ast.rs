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
    Decimal,
    Date,
    Time,
    DateTime,
    Duration,
    Latitude,
    Longitude,
    Money(String),
    Percentage,
    Quantity(String),
    Coordinate,
    LocalDateTime,
    ZonedDateTime,
    CalendarDuration,
    Uuid,
    Email,
    Url,
    PhoneNumber,
    IpAddress,
    Cidr,
    CountryCode,
    LanguageCode,
    CurrencyCode,
    List(Box<TypeReferenceAst>),
    Set(Box<TypeReferenceAst>),
    Map(Box<TypeReferenceAst>, Box<TypeReferenceAst>),
    Reference(String),
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
pub struct EventAst {
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

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct EventInputAst {
    pub event: String,
    pub declaration: NamedIdAst,
    pub kind: ActionInputKindAst,
    pub span: Span,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CreationDecisionAst {
    Create,
    Skip,
}

/// A single explicit enum branch that conditionally creates one output model.
/// The condition is intentionally limited to one direct action input and one
/// enum variant; broader predicates have no Korean surface in this slice.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct CreationBranchAst {
    pub declaration: NamedIdAst,
    pub action: String,
    pub input: String,
    pub variant: String,
    pub output_model: String,
    pub decision: CreationDecisionAst,
    pub span: Span,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(tag = "kind", content = "value", rename_all = "snake_case")]
pub enum FieldProducerSourceAst {
    ActionInput { input: String },
    InputField { input: String, field: String },
    Constant { literal: LiteralAst },
    Template { value: String },
}

/// The only payload condition in this slice: one direct enum action input
/// equal to one of that enum's declared variants.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct FieldProducerConditionAst {
    pub input: String,
    pub variant: String,
}

/// The Korean verb phrase makes the owner explicit: `실행될 때` is an Action
/// invocation and `발생할 때` is an immutable Event payload.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ProducerTriggerKindAst {
    Action,
    Event,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct ProducerTriggerAst {
    pub name: String,
    pub kind: ProducerTriggerKindAst,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct FieldProducerAst {
    pub declaration: NamedIdAst,
    pub trigger: ProducerTriggerAst,
    pub output_model: String,
    pub output_field: String,
    pub source: FieldProducerSourceAst,
    pub condition: Option<FieldProducerConditionAst>,
    pub span: Span,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct RelationProducerAst {
    pub declaration: NamedIdAst,
    pub trigger: ProducerTriggerAst,
    pub input: String,
    pub output_model: String,
    pub relation: String,
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
    Event(EventAst),
    ActionInput(ActionInputAst),
    EventInput(EventInputAst),
    CreationBranch(CreationBranchAst),
    FieldProducer(FieldProducerAst),
    RelationProducer(RelationProducerAst),
    Policy(PolicyAst),
}

/// 머리말이 다른 선언을 가리키는 방법.
///
/// 문장과 같은 방식으로 적으므로 stable ID 가 아니라 한국어 표시 이름이 올 수 있다. 이름
/// 해석은 이후 단계의 일이라 여기서는 적힌 글자를 그대로 들고 있는다.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct FrontmatterRefAst {
    pub text: String,
    pub span: Span,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct CategoryAst {
    pub declaration: NamedIdAst,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub children: Vec<CategoryAst>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub screens: Vec<FrontmatterRefAst>,
    pub span: Span,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ScreenLayoutKindAst {
    Page,
    Popup,
    Tab,
    Link,
}

/// 레이아웃 어휘. 고정 목록이며 좌표나 시각 속성은 담지 않는다 — 그것은 디자인의 영역이다.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum LayoutElementAst {
    Header {
        id: Option<String>,
        children: Vec<LayoutElementAst>,
        span: Span,
    },
    Section {
        id: Option<String>,
        children: Vec<LayoutElementAst>,
        span: Span,
    },
    Heading {
        id: Option<String>,
        text: String,
        span: Span,
    },
    Form {
        id: Option<String>,
        inputs: Vec<LayoutElementAst>,
        span: Span,
    },
    Input {
        id: Option<String>,
        field: FrontmatterRefAst,
        span: Span,
    },
    List {
        id: Option<String>,
        model: FrontmatterRefAst,
        fields: Vec<FrontmatterRefAst>,
        span: Span,
    },
    Button {
        id: String,
        name: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        action: Option<FrontmatterRefAst>,
        span: Span,
    },
    /// 선언할 수 없는 자리의 이름표. 지도와 차트가 여기 들어간다.
    Placeholder {
        id: Option<String>,
        text: String,
        span: Span,
    },
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct ScreenLayoutAst {
    pub screen: FrontmatterRefAst,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub kind: Option<ScreenLayoutKindAst>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub elements: Vec<LayoutElementAst>,
    pub span: Span,
}

/// 흐름 하나. 출발은 화면 안의 요소이고 도착은 화면이다.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct ScreenPathAst {
    pub source_screen: FrontmatterRefAst,
    pub source_element: FrontmatterRefAst,
    pub target_screen: FrontmatterRefAst,
    /// 사람이 읽는 설명. 조건식이 아니므로 해석하지 않는다.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
    pub span: Span,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct WorkflowDataAst {
    pub model: FrontmatterRefAst,
    pub field: FrontmatterRefAst,
    pub span: Span,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct WorkflowCompletionAst {
    pub screen: FrontmatterRefAst,
    pub required_data: Vec<WorkflowDataAst>,
    pub span: Span,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct WorkflowAcquisitionAst {
    pub source_screen: FrontmatterRefAst,
    pub source_element: FrontmatterRefAst,
    pub data: Vec<WorkflowDataAst>,
    pub span: Span,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct WorkflowAst {
    pub declaration: NamedIdAst,
    pub start_screen: FrontmatterRefAst,
    pub initial_data: Vec<WorkflowDataAst>,
    pub acquisitions: Vec<WorkflowAcquisitionAst>,
    pub completions: Vec<WorkflowCompletionAst>,
    pub span: Span,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct FrontmatterAst {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub module: Option<NamedIdAst>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub information_architecture: Vec<CategoryAst>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub screens: Vec<ScreenLayoutAst>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub paths: Vec<ScreenPathAst>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub workflows: Vec<WorkflowAst>,
    pub span: Span,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct DocumentAst {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub frontmatter: Option<FrontmatterAst>,
    pub module: ModuleAst,
    pub declarations: Vec<DeclarationAst>,
}
