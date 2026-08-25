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
        variant_ids: impl IntoIterator<Item = CanonicalId>,
    ) -> Result<Self, ModelError> {
        let mut variants = BTreeSet::new();
        for variant in variant_ids {
            if !variants.insert(variant.clone()) {
                return Err(ModelError::DuplicateEnumVariant {
                    type_id: id,
                    variant,
                });
            }
        }
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

/// An ISO 4217-shaped currency identifier.  RSPDL deliberately does not use a
/// live currency registry: validation is deterministic and network-free.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(transparent)]
pub struct CurrencyCode(String);

impl CurrencyCode {
    pub fn new(value: impl Into<String>) -> Result<Self, ModelError> {
        let value = value.into();
        if value.len() == 3 && value.bytes().all(|byte| byte.is_ascii_uppercase()) {
            Ok(Self(value))
        } else {
            Err(ModelError::InvalidCurrencyCode { value })
        }
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for CurrencyCode {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

/// Physical dimensions supported by the closed built-in unit vocabulary.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum QuantityDimension {
    Mass,
    Length,
    Duration,
}

impl QuantityDimension {
    /// 선언된 단위 이름에서 차원을 얻는다. 값을 하나 지어내 파싱해 보는 대신 이 표를
    /// 직접 본다 — 그래야 진단이 소스에 없는 `1 없는단위` 대신 선언된 단위를 가리킨다.
    pub fn from_unit(unit: &str) -> Result<Self, ModelError> {
        crate::value::unit_conversion(unit)
            .map(|(dimension, _, _)| dimension)
            .ok_or_else(|| ModelError::InvalidUnit {
                unit: unit.to_owned(),
            })
    }
}

impl fmt::Display for QuantityDimension {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::Mass => "mass",
            Self::Length => "length",
            Self::Duration => "duration",
        })
    }
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
    Decimal,
    String,
    Date,
    Time,
    DateTime,
    Duration,
    Latitude,
    Longitude,
    Money(CurrencyCode),
    Percentage,
    Quantity(QuantityDimension),
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
    List(Box<CanonicalType>),
    Set(Box<CanonicalType>),
    Map {
        key: Box<CanonicalType>,
        value: Box<CanonicalType>,
    },
    Reference(CanonicalId),
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

    /// Whether values of this type have a semantic total order.
    pub const fn is_ordered(&self) -> bool {
        matches!(
            self,
            Self::Integer
                | Self::Decimal
                | Self::Date
                | Self::Time
                | Self::DateTime
                | Self::Duration
                | Self::Latitude
                | Self::Longitude
                | Self::Money(_)
                | Self::Percentage
                | Self::Quantity(_)
                | Self::LocalDateTime
                | Self::ZonedDateTime
        )
    }

    /// Map keys deliberately exclude collections and opaque structured values
    /// so equality and canonical JSON object ordering remain deterministic.
    pub fn map(key: CanonicalType, value: CanonicalType) -> Result<Self, ModelError> {
        if !matches!(
            key,
            Self::String
                | Self::Uuid
                | Self::Email
                | Self::Url
                | Self::PhoneNumber
                | Self::IpAddress
                | Self::Cidr
                | Self::CountryCode
                | Self::LanguageCode
                | Self::CurrencyCode
        ) {
            return Err(ModelError::UnsupportedMapKeyType { value_type: key });
        }
        Ok(Self::Map {
            key: Box::new(key),
            value: Box::new(value),
        })
    }
}

impl fmt::Display for CanonicalType {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Boolean => formatter.write_str("boolean"),
            Self::Integer => formatter.write_str("integer"),
            Self::Decimal => formatter.write_str("decimal"),
            Self::String => formatter.write_str("string"),
            Self::Date => formatter.write_str("date"),
            Self::Time => formatter.write_str("time"),
            Self::DateTime => formatter.write_str("date_time"),
            Self::Duration => formatter.write_str("duration"),
            Self::Latitude => formatter.write_str("latitude"),
            Self::Longitude => formatter.write_str("longitude"),
            Self::Money(currency) => write!(formatter, "money({currency})"),
            Self::Percentage => formatter.write_str("percentage"),
            Self::Quantity(dimension) => write!(formatter, "quantity({dimension})"),
            Self::Coordinate => formatter.write_str("coordinate"),
            Self::LocalDateTime => formatter.write_str("local_date_time"),
            Self::ZonedDateTime => formatter.write_str("zoned_date_time"),
            Self::CalendarDuration => formatter.write_str("calendar_duration"),
            Self::Uuid => formatter.write_str("uuid"),
            Self::Email => formatter.write_str("email"),
            Self::Url => formatter.write_str("url"),
            Self::PhoneNumber => formatter.write_str("phone_number"),
            Self::IpAddress => formatter.write_str("ip_address"),
            Self::Cidr => formatter.write_str("cidr"),
            Self::CountryCode => formatter.write_str("country_code"),
            Self::LanguageCode => formatter.write_str("language_code"),
            Self::CurrencyCode => formatter.write_str("currency_code"),
            Self::List(element) => write!(formatter, "list({element})"),
            Self::Set(element) => write!(formatter, "set({element})"),
            Self::Map { key, value } => write!(formatter, "map({key}, {value})"),
            Self::Reference(model) => write!(formatter, "reference({model})"),
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
