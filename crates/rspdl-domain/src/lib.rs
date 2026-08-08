//! Locale-independent semantic building blocks for RSPDL.
//!
//! This crate deliberately contains no surface-language concepts. Every value,
//! set, and logical term has an explicit canonical type so later Datalog and
//! SMT backends can reject unsupported models instead of approximating them.

#![forbid(unsafe_code)]

pub mod analysis;
pub mod diagnostic;
pub mod domain;
pub mod error;
pub mod frontend;
pub mod logic;
pub mod rule;
pub mod semantic;
pub mod set;
pub mod solver;
pub mod source;
pub mod types;
pub mod value;

pub use analysis::{AnalysisOutput, analyze};
pub use diagnostic::{Diagnostic, Severity};
pub use domain::{
    Backend, Cardinality, Domain, DomainCapabilities, EnumerationSupport, GroundMembership,
    InfiniteDomain, SymbolicSupport,
};
pub use error::ModelError;
pub use frontend::{
    Frontend, FrontendOutput, SurfaceRef, UnlinkedAction, UnlinkedConstraint, UnlinkedDataModel,
    UnlinkedDeclaration, UnlinkedEnum, UnlinkedEnumVariant, UnlinkedField, UnlinkedFieldIntent,
    UnlinkedLiteral, UnlinkedModule, UnlinkedOperand, UnlinkedPolicy, UnlinkedRecalculation,
    UnlinkedRole, UnlinkedScreen, UnlinkedSumDerivation, UnlinkedTypeReference,
};
pub use logic::{
    Atom, AtomView, BooleanExpression, BooleanExpressionView, ComparisonOperator,
    PredicateSignature, Term, Variable,
};
pub use rule::{DerivationRule, Fact, LogicProgram, PredicateApplication, RuleLiteral};
pub use semantic::{
    ActionDefinition, ConstraintDefinition, ConstraintOperand, DataModelDefinition,
    DerivationDefinition, DerivationExpression, EnumDefinition, EnumVariantDefinition,
    FieldDefinition, FieldIntentDefinition, FieldIntentKind, PolicyDefinition, PolicyEffect,
    RelationOperator, RoleDefinition, ScreenDefinition, ScreenOperationDefinition,
    ScreenOperationKind, SemanticModule,
};
pub use set::{SetExpression, SetExpressionView};
pub use solver::SolverContractError;
pub use solver::{
    CanonicalModel, ConstraintProblem, ConstraintSolver, SolveOptions, SolveResult, VariableDomain,
};
pub use source::TextRange;
pub use types::{BuiltinRefinement, CanonicalId, CanonicalType, EnumType, RefinementType};
pub use value::{CanonicalInteger, CanonicalValue};
