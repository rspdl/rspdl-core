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

    #[error("`{value}` is not a three-letter uppercase currency code")]
    InvalidCurrencyCode { value: String },

    #[error("type `{value_type}` cannot be used as a deterministic map key")]
    UnsupportedMapKeyType { value_type: CanonicalType },

    #[error("enum type `{type_id}` must declare at least one variant")]
    EmptyEnum { type_id: CanonicalId },

    #[error("enum type `{type_id}` declares variant `{variant}` more than once")]
    DuplicateEnumVariant {
        type_id: CanonicalId,
        variant: CanonicalId,
    },

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

    #[error("`{value}` is not a decimal without exponent notation")]
    InvalidDecimal { value: String },

    #[error("`{value}` is not a valid ISO calendar date")]
    InvalidDate { value: String },

    #[error("`{value}` is not a valid ISO local time")]
    InvalidTime { value: String },

    #[error("`{value}` is not a valid RFC 3339 date-time")]
    InvalidDateTime { value: String },

    #[error("`{value}` is not a fixed duration in `[-]PT<seconds>[.<fraction>]S` form")]
    InvalidDuration { value: String },

    #[error("latitude `{value}` is outside [-90, 90]")]
    InvalidLatitude { value: String },

    #[error("longitude `{value}` is outside [-180, 180]")]
    InvalidLongitude { value: String },

    #[error("`{value}` is not a canonical <decimal> <ISO-4217-code> money value")]
    InvalidMoney { value: String },
    #[error("`{value}` is not a canonical percentage value")]
    InvalidPercentage { value: String },
    #[error("`{value}` is not a supported built-in unit quantity")]
    InvalidQuantity { value: String },
    #[error("`{value}` is not a valid latitude,longitude coordinate pair")]
    InvalidCoordinate { value: String },
    #[error("`{value}` is not a valid `{value_type}` value")]
    InvalidRefinementText {
        value_type: CanonicalType,
        value: String,
    },

    #[error("`{value}` is not an offset-free ISO local date-time")]
    InvalidLocalDateTime { value: String },
    #[error(
        "`{value}` must be `<RFC3339-with-offset> <IANA-zone>` with an offset valid in that zone"
    )]
    InvalidZonedDateTime { value: String },
    #[error("`{value}` is not an ISO 8601 calendar Y/M/D duration")]
    InvalidCalendarDuration { value: String },
    #[error("calendar duration application overflows the target month or date")]
    CalendarDateOverflow,

    #[error("set values cannot contain duplicate elements")]
    DuplicateSetElement,
    #[error("map values cannot contain duplicate keys")]
    DuplicateMapKey,
    #[error("reference record ID must not be empty")]
    InvalidReferenceRecordId,
    #[error("coordinate radius must be non-negative")]
    InvalidRadius,

    #[error("operation `{operation}` is not defined for type `{value_type}`")]
    UnsupportedOperation {
        operation: &'static str,
        value_type: CanonicalType,
    },
}
