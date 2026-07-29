use thiserror::Error;

use crate::types::{CanonicalId, CanonicalType};

/// A construction error for the canonical semantic model.
///
/// Invalid states are rejected at construction boundaries instead of being
/// carried into a backend.
#[derive(Clone, Debug, Eq, PartialEq, Error)]
pub enum ModelError {
    #[error("`{value}` is not a canonical machine identifier")]
    InvalidCanonicalId { value: String },

    #[error("enum type `{type_id}` must declare at least one variant")]
    EmptyEnum { type_id: CanonicalId },

    #[error("`{variant}` is not a variant of enum type `{type_id}`")]
    UnknownEnumVariant {
        type_id: CanonicalId,
        variant: CanonicalId,
    },

    #[error("refinement `{refinement}` requires `{expected}`, but received `{actual}`")]
    InvalidRefinementBase {
        refinement: &'static str,
        expected: CanonicalType,
        actual: CanonicalType,
    },

    #[error("value `{value}` does not satisfy type `{value_type}`")]
    InvalidRefinedValue {
        value_type: CanonicalType,
        value: String,
    },

    #[error(
        "value `{value}` exceeds the supported magnitude for refinement `{refinement}` (maximum `{maximum}`)"
    )]
    RefinementMagnitudeExceeded {
        refinement: &'static str,
        value: String,
        maximum: &'static str,
    },

    #[error("type mismatch in {context}: expected `{expected}`, received `{actual}`")]
    TypeMismatch {
        context: &'static str,
        expected: CanonicalType,
        actual: CanonicalType,
    },

    #[error("{operation} requires at least one operand")]
    EmptyOperands { operation: &'static str },

    #[error(
        "predicate `{predicate}` expects {expected} arguments, but received {actual} arguments"
    )]
    ArityMismatch {
        predicate: CanonicalId,
        expected: usize,
        actual: usize,
    },

    #[error("`{value}` is not a canonical base-10 integer")]
    InvalidInteger { value: String },

    #[error("predicate `{predicate}` is not declared")]
    UnknownPredicate { predicate: CanonicalId },
    #[error("predicate `{predicate}` has conflicting signatures")]
    ConflictingPredicateSignature { predicate: CanonicalId },

    #[error("fact for predicate `{predicate}` contains variable `{variable}`")]
    NonGroundFact {
        predicate: CanonicalId,
        variable: CanonicalId,
    },
}
