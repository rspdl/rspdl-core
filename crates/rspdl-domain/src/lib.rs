//! Locale-independent semantic building blocks for RSPDL.
//!
//! This crate deliberately contains no surface-language concepts. Every value,
//! set, and logical term has an explicit canonical type so runtime matching and
//! SMT backends can reject unsupported models instead of approximating them.

#![forbid(unsafe_code)]

pub mod analysis;
pub mod diagnostic;
pub mod domain;
pub mod error;
pub mod frontend;
pub mod logic;
pub mod policy_analysis;
pub mod relational_analysis;
pub mod semantic;
pub mod set;
pub mod solver;
pub mod source;
pub mod types;
pub mod value;

pub use analysis::{AnalysisOutput, analyze, analyze_with_source};
pub use diagnostic::{Diagnostic, Severity};
pub use domain::{
    Backend, Cardinality, Domain, DomainCapabilities, EnumerationSupport, GroundMembership,
    InfiniteDomain, SymbolicSupport,
};
pub use error::ModelError;
pub use frontend::{
    Frontend, FrontendOutput, SurfaceRef, UnlinkedAction, UnlinkedActionDataMutation,
    UnlinkedActionInput, UnlinkedActionInputKind, UnlinkedConstraint, UnlinkedCreationBranch,
    UnlinkedDataModel, UnlinkedDeclaration, UnlinkedEnum, UnlinkedEnumVariant, UnlinkedField,
    UnlinkedFieldIntent, UnlinkedLiteral, UnlinkedModule, UnlinkedOperand, UnlinkedPolicy,
    UnlinkedRecalculation, UnlinkedRelation, UnlinkedRelationalConstraint,
    UnlinkedRelationalConstraintKind, UnlinkedRole, UnlinkedScreen, UnlinkedSumDerivation,
    UnlinkedTypeReference,
};
pub use logic::{
    Atom, AtomView, BooleanExpression, BooleanExpressionView, ComparisonOperator,
    PredicateSignature, Term, Variable,
};
pub use policy_analysis::{
    AnalysisUnknown, CompatibleOverlap, DecisionPointError, EnumGap, PolicyAnalysisError,
    PolicyAnalysisQuery, PolicyBranch, PolicyConflict, TotalDecisionAnalysis, TotalDecisionPoint,
    analyze_total_decision_point,
};
pub use relational_analysis::{
    BoundedModelConfigurationError, BoundedModelOptions, BoundedModelResult,
    MAX_BOUNDED_SCOPE_PER_MODEL, RelationalAnalysisError, RelationalWitness, VirtualEntity,
    VirtualFieldValue, VirtualRelationTuple, find_bounded_relational_model,
};
pub use semantic::{
    ActionDataMutationDefinition, ActionDataMutationProvenance, ActionDefinition,
    ActionInputDefinition, ActionInputKind, ConditionalProductionDefinition, ConstraintDefinition,
    ConstraintOperand, CreationBranchDefinition, CreationDecision, DataModelDefinition,
    DataMutationKind, DerivationDefinition, DerivationExpression, EnumDefinition,
    EnumVariantDefinition, FieldDefinition, FieldIntentDefinition, FieldIntentKind,
    PolicyDefinition, PolicyEffect, ProductionCardinality, RecalculationDefinition,
    RelationDefinition, RelationOperator, RelationalConstraintDefinition, RelationalConstraintKind,
    RoleDefinition, ScreenDefinition, ScreenOperationDefinition, ScreenOperationKind,
    SemanticModule,
};
pub use set::{SetExpression, SetExpressionView};
pub use solver::SolverContractError;
pub use solver::{
    CanonicalModel, ConstraintProblem, ConstraintSolver, SolveOptions, SolveResult, VariableDomain,
};
pub use source::{SourceId, TextRange};
pub use types::{BuiltinRefinement, CanonicalId, CanonicalType, EnumType, RefinementType};
pub use value::{CanonicalInteger, CanonicalValue};
