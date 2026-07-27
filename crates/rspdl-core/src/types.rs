use std::collections::BTreeSet;
use std::fmt;

use serde::Serialize;

use crate::error::ModelError;

/// A stable, locale-independent identifier used by the semantic model.
///
/// IDs consist of dot-separated segments. Each segment starts with a lowercase
/// ASCII letter and continues with lowercase letters, digits, or underscores.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(transparent)]
pub struct CanonicalId(String);

impl CanonicalId {
    pub fn new(value: impl Into<String>) -> Result<Self, ModelError> {
        let value = value.into();
        if is_canonical_id(&value) {
            Ok(Self(value))
        } else {
            Err(ModelError::InvalidCanonicalId { value })
        }
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for CanonicalId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

fn is_canonical_id(value: &str) -> bool {
    !value.is_empty()
        && value.split('.').all(|segment| {
            let mut chars = segment.chars();
            chars.next().is_some_and(|first| first.is_ascii_lowercase())
                && chars.all(|character| {
                    character.is_ascii_lowercase() || character.is_ascii_digit() || character == '_'
                })
        })
}

/// A closed enumeration whose identity and variants are canonical.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
pub struct EnumType {
    id: CanonicalId,
    variants: BTreeSet<CanonicalId>,
}

impl EnumType {
    pub fn new(
        id: CanonicalId,
        variants: impl IntoIterator<Item = CanonicalId>,
    ) -> Result<Self, ModelError> {
        let variants = variants.into_iter().collect::<BTreeSet<_>>();
        if variants.is_empty() {
            return Err(ModelError::EmptyEnum { type_id: id });
        }
        Ok(Self { id, variants })
    }

    pub fn id(&self) -> &CanonicalId {
        &self.id
    }

    pub fn variants(&self) -> &BTreeSet<CanonicalId> {
        &self.variants
    }

    pub fn contains(&self, variant: &CanonicalId) -> bool {
        self.variants.contains(variant)
    }
}

/// Refinements built into the canonical language specification.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum BuiltinRefinement {
    Prime,
}

impl BuiltinRefinement {
    pub const fn name(self) -> &'static str {
        match self {
            Self::Prime => "prime",
        }
    }

    pub const fn required_base(self) -> CanonicalType {
        match self {
            Self::Prime => CanonicalType::Integer,
        }
    }
}

/// A type narrowed by a decidable built-in predicate.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
pub struct RefinementType {
    base: Box<CanonicalType>,
    predicate: BuiltinRefinement,
}

impl RefinementType {
    pub fn new(base: CanonicalType, predicate: BuiltinRefinement) -> Result<Self, ModelError> {
        let expected = predicate.required_base();
        if base != expected {
            return Err(ModelError::InvalidRefinementBase {
                refinement: predicate.name(),
                expected,
                actual: base,
            });
        }
        Ok(Self {
            base: Box::new(base),
            predicate,
        })
    }

    pub fn base(&self) -> &CanonicalType {
        &self.base
    }

    pub const fn predicate(&self) -> BuiltinRefinement {
        self.predicate
    }
}

/// The only types that may appear in canonical values and logical expressions.
///
/// There is intentionally no `Any`, inferred, or unknown variant.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(tag = "kind", content = "definition", rename_all = "snake_case")]
pub enum CanonicalType {
    Boolean,
    Integer,
    String,
    Enum(EnumType),
    Refinement(RefinementType),
}

impl CanonicalType {
    pub fn prime() -> Self {
        Self::Refinement(
            RefinementType::new(Self::Integer, BuiltinRefinement::Prime)
                .expect("the built-in prime refinement must accept integer"),
        )
    }
}

impl fmt::Display for CanonicalType {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Boolean => formatter.write_str("boolean"),
            Self::Integer => formatter.write_str("integer"),
            Self::String => formatter.write_str("string"),
            Self::Enum(enum_type) => write!(formatter, "enum({})", enum_type.id()),
            Self::Refinement(refinement) => {
                write!(
                    formatter,
                    "{}({})",
                    refinement.predicate().name(),
                    refinement.base()
                )
            }
        }
    }
}
