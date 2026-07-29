use std::fmt;
use std::str::FromStr;

use num_bigint::BigInt;
use num_prime::nt_funcs::is_prime64;
use num_traits::{Signed, ToPrimitive};
use serde::{Serialize, Serializer};

use crate::error::ModelError;
use crate::types::{BuiltinRefinement, CanonicalId, CanonicalType, EnumType};

/// An unbounded mathematical integer with canonical decimal serialization.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct CanonicalInteger(BigInt);

impl CanonicalInteger {
    pub fn parse(value: impl Into<String>) -> Result<Self, ModelError> {
        let value = value.into();
        let integer = BigInt::from_str(&value).map_err(|_| ModelError::InvalidInteger {
            value: value.clone(),
        })?;

        if integer.to_string() != value {
            return Err(ModelError::InvalidInteger { value });
        }
        Ok(Self(integer))
    }

    pub fn as_bigint(&self) -> &BigInt {
        &self.0
    }
}

impl From<i64> for CanonicalInteger {
    fn from(value: i64) -> Self {
        Self(BigInt::from(value))
    }
}

impl fmt::Display for CanonicalInteger {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}", self.0)
    }
}

impl Serialize for CanonicalInteger {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&self.0.to_string())
    }
}

#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(tag = "kind", content = "value", rename_all = "snake_case")]
enum ValueRepresentation {
    Boolean(bool),
    Integer(CanonicalInteger),
    String(String),
    EnumVariant(CanonicalId),
}

/// A value paired with its fully resolved canonical type.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
pub struct CanonicalValue {
    value_type: CanonicalType,
    representation: ValueRepresentation,
}

impl CanonicalValue {
    pub fn boolean(value: bool) -> Self {
        Self {
            value_type: CanonicalType::Boolean,
            representation: ValueRepresentation::Boolean(value),
        }
    }

    pub fn integer(value: impl Into<CanonicalInteger>) -> Self {
        Self {
            value_type: CanonicalType::Integer,
            representation: ValueRepresentation::Integer(value.into()),
        }
    }

    pub fn integer_from_decimal(value: impl Into<String>) -> Result<Self, ModelError> {
        Ok(Self::integer(CanonicalInteger::parse(value)?))
    }

    pub fn string(value: impl Into<String>) -> Self {
        Self {
            value_type: CanonicalType::String,
            representation: ValueRepresentation::String(value.into()),
        }
    }

    pub fn enum_variant(enum_type: EnumType, variant: CanonicalId) -> Result<Self, ModelError> {
        if !enum_type.contains(&variant) {
            return Err(ModelError::UnknownEnumVariant {
                type_id: enum_type.id().clone(),
                variant,
            });
        }
        Ok(Self {
            value_type: CanonicalType::Enum(enum_type),
            representation: ValueRepresentation::EnumVariant(variant),
        })
    }

    pub fn prime(value: impl Into<CanonicalInteger>) -> Result<Self, ModelError> {
        let value = value.into();
        if !is_prime(value.as_bigint())? {
            return Err(ModelError::InvalidRefinedValue {
                value_type: CanonicalType::prime(),
                value: value.to_string(),
            });
        }
        Ok(Self {
            value_type: CanonicalType::prime(),
            representation: ValueRepresentation::Integer(value),
        })
    }

    pub fn prime_from_decimal(value: impl Into<String>) -> Result<Self, ModelError> {
        Self::prime(CanonicalInteger::parse(value)?)
    }

    pub fn value_type(&self) -> &CanonicalType {
        &self.value_type
    }

    pub fn as_integer(&self) -> Option<&CanonicalInteger> {
        match &self.representation {
            ValueRepresentation::Integer(value) => Some(value),
            _ => None,
        }
    }
    pub fn as_boolean(&self) -> Option<bool> {
        match self.representation {
            ValueRepresentation::Boolean(value) => Some(value),
            _ => None,
        }
    }
    pub fn as_string(&self) -> Option<&str> {
        match &self.representation {
            ValueRepresentation::String(value) => Some(value),
            _ => None,
        }
    }
    pub fn as_enum_variant(&self) -> Option<&CanonicalId> {
        match &self.representation {
            ValueRepresentation::EnumVariant(value) => Some(value),
            _ => None,
        }
    }

    pub fn satisfies_refinement(&self, refinement: BuiltinRefinement) -> Result<bool, ModelError> {
        match (refinement, self.as_integer()) {
            (BuiltinRefinement::Prime, Some(value)) => is_prime(value.as_bigint()),
            (BuiltinRefinement::Prime, None) => Ok(false),
        }
    }
}

fn is_prime(value: &BigInt) -> Result<bool, ModelError> {
    if value.is_negative() || value < &BigInt::from(2_u8) {
        return Ok(false);
    }
    let value = value
        .to_u64()
        .ok_or_else(|| ModelError::RefinementMagnitudeExceeded {
            refinement: "prime",
            value: value.to_string(),
            maximum: "18446744073709551615",
        })?;
    Ok(is_prime64(value))
}

#[cfg(test)]
mod tests {
    use super::is_prime;
    use num_bigint::BigInt;

    #[test]
    fn primality_is_exact_for_boundaries_and_a_larger_value() {
        for composite in [-7_i64, 0, 1, 4, 9, 104_730] {
            assert!(!is_prime(&BigInt::from(composite)).unwrap(), "{composite}");
        }
        for prime in [2_i64, 3, 5, 104_729] {
            assert!(is_prime(&BigInt::from(prime)).unwrap(), "{prime}");
        }
    }

    #[test]
    fn primality_rejects_values_above_the_exact_supported_range() {
        let too_large = BigInt::from(u64::MAX) + 1_u8;
        assert!(matches!(
            is_prime(&too_large),
            Err(crate::ModelError::RefinementMagnitudeExceeded { .. })
        ));
    }
}
