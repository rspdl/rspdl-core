use std::collections::BTreeSet;

use serde::Serialize;

use crate::error::ModelError;
use crate::types::CanonicalType;
use crate::value::CanonicalValue;

/// A target backend asking whether it can reason about a domain symbolically.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Backend {
    Smt,
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Cardinality {
    Finite(usize),
    CountablyInfinite,
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum EnumerationSupport {
    Exact,
    Unsupported,
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum GroundMembership {
    Exact,
}

/// The degree to which a backend can preserve a domain's meaning.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SymbolicSupport {
    Exact,
    RequiresFiniteGrounding,
    Unsupported,
}

impl SymbolicSupport {
    pub(crate) fn combine(self, other: Self) -> Self {
        match (self, other) {
            (Self::Unsupported, _) | (_, Self::Unsupported) => Self::Unsupported,
            (Self::RequiresFiniteGrounding, _) | (_, Self::RequiresFiniteGrounding) => {
                Self::RequiresFiniteGrounding
            }
            (Self::Exact, Self::Exact) => Self::Exact,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
pub struct DomainCapabilities {
    pub cardinality: Cardinality,
    pub enumeration: EnumerationSupport,
    pub ground_membership: GroundMembership,
}

/// Built-in infinite domains. None of these are ever eagerly enumerated.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum InfiniteDomain {
    Integers,
    Strings,
    Primes,
}

#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(tag = "kind", content = "definition", rename_all = "snake_case")]
enum DomainKind {
    Finite(BTreeSet<CanonicalValue>),
    Infinite(InfiniteDomain),
}

/// A normalized set of possible values for one canonical type.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
pub struct Domain {
    value_type: CanonicalType,
    domain: DomainKind,
}

impl Domain {
    pub fn finite(
        value_type: CanonicalType,
        values: impl IntoIterator<Item = CanonicalValue>,
    ) -> Result<Self, ModelError> {
        let mut normalized = BTreeSet::new();
        for value in values {
            ensure_type("finite domain", &value_type, value.value_type())?;
            normalized.insert(value);
        }
        Ok(Self {
            value_type,
            domain: DomainKind::Finite(normalized),
        })
    }

    pub fn integers() -> Self {
        Self::infinite(CanonicalType::Integer, InfiniteDomain::Integers)
    }

    pub fn strings() -> Self {
        Self::infinite(CanonicalType::String, InfiniteDomain::Strings)
    }

    pub fn primes() -> Self {
        Self::infinite(CanonicalType::prime(), InfiniteDomain::Primes)
    }

    fn infinite(value_type: CanonicalType, domain: InfiniteDomain) -> Self {
        Self {
            value_type,
            domain: DomainKind::Infinite(domain),
        }
    }

    pub fn value_type(&self) -> &CanonicalType {
        &self.value_type
    }

    pub fn finite_values(&self) -> Option<&BTreeSet<CanonicalValue>> {
        match &self.domain {
            DomainKind::Finite(values) => Some(values),
            DomainKind::Infinite(_) => None,
        }
    }

    pub fn infinite_kind(&self) -> Option<InfiniteDomain> {
        match &self.domain {
            DomainKind::Finite(_) => None,
            DomainKind::Infinite(domain) => Some(*domain),
        }
    }

    pub fn contains(&self, value: &CanonicalValue) -> Result<bool, ModelError> {
        ensure_type("domain membership", &self.value_type, value.value_type())?;
        match &self.domain {
            DomainKind::Finite(values) => Ok(values.contains(value)),
            DomainKind::Infinite(InfiniteDomain::Integers | InfiniteDomain::Strings) => Ok(true),
            DomainKind::Infinite(InfiniteDomain::Primes) => {
                value.satisfies_refinement(crate::types::BuiltinRefinement::Prime)
            }
        }
    }

    pub fn capabilities(&self) -> DomainCapabilities {
        match &self.domain {
            DomainKind::Finite(values) => DomainCapabilities {
                cardinality: Cardinality::Finite(values.len()),
                enumeration: EnumerationSupport::Exact,
                ground_membership: GroundMembership::Exact,
            },
            DomainKind::Infinite(_) => DomainCapabilities {
                cardinality: Cardinality::CountablyInfinite,
                enumeration: EnumerationSupport::Unsupported,
                ground_membership: GroundMembership::Exact,
            },
        }
    }

    pub fn symbolic_support(&self, backend: Backend) -> SymbolicSupport {
        match (&self.domain, backend) {
            (DomainKind::Finite(_), _) => SymbolicSupport::Exact,
            (
                DomainKind::Infinite(InfiniteDomain::Integers | InfiniteDomain::Strings),
                Backend::Smt,
            ) => SymbolicSupport::Exact,
            (DomainKind::Infinite(InfiniteDomain::Primes), Backend::Smt) => {
                SymbolicSupport::Unsupported
            }
        }
    }
}

pub(crate) fn ensure_type(
    context: &'static str,
    expected: &CanonicalType,
    actual: &CanonicalType,
) -> Result<(), ModelError> {
    if expected == actual {
        Ok(())
    } else {
        Err(ModelError::TypeMismatch {
            context,
            expected: expected.clone(),
            actual: actual.clone(),
        })
    }
}
