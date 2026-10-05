//! Locale-neutral guarded statements. This slice links and type-checks intent;
//! it does not execute effects or prove total condition-space consistency.
use crate::*;
use serde::Serialize;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum StatementPolicy {
    Can,
    Cannot,
    DoAttempt,
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(tag = "kind", content = "definition", rename_all = "snake_case")]
pub enum UnlinkedStatementActor {
    System,
    Role(SurfaceRef),
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct UnlinkedStatementBinding {
    pub declaration: UnlinkedDeclaration,
    pub input: SurfaceRef,
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(tag = "kind", content = "definition", rename_all = "snake_case")]
pub enum UnlinkedStatementOperand {
    Input(SurfaceRef),
    InputField {
        binding: SurfaceRef,
        field: SurfaceRef,
    },
    Literal(UnlinkedLiteral),
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(tag = "kind", content = "definition", rename_all = "snake_case")]
pub enum UnlinkedStatementCondition {
    True,
    Compare {
        left: UnlinkedStatementOperand,
        operator: RelationOperator,
        right: UnlinkedStatementOperand,
    },
    And(Vec<UnlinkedStatementCondition>),
    Or(Vec<UnlinkedStatementCondition>),
    Not(Box<UnlinkedStatementCondition>),
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct UnlinkedStatementAssignment {
    pub field: SurfaceRef,
    pub value: UnlinkedStatementOperand,
    pub span: TextRange,
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(tag = "kind", content = "definition", rename_all = "snake_case")]
pub enum UnlinkedStatementEffect {
    Read {
        binding: SurfaceRef,
        span: TextRange,
    },
    Delete {
        binding: SurfaceRef,
        span: TextRange,
    },
    Update {
        binding: SurfaceRef,
        assignments: Vec<UnlinkedStatementAssignment>,
        span: TextRange,
    },
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct UnlinkedStatement {
    pub declaration: UnlinkedDeclaration,
    pub trigger: UnlinkedProductionTrigger,
    pub actor: UnlinkedStatementActor,
    pub actor_span: TextRange,
    pub bindings: Vec<UnlinkedStatementBinding>,
    pub condition: UnlinkedStatementCondition,
    pub policy: StatementPolicy,
    pub effects: Vec<UnlinkedStatementEffect>,
    pub span: TextRange,
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(tag = "kind", content = "definition", rename_all = "snake_case")]
pub enum StatementActor {
    System,
    Role(CanonicalId),
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(tag = "kind", content = "definition", rename_all = "snake_case")]
pub enum StatementBindingKind {
    ExistingModel { model_id: CanonicalId },
    Value { value_type: CanonicalType },
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct StatementBinding {
    pub id: CanonicalId,
    pub name: String,
    pub input_id: CanonicalId,
    pub input_span: TextRange,
    pub phase: ProducerPhase,
    pub kind: StatementBindingKind,
    pub span: TextRange,
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(tag = "kind", content = "definition", rename_all = "snake_case")]
pub enum StatementOperand {
    Input {
        binding_id: CanonicalId,
        input_id: CanonicalId,
        value_type: CanonicalType,
        phase: ProducerPhase,
        span: TextRange,
    },
    InputField {
        binding_id: CanonicalId,
        input_id: CanonicalId,
        model_id: CanonicalId,
        field_id: CanonicalId,
        value_type: CanonicalType,
        phase: ProducerPhase,
        span: TextRange,
    },
    Constant {
        value: CanonicalValue,
        span: TextRange,
    },
}
impl StatementOperand {
    pub fn value_type(&self) -> &CanonicalType {
        match self {
            Self::Input { value_type, .. } | Self::InputField { value_type, .. } => value_type,
            Self::Constant { value, .. } => value.value_type(),
        }
    }
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(tag = "kind", content = "definition", rename_all = "snake_case")]
pub enum StatementCondition {
    True,
    Compare {
        left: Box<StatementOperand>,
        operator: RelationOperator,
        right: Box<StatementOperand>,
    },
    And(Vec<StatementCondition>),
    Or(Vec<StatementCondition>),
    Not(Box<StatementCondition>),
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct StatementAssignment {
    pub field_id: CanonicalId,
    pub value: StatementOperand,
    pub span: TextRange,
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(tag = "kind", content = "definition", rename_all = "snake_case")]
pub enum StatementEffect {
    Read {
        binding_id: CanonicalId,
        model_id: CanonicalId,
        span: TextRange,
    },
    Delete {
        binding_id: CanonicalId,
        model_id: CanonicalId,
        span: TextRange,
    },
    Update {
        binding_id: CanonicalId,
        model_id: CanonicalId,
        assignments: Vec<StatementAssignment>,
        span: TextRange,
    },
}
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum StatementAnalysisStatus {
    LinkedAndTypeCheckedOnly,
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct StatementDefinition {
    pub id: CanonicalId,
    pub name: String,
    pub trigger_span: TextRange,
    pub actor_span: TextRange,
    pub trigger: ProductionTriggerDefinition,
    pub actor: StatementActor,
    pub bindings: Vec<StatementBinding>,
    pub condition: StatementCondition,
    pub policy: StatementPolicy,
    pub effects: Vec<StatementEffect>,
    pub analysis_status: StatementAnalysisStatus,
    pub span: TextRange,
}
