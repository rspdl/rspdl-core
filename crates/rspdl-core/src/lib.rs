//! Locale-independent semantic building blocks for RSPDL.
//!
//! This crate deliberately contains no surface-language concepts. Every value,
//! set, and logical term has an explicit canonical type so later Datalog and
//! SMT backends can reject unsupported models instead of approximating them.

#![forbid(unsafe_code)]

pub mod domain;
pub mod error;
pub mod logic;
pub mod rule;
pub mod set;
pub mod solver;
pub mod types;
pub mod value;

pub use domain::{
    Backend, Cardinality, Domain, DomainCapabilities, EnumerationSupport, GroundMembership,
    InfiniteDomain, SymbolicSupport,
};
pub use error::ModelError;
pub use logic::{
    Atom, AtomView, BooleanExpression, BooleanExpressionView, ComparisonOperator,
    PredicateSignature, Term, Variable,
};
pub use rule::{DerivationRule, Fact, LogicProgram, PredicateApplication, RuleLiteral};
pub use set::SetExpression;
pub use solver::SolverContractError;
pub use solver::{
    CanonicalModel, ConstraintProblem, ConstraintSolver, SolveOptions, SolveResult, VariableDomain,
};
pub use types::{BuiltinRefinement, CanonicalId, CanonicalType, EnumType, RefinementType};
pub use value::{CanonicalInteger, CanonicalValue};
