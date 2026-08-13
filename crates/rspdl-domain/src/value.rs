use std::cmp::Ordering;
use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::str::FromStr;

use chrono::{DateTime, Datelike, NaiveDate, NaiveDateTime, NaiveTime, Offset, Timelike, Utc};
use chrono_tz::Tz;
use num_bigint::BigInt;
use num_prime::nt_funcs::is_prime64;
use num_traits::{Signed, ToPrimitive, Zero};
use serde::{Serialize, Serializer};

use crate::error::ModelError;
use crate::types::{
    BuiltinRefinement, CanonicalId, CanonicalType, CurrencyCode, EnumType, QuantityDimension,
};

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

/// An arbitrary-precision finite decimal with one normalized representation.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct CanonicalDecimal {
    coefficient: BigInt,
    scale: u32,
}

impl CanonicalDecimal {
    pub fn parse(value: impl Into<String>) -> Result<Self, ModelError> {
        let value = value.into();
        let (negative, body) = match value.strip_prefix('-') {
            Some(body) => (true, body),
            None => (false, value.as_str()),
        };
        if body.is_empty() || body.starts_with('+') {
            return Err(ModelError::InvalidDecimal { value });
        }
        let mut parts = body.split('.');
        let integer = parts.next().expect("split always returns one part");
        let fraction = parts.next();
        if parts.next().is_some()
            || integer.is_empty()
            || !integer.bytes().all(|byte| byte.is_ascii_digit())
            || (integer.len() > 1 && integer.starts_with('0'))
            || fraction.is_some_and(|fraction| {
                fraction.is_empty() || !fraction.bytes().all(|byte| byte.is_ascii_digit())
            })
        {
            return Err(ModelError::InvalidDecimal { value });
        }
        let fraction = fraction.unwrap_or_default();
        let scale = u32::try_from(fraction.len()).map_err(|_| ModelError::InvalidDecimal {
            value: value.clone(),
        })?;
        let mut coefficient = BigInt::from_str(&format!("{integer}{fraction}")).map_err(|_| {
            ModelError::InvalidDecimal {
                value: value.clone(),
            }
        })?;
        if negative {
            coefficient = -coefficient;
        }
        let mut decimal = Self { coefficient, scale };
        decimal.normalize();
        Ok(decimal)
    }

    fn normalize(&mut self) {
        if self.coefficient.is_zero() {
            self.scale = 0;
            return;
        }
        while self.scale > 0 && (&self.coefficient % 10_u8).is_zero() {
            self.coefficient /= 10_u8;
            self.scale -= 1;
        }
    }

    pub fn coefficient(&self) -> &BigInt {
        &self.coefficient
    }

    pub const fn scale(&self) -> u32 {
        self.scale
    }
}

impl Ord for CanonicalDecimal {
    fn cmp(&self, other: &Self) -> Ordering {
        match self.scale.cmp(&other.scale) {
            Ordering::Equal => self.coefficient.cmp(&other.coefficient),
            Ordering::Less => {
                let factor = BigInt::from(10_u8).pow(other.scale - self.scale);
                (&self.coefficient * factor).cmp(&other.coefficient)
            }
            Ordering::Greater => {
                let factor = BigInt::from(10_u8).pow(self.scale - other.scale);
                self.coefficient.cmp(&(&other.coefficient * factor))
            }
        }
    }
}

impl PartialOrd for CanonicalDecimal {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl fmt::Display for CanonicalDecimal {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let negative = self.coefficient.is_negative();
        let digits = self.coefficient.abs().to_string();
        if negative {
            formatter.write_str("-")?;
        }
        if self.scale == 0 {
            return formatter.write_str(&digits);
        }
        let scale = self.scale as usize;
        if digits.len() <= scale {
            formatter.write_str("0.")?;
            for _ in 0..(scale - digits.len()) {
                formatter.write_str("0")?;
            }
            formatter.write_str(&digits)
        } else {
            let split = digits.len() - scale;
            write!(formatter, "{}.{}", &digits[..split], &digits[split..])
        }
    }
}

impl Serialize for CanonicalDecimal {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct CanonicalDate(NaiveDate);

impl CanonicalDate {
    pub fn parse(value: impl Into<String>) -> Result<Self, ModelError> {
        let value = value.into();
        let bytes = value.as_bytes();
        if bytes.len() != 10
            || bytes[4] != b'-'
            || bytes[7] != b'-'
            || bytes
                .iter()
                .enumerate()
                .any(|(index, byte)| index != 4 && index != 7 && !byte.is_ascii_digit())
        {
            return Err(ModelError::InvalidDate { value });
        }
        let date =
            NaiveDate::parse_from_str(&value, "%Y-%m-%d").map_err(|_| ModelError::InvalidDate {
                value: value.clone(),
            })?;
        if !(1..=9999).contains(&date.year()) {
            return Err(ModelError::InvalidDate { value });
        }
        Ok(Self(date))
    }
}

impl fmt::Display for CanonicalDate {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}", self.0.format("%Y-%m-%d"))
    }
}

impl Serialize for CanonicalDate {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct CanonicalTime(NaiveTime);

impl CanonicalTime {
    pub fn parse(value: impl Into<String>) -> Result<Self, ModelError> {
        let value = value.into();
        let bytes = value.as_bytes();
        let shape_is_valid = bytes.len() >= 8
            && bytes[2] == b':'
            && bytes[5] == b':'
            && bytes[..8]
                .iter()
                .enumerate()
                .all(|(index, byte)| matches!(index, 2 | 5) || byte.is_ascii_digit())
            && (bytes.len() == 8
                || (bytes[8] == b'.'
                    && (10..=18).contains(&bytes.len())
                    && bytes[9..].iter().all(u8::is_ascii_digit)));
        if !shape_is_valid {
            return Err(ModelError::InvalidTime { value });
        }
        let time = NaiveTime::parse_from_str(&value, "%H:%M:%S%.f").map_err(|_| {
            ModelError::InvalidTime {
                value: value.clone(),
            }
        })?;
        if time.nanosecond() >= 1_000_000_000 {
            return Err(ModelError::InvalidTime { value });
        }
        Ok(Self(time))
    }
}

impl fmt::Display for CanonicalTime {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}", self.0.format("%H:%M:%S"))?;
        write_fraction(formatter, self.0.nanosecond())
    }
}

impl Serialize for CanonicalTime {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct CanonicalDateTime(DateTime<Utc>);

impl CanonicalDateTime {
    pub fn parse(value: impl Into<String>) -> Result<Self, ModelError> {
        let value = value.into();
        let parsed = DateTime::parse_from_rfc3339(&value)
            .map_err(|_| ModelError::InvalidDateTime {
                value: value.clone(),
            })?
            .with_timezone(&Utc);
        if !(1..=9999).contains(&parsed.year()) || parsed.nanosecond() >= 1_000_000_000 {
            return Err(ModelError::InvalidDateTime { value });
        }
        Ok(Self(parsed))
    }
}

impl fmt::Display for CanonicalDateTime {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}", self.0.format("%Y-%m-%dT%H:%M:%S"))?;
        write_fraction(formatter, self.0.nanosecond())?;
        formatter.write_str("Z")
    }
}

impl Serialize for CanonicalDateTime {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

fn write_fraction(formatter: &mut fmt::Formatter<'_>, nanosecond: u32) -> fmt::Result {
    if nanosecond == 0 {
        return Ok(());
    }
    let fraction = format!("{nanosecond:09}");
    formatter.write_str(".")?;
    formatter.write_str(fraction.trim_end_matches('0'))
}

#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct CanonicalDuration(BigInt);

impl CanonicalDuration {
    pub fn parse(value: impl Into<String>) -> Result<Self, ModelError> {
        let value = value.into();
        let (negative, body) = match value.strip_prefix('-') {
            Some(body) => (true, body),
            None => (false, value.as_str()),
        };
        let seconds = body
            .strip_prefix("PT")
            .and_then(|body| body.strip_suffix('S'))
            .filter(|body| !body.is_empty())
            .ok_or_else(|| ModelError::InvalidDuration {
                value: value.clone(),
            })?;
        let decimal =
            CanonicalDecimal::parse(seconds).map_err(|_| ModelError::InvalidDuration {
                value: value.clone(),
            })?;
        if decimal.coefficient().is_negative() || decimal.scale() > 9 {
            return Err(ModelError::InvalidDuration { value });
        }
        let mut nanoseconds =
            decimal.coefficient().clone() * BigInt::from(10_u8).pow(9 - decimal.scale());
        if negative {
            nanoseconds = -nanoseconds;
        }
        Ok(Self(nanoseconds))
    }
}

impl fmt::Display for CanonicalDuration {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let negative = self.0.is_negative();
        let magnitude = self.0.abs();
        let billion = BigInt::from(1_000_000_000_u64);
        let seconds = &magnitude / &billion;
        let nanoseconds = (&magnitude % &billion)
            .to_u32()
            .expect("nanosecond remainder fits u32");
        if negative {
            formatter.write_str("-")?;
        }
        write!(formatter, "PT{seconds}")?;
        write_fraction(formatter, nanoseconds)?;
        formatter.write_str("S")
    }
}

impl Serialize for CanonicalDuration {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(transparent)]
pub struct CanonicalLatitude(CanonicalDecimal);

impl CanonicalLatitude {
    pub fn parse(value: impl Into<String>) -> Result<Self, ModelError> {
        let value = value.into();
        let decimal = CanonicalDecimal::parse(&value)?;
        let minimum = CanonicalDecimal::parse("-90").expect("constant is valid");
        let maximum = CanonicalDecimal::parse("90").expect("constant is valid");
        if decimal < minimum || decimal > maximum {
            return Err(ModelError::InvalidLatitude { value });
        }
        Ok(Self(decimal))
    }
}

impl fmt::Display for CanonicalLatitude {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(formatter)
    }
}

#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(transparent)]
pub struct CanonicalLongitude(CanonicalDecimal);

impl CanonicalLongitude {
    pub fn parse(value: impl Into<String>) -> Result<Self, ModelError> {
        let value = value.into();
        let decimal = CanonicalDecimal::parse(&value)?;
        let minimum = CanonicalDecimal::parse("-180").expect("constant is valid");
        let maximum = CanonicalDecimal::parse("180").expect("constant is valid");
        if decimal < minimum || decimal > maximum {
            return Err(ModelError::InvalidLongitude { value });
        }
        Ok(Self(decimal))
    }
}

impl fmt::Display for CanonicalLongitude {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(formatter)
    }
}

#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(tag = "kind", content = "value", rename_all = "snake_case")]
enum ValueRepresentation {
    Boolean(bool),
    Integer(CanonicalInteger),
    Decimal(CanonicalDecimal),
    String(String),
    Date(CanonicalDate),
    Time(CanonicalTime),
    DateTime(CanonicalDateTime),
    Duration(CanonicalDuration),
    Latitude(CanonicalLatitude),
    Longitude(CanonicalLongitude),
    EnumVariant(CanonicalId),
    /// Canonical text plus an optional exact order key for parameterized scalar
    /// values.  The type tag above remains the source of semantic identity.
    Extended {
        canonical: String,
        ordered: Option<CanonicalDecimal>,
    },
    List(Vec<CanonicalValue>),
    Set(BTreeSet<CanonicalValue>),
    Map(BTreeMap<CanonicalValue, CanonicalValue>),
    Reference {
        record_id: String,
    },
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

    pub fn decimal_from_str(value: impl Into<String>) -> Result<Self, ModelError> {
        Ok(Self {
            value_type: CanonicalType::Decimal,
            representation: ValueRepresentation::Decimal(CanonicalDecimal::parse(value)?),
        })
    }

    pub fn string(value: impl Into<String>) -> Self {
        Self {
            value_type: CanonicalType::String,
            representation: ValueRepresentation::String(value.into()),
        }
    }

    pub fn date_from_iso(value: impl Into<String>) -> Result<Self, ModelError> {
        Ok(Self {
            value_type: CanonicalType::Date,
            representation: ValueRepresentation::Date(CanonicalDate::parse(value)?),
        })
    }

    pub fn time_from_iso(value: impl Into<String>) -> Result<Self, ModelError> {
        Ok(Self {
            value_type: CanonicalType::Time,
            representation: ValueRepresentation::Time(CanonicalTime::parse(value)?),
        })
    }

    pub fn date_time_from_rfc3339(value: impl Into<String>) -> Result<Self, ModelError> {
        Ok(Self {
            value_type: CanonicalType::DateTime,
            representation: ValueRepresentation::DateTime(CanonicalDateTime::parse(value)?),
        })
    }

    pub fn duration_from_iso(value: impl Into<String>) -> Result<Self, ModelError> {
        Ok(Self {
            value_type: CanonicalType::Duration,
            representation: ValueRepresentation::Duration(CanonicalDuration::parse(value)?),
        })
    }

    pub fn latitude_from_decimal(value: impl Into<String>) -> Result<Self, ModelError> {
        Ok(Self {
            value_type: CanonicalType::Latitude,
            representation: ValueRepresentation::Latitude(CanonicalLatitude::parse(value)?),
        })
    }

    pub fn longitude_from_decimal(value: impl Into<String>) -> Result<Self, ModelError> {
        Ok(Self {
            value_type: CanonicalType::Longitude,
            representation: ValueRepresentation::Longitude(CanonicalLongitude::parse(value)?),
        })
    }

    pub fn money_from_str(value: impl Into<String>) -> Result<Self, ModelError> {
        let value = value.into();
        let (amount, currency) =
            value
                .rsplit_once(' ')
                .ok_or_else(|| ModelError::InvalidMoney {
                    value: value.clone(),
                })?;
        if amount.is_empty() || currency.contains(' ') {
            return Err(ModelError::InvalidMoney { value });
        }
        let amount = CanonicalDecimal::parse(amount).map_err(|_| ModelError::InvalidMoney {
            value: value.clone(),
        })?;
        let currency =
            CurrencyCode::new(currency.to_owned()).map_err(|_| ModelError::InvalidMoney {
                value: value.clone(),
            })?;
        Ok(Self {
            value_type: CanonicalType::Money(currency.clone()),
            representation: ValueRepresentation::Extended {
                canonical: format!("{amount} {currency}"),
                ordered: Some(amount),
            },
        })
    }

    pub fn percentage_from_str(value: impl Into<String>) -> Result<Self, ModelError> {
        let value = value.into();
        let number = value
            .strip_suffix('%')
            .ok_or_else(|| ModelError::InvalidPercentage {
                value: value.clone(),
            })?;
        let decimal =
            CanonicalDecimal::parse(number).map_err(|_| ModelError::InvalidPercentage {
                value: value.clone(),
            })?;
        Ok(Self {
            value_type: CanonicalType::Percentage,
            representation: ValueRepresentation::Extended {
                canonical: format!("{decimal}%"),
                ordered: Some(decimal),
            },
        })
    }

    pub fn percentage_from_ratio_str(value: impl Into<String>) -> Result<Self, ModelError> {
        let ratio = CanonicalDecimal::parse(value.into())?;
        let hundred = CanonicalDecimal::parse("100").expect("constant");
        let percentage = multiply_decimal(&ratio, &hundred);
        Self::percentage_from_str(format!("{percentage}%"))
    }

    pub fn as_ratio(&self) -> Result<CanonicalDecimal, ModelError> {
        if self.value_type != CanonicalType::Percentage {
            return Err(ModelError::UnsupportedOperation {
                operation: "percentage ratio conversion",
                value_type: self.value_type.clone(),
            });
        }
        let ValueRepresentation::Extended {
            ordered: Some(value),
            ..
        } = &self.representation
        else {
            unreachable!("percentage representation")
        };
        let hundred = CanonicalDecimal::parse("0.01").expect("constant");
        Ok(multiply_decimal(value, &hundred))
    }

    pub fn quantity_from_str(value: impl Into<String>) -> Result<Self, ModelError> {
        let value = value.into();
        let (number, unit) = value
            .rsplit_once(' ')
            .ok_or_else(|| ModelError::InvalidQuantity {
                value: value.clone(),
            })?;
        let number = CanonicalDecimal::parse(number).map_err(|_| ModelError::InvalidQuantity {
            value: value.clone(),
        })?;
        let (dimension, factor, canonical_unit) = match unit {
            "kg" => (QuantityDimension::Mass, "1", "kg"),
            "g" => (QuantityDimension::Mass, "0.001", "kg"),
            "m" => (QuantityDimension::Length, "1", "m"),
            "km" => (QuantityDimension::Length, "1000", "m"),
            "s" => (QuantityDimension::Duration, "1", "s"),
            "ms" => (QuantityDimension::Duration, "0.001", "s"),
            _ => return Err(ModelError::InvalidQuantity { value }),
        };
        let factor = CanonicalDecimal::parse(factor).expect("built-in conversion is valid");
        let base = multiply_decimal(&number, &factor);
        Ok(Self {
            value_type: CanonicalType::Quantity(dimension),
            representation: ValueRepresentation::Extended {
                canonical: format!("{base} {canonical_unit}"),
                ordered: Some(base),
            },
        })
    }

    pub fn coordinate_from_str(value: impl Into<String>) -> Result<Self, ModelError> {
        let value = value.into();
        let (latitude, longitude) =
            value
                .split_once(',')
                .ok_or_else(|| ModelError::InvalidCoordinate {
                    value: value.clone(),
                })?;
        let latitude = CanonicalLatitude::parse(latitude.trim()).map_err(|_| {
            ModelError::InvalidCoordinate {
                value: value.clone(),
            }
        })?;
        let longitude = CanonicalLongitude::parse(longitude.trim()).map_err(|_| {
            ModelError::InvalidCoordinate {
                value: value.clone(),
            }
        })?;
        Ok(Self {
            value_type: CanonicalType::Coordinate,
            representation: ValueRepresentation::Extended {
                canonical: format!("{latitude},{longitude}"),
                ordered: None,
            },
        })
    }

    pub fn local_date_time_from_iso(value: impl Into<String>) -> Result<Self, ModelError> {
        let value = value.into();
        let local =
            NaiveDateTime::parse_from_str(&value, "%Y-%m-%dT%H:%M:%S%.f").map_err(|_| {
                ModelError::InvalidLocalDateTime {
                    value: value.clone(),
                }
            })?;
        if !(1..=9999).contains(&local.year())
            || local
                .format("%Y-%m-%dT%H:%M:%S%.f")
                .to_string()
                .trim_end_matches('0')
                .trim_end_matches('.')
                .is_empty()
        {
            return Err(ModelError::InvalidLocalDateTime { value });
        }
        let canonical = format_local_date_time(local);
        Ok(Self {
            value_type: CanonicalType::LocalDateTime,
            representation: ValueRepresentation::Extended {
                canonical,
                ordered: None,
            },
        })
    }

    pub fn zoned_date_time_from_str(value: impl Into<String>) -> Result<Self, ModelError> {
        let value = value.into();
        let (instant, zone) =
            value
                .rsplit_once(' ')
                .ok_or_else(|| ModelError::InvalidZonedDateTime {
                    value: value.clone(),
                })?;
        let parsed = DateTime::parse_from_rfc3339(instant).map_err(|_| {
            ModelError::InvalidZonedDateTime {
                value: value.clone(),
            }
        })?;
        let zone = zone
            .parse::<Tz>()
            .map_err(|_| ModelError::InvalidZonedDateTime {
                value: value.clone(),
            })?;
        let in_zone = parsed.with_timezone(&zone);
        if in_zone.offset().fix() != *parsed.offset() {
            return Err(ModelError::InvalidZonedDateTime { value });
        }
        let local = in_zone.naive_local();
        let canonical = format!(
            "{}{} {}",
            format_local_date_time(local),
            in_zone.format("%:z"),
            zone.name()
        );
        Ok(Self {
            value_type: CanonicalType::ZonedDateTime,
            representation: ValueRepresentation::Extended {
                canonical,
                ordered: None,
            },
        })
    }

    pub fn calendar_duration_from_iso(value: impl Into<String>) -> Result<Self, ModelError> {
        let value = value.into();
        let (years, months, days) =
            parse_calendar_duration(&value).ok_or_else(|| ModelError::InvalidCalendarDuration {
                value: value.clone(),
            })?;
        let years = years
            .checked_add(months / 12)
            .ok_or(ModelError::CalendarDateOverflow)?;
        let months = months % 12;
        Ok(Self {
            value_type: CanonicalType::CalendarDuration,
            representation: ValueRepresentation::Extended {
                canonical: format_calendar_duration(years, months, days),
                ordered: None,
            },
        })
    }

    /// Applies a calendar Y/M/D duration with the fixed `RejectOverflow`
    /// policy: e.g. 2026-01-31 + P1M is an error, never a guessed last day.
    pub fn apply_calendar_duration_to_date(&self, duration: &Self) -> Result<Self, ModelError> {
        if self.value_type != CanonicalType::Date
            || duration.value_type != CanonicalType::CalendarDuration
        {
            return Err(ModelError::UnsupportedOperation {
                operation: "calendar duration application",
                value_type: self.value_type.clone(),
            });
        }
        let date = CanonicalDate::parse(self.canonical_text())?;
        let (years, months, days) = parse_calendar_duration(&duration.canonical_text())
            .expect("calendar value is constructed valid");
        let applied = apply_calendar_date(date.0, years, months, days)?;
        Self::date_from_iso(applied.format("%Y-%m-%d").to_string())
    }

    /// Deterministic Haversine distance using a fixed mean Earth radius
    /// (6,371,008.8 m), rounded to 9 decimal metres; it is approximate.
    pub fn distance_to(&self, other: &Self) -> Result<Self, ModelError> {
        let (lat1, lon1) = coordinate_parts(self)?;
        let (lat2, lon2) = coordinate_parts(other)?;
        let radians = std::f64::consts::PI / 180.0;
        let dlat = (lat2 - lat1) * radians;
        let dlon = (lon2 - lon1) * radians;
        let a = libm::sin(dlat / 2.0).powi(2)
            + libm::cos(lat1 * radians) * libm::cos(lat2 * radians) * libm::sin(dlon / 2.0).powi(2);
        let a = a.clamp(0.0, 1.0);
        let metres = 6_371_008.8_f64 * 2.0 * libm::atan2(libm::sqrt(a), libm::sqrt(1.0 - a));
        Self::quantity_from_str(format!("{metres:.9} m"))
    }

    pub fn is_within_radius(&self, center: &Self, radius: &Self) -> Result<bool, ModelError> {
        if radius.value_type != CanonicalType::Quantity(QuantityDimension::Length) {
            return Err(ModelError::TypeMismatch {
                context: "coordinate radius",
                expected: CanonicalType::Quantity(QuantityDimension::Length),
                actual: radius.value_type.clone(),
            });
        }
        let zero = CanonicalValue::quantity_from_str("0 m").expect("zero length is valid");
        if radius.compare_ordered(&zero)? == Ordering::Less {
            return Err(ModelError::InvalidRadius);
        }
        Ok(self.distance_to(center)?.compare_ordered(radius)? != Ordering::Greater)
    }

    pub fn cidr_contains(&self, address: &Self) -> Result<bool, ModelError> {
        if self.value_type != CanonicalType::Cidr || address.value_type != CanonicalType::IpAddress
        {
            return Err(ModelError::TypeMismatch {
                context: "CIDR contains",
                expected: CanonicalType::IpAddress,
                actual: address.value_type.clone(),
            });
        }
        let canonical = self.canonical_text();
        let (network, prefix) = canonical.split_once('/').expect("canonical CIDR");
        let network = network.parse::<std::net::IpAddr>().expect("canonical IP");
        let address = address
            .canonical_text()
            .parse::<std::net::IpAddr>()
            .expect("canonical IP");
        Ok(ip_in_cidr(
            network,
            prefix.parse().expect("canonical prefix"),
            address,
        ))
    }

    pub fn refinement_from_str(
        value_type: CanonicalType,
        value: impl Into<String>,
    ) -> Result<Self, ModelError> {
        let value = value.into();
        let canonical = match value_type {
            CanonicalType::Uuid => canonical_uuid(&value),
            CanonicalType::Email => valid_email(&value).then(|| value.clone()),
            CanonicalType::Url => valid_url(&value).then(|| value.clone()),
            CanonicalType::PhoneNumber => valid_phone(&value).then(|| value.clone()),
            CanonicalType::IpAddress => value
                .parse::<std::net::IpAddr>()
                .ok()
                .map(|ip| ip.to_string()),
            CanonicalType::Cidr => canonical_cidr(&value),
            CanonicalType::CountryCode => valid_country_code(&value).then(|| value.clone()),
            CanonicalType::LanguageCode => valid_language_code(&value).then(|| value.clone()),
            CanonicalType::CurrencyCode => CurrencyCode::new(value.clone())
                .ok()
                .map(|code| code.to_string()),
            _ => None,
        }
        .ok_or_else(|| ModelError::InvalidRefinementText {
            value_type: value_type.clone(),
            value: value.clone(),
        })?;
        Ok(Self {
            value_type,
            representation: ValueRepresentation::Extended {
                canonical,
                ordered: None,
            },
        })
    }

    pub fn list(element_type: CanonicalType, values: Vec<Self>) -> Result<Self, ModelError> {
        for value in &values {
            ensure_value_type("list element", &element_type, value)?;
        }
        Ok(Self {
            value_type: CanonicalType::List(Box::new(element_type)),
            representation: ValueRepresentation::List(values),
        })
    }

    pub fn set(
        element_type: CanonicalType,
        input: impl IntoIterator<Item = Self>,
    ) -> Result<Self, ModelError> {
        let mut values = BTreeSet::new();
        for value in input {
            ensure_value_type("set element", &element_type, &value)?;
            if !values.insert(value) {
                return Err(ModelError::DuplicateSetElement);
            }
        }
        Ok(Self {
            value_type: CanonicalType::Set(Box::new(element_type)),
            representation: ValueRepresentation::Set(values),
        })
    }

    pub fn map(
        key_type: CanonicalType,
        value_type: CanonicalType,
        input: impl IntoIterator<Item = (Self, Self)>,
    ) -> Result<Self, ModelError> {
        let mut values = BTreeMap::new();
        for (key, value) in input {
            ensure_value_type("map key", &key_type, &key)?;
            ensure_value_type("map value", &value_type, &value)?;
            if values.insert(key, value).is_some() {
                return Err(ModelError::DuplicateMapKey);
            }
        }
        Ok(Self {
            value_type: CanonicalType::map(key_type, value_type)?,
            representation: ValueRepresentation::Map(values),
        })
    }

    pub fn reference(
        target_model: CanonicalId,
        record_id: impl Into<String>,
    ) -> Result<Self, ModelError> {
        let record_id = record_id.into();
        if record_id.is_empty() {
            return Err(ModelError::InvalidReferenceRecordId);
        }
        Ok(Self {
            value_type: CanonicalType::Reference(target_model),
            representation: ValueRepresentation::Reference { record_id },
        })
    }

    pub fn list_contains(&self, value: &Self) -> Result<bool, ModelError> {
        match (&self.value_type, &self.representation) {
            (CanonicalType::List(element), ValueRepresentation::List(values)) => {
                ensure_value_type("list contains", element, value)?;
                Ok(values.contains(value))
            }
            _ => Err(ModelError::UnsupportedOperation {
                operation: "list contains",
                value_type: self.value_type.clone(),
            }),
        }
    }
    pub fn list_count(&self) -> Result<usize, ModelError> {
        match &self.representation {
            ValueRepresentation::List(values) => Ok(values.len()),
            _ => Err(ModelError::UnsupportedOperation {
                operation: "list count",
                value_type: self.value_type.clone(),
            }),
        }
    }
    pub fn set_contains(&self, value: &Self) -> Result<bool, ModelError> {
        match (&self.value_type, &self.representation) {
            (CanonicalType::Set(element), ValueRepresentation::Set(values)) => {
                ensure_value_type("set contains", element, value)?;
                Ok(values.contains(value))
            }
            _ => Err(ModelError::UnsupportedOperation {
                operation: "set contains",
                value_type: self.value_type.clone(),
            }),
        }
    }
    pub fn set_union(&self, other: &Self) -> Result<Self, ModelError> {
        self.combine_sets(other, true)
    }
    pub fn set_intersection(&self, other: &Self) -> Result<Self, ModelError> {
        self.combine_sets(other, false)
    }
    fn combine_sets(&self, other: &Self, union: bool) -> Result<Self, ModelError> {
        if self.value_type != other.value_type {
            return Err(ModelError::TypeMismatch {
                context: "set operation",
                expected: self.value_type.clone(),
                actual: other.value_type.clone(),
            });
        }
        let (
            CanonicalType::Set(element),
            ValueRepresentation::Set(left),
            ValueRepresentation::Set(right),
        ) = (
            &self.value_type,
            &self.representation,
            &other.representation,
        )
        else {
            return Err(ModelError::UnsupportedOperation {
                operation: "set operation",
                value_type: self.value_type.clone(),
            });
        };
        let values = if union {
            left.union(right).cloned().collect::<Vec<_>>()
        } else {
            left.intersection(right).cloned().collect::<Vec<_>>()
        };
        Self::set((**element).clone(), values)
    }
    pub fn map_lookup(&self, key: &Self) -> Result<Option<&Self>, ModelError> {
        match (&self.value_type, &self.representation) {
            (CanonicalType::Map { key: key_type, .. }, ValueRepresentation::Map(values)) => {
                ensure_value_type("map lookup", key_type, key)?;
                Ok(values.get(key))
            }
            _ => Err(ModelError::UnsupportedOperation {
                operation: "map lookup",
                value_type: self.value_type.clone(),
            }),
        }
    }

    /// Exact add/subtract for same-currency money, percentages, and compatible
    /// built-in quantities. Other arithmetic remains explicitly unsupported.
    pub fn add(&self, other: &Self) -> Result<Self, ModelError> {
        self.combine_ordered(other, "addition", |left, right| left + right)
    }

    pub fn subtract(&self, other: &Self) -> Result<Self, ModelError> {
        self.combine_ordered(other, "subtraction", |left, right| left - right)
    }

    fn combine_ordered(
        &self,
        other: &Self,
        operation: &'static str,
        combine: impl FnOnce(BigInt, BigInt) -> BigInt,
    ) -> Result<Self, ModelError> {
        if self.value_type != other.value_type {
            return Err(ModelError::TypeMismatch {
                context: operation,
                expected: self.value_type.clone(),
                actual: other.value_type.clone(),
            });
        }
        let (
            ValueRepresentation::Extended {
                canonical: left_text,
                ordered: Some(left),
            },
            ValueRepresentation::Extended {
                ordered: Some(right),
                ..
            },
        ) = (&self.representation, &other.representation)
        else {
            return Err(ModelError::UnsupportedOperation {
                operation,
                value_type: self.value_type.clone(),
            });
        };
        if !matches!(
            self.value_type,
            CanonicalType::Money(_) | CanonicalType::Percentage | CanonicalType::Quantity(_)
        ) {
            return Err(ModelError::UnsupportedOperation {
                operation,
                value_type: self.value_type.clone(),
            });
        }
        let scale = left.scale().max(right.scale());
        let left_factor = BigInt::from(10_u8).pow(scale - left.scale());
        let right_factor = BigInt::from(10_u8).pow(scale - right.scale());
        let mut result = CanonicalDecimal {
            coefficient: combine(
                left.coefficient() * left_factor,
                right.coefficient() * right_factor,
            ),
            scale,
        };
        result.normalize();
        let suffix = left_text
            .split_once(' ')
            .map(|(_, suffix)| format!(" {suffix}"))
            .unwrap_or_else(|| {
                if matches!(self.value_type, CanonicalType::Percentage) {
                    "%".into()
                } else {
                    String::new()
                }
            });
        Ok(Self {
            value_type: self.value_type.clone(),
            representation: ValueRepresentation::Extended {
                canonical: format!("{result}{suffix}"),
                ordered: Some(result),
            },
        })
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

    pub fn canonical_text(&self) -> String {
        match &self.representation {
            ValueRepresentation::Boolean(value) => value.to_string(),
            ValueRepresentation::Integer(value) => value.to_string(),
            ValueRepresentation::Decimal(value) => value.to_string(),
            ValueRepresentation::String(value) => value.clone(),
            ValueRepresentation::Date(value) => value.to_string(),
            ValueRepresentation::Time(value) => value.to_string(),
            ValueRepresentation::DateTime(value) => value.to_string(),
            ValueRepresentation::Duration(value) => value.to_string(),
            ValueRepresentation::Latitude(value) => value.to_string(),
            ValueRepresentation::Longitude(value) => value.to_string(),
            ValueRepresentation::EnumVariant(value) => value.to_string(),
            ValueRepresentation::Extended { canonical, .. } => canonical.clone(),
            ValueRepresentation::List(values) => {
                serde_json::to_string(values).expect("canonical values serialize")
            }
            ValueRepresentation::Set(values) => {
                serde_json::to_string(values).expect("canonical values serialize")
            }
            ValueRepresentation::Map(values) => {
                serde_json::to_string(&values.iter().collect::<Vec<_>>())
                    .expect("canonical values serialize")
            }
            ValueRepresentation::Reference { record_id } => record_id.clone(),
        }
    }

    pub fn as_integer(&self) -> Option<&CanonicalInteger> {
        match &self.representation {
            ValueRepresentation::Integer(value) => Some(value),
            _ => None,
        }
    }
    pub fn as_decimal(&self) -> Option<&CanonicalDecimal> {
        match &self.representation {
            ValueRepresentation::Decimal(value) => Some(value),
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
    pub fn as_date(&self) -> Option<&CanonicalDate> {
        match &self.representation {
            ValueRepresentation::Date(value) => Some(value),
            _ => None,
        }
    }
    pub fn as_time(&self) -> Option<&CanonicalTime> {
        match &self.representation {
            ValueRepresentation::Time(value) => Some(value),
            _ => None,
        }
    }
    pub fn as_date_time(&self) -> Option<&CanonicalDateTime> {
        match &self.representation {
            ValueRepresentation::DateTime(value) => Some(value),
            _ => None,
        }
    }
    pub fn as_duration(&self) -> Option<&CanonicalDuration> {
        match &self.representation {
            ValueRepresentation::Duration(value) => Some(value),
            _ => None,
        }
    }
    pub fn as_latitude(&self) -> Option<&CanonicalLatitude> {
        match &self.representation {
            ValueRepresentation::Latitude(value) => Some(value),
            _ => None,
        }
    }
    pub fn as_longitude(&self) -> Option<&CanonicalLongitude> {
        match &self.representation {
            ValueRepresentation::Longitude(value) => Some(value),
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

    /// Compares values using their domain order rather than representation tag order.
    pub fn compare_ordered(&self, other: &Self) -> Result<Ordering, ModelError> {
        if self.value_type != other.value_type {
            return Err(ModelError::TypeMismatch {
                context: "ordered comparison",
                expected: self.value_type.clone(),
                actual: other.value_type.clone(),
            });
        }
        if !self.value_type.is_ordered() {
            return Err(ModelError::UnsupportedOperation {
                operation: "ordered comparison",
                value_type: self.value_type.clone(),
            });
        }
        let ordering = match (&self.representation, &other.representation) {
            (ValueRepresentation::Integer(left), ValueRepresentation::Integer(right)) => {
                left.cmp(right)
            }
            (ValueRepresentation::Decimal(left), ValueRepresentation::Decimal(right)) => {
                left.cmp(right)
            }
            (ValueRepresentation::Date(left), ValueRepresentation::Date(right)) => left.cmp(right),
            (ValueRepresentation::Time(left), ValueRepresentation::Time(right)) => left.cmp(right),
            (ValueRepresentation::DateTime(left), ValueRepresentation::DateTime(right)) => {
                left.cmp(right)
            }
            (ValueRepresentation::Duration(left), ValueRepresentation::Duration(right)) => {
                left.cmp(right)
            }
            (ValueRepresentation::Latitude(left), ValueRepresentation::Latitude(right)) => {
                left.cmp(right)
            }
            (ValueRepresentation::Longitude(left), ValueRepresentation::Longitude(right)) => {
                left.cmp(right)
            }
            (
                ValueRepresentation::Extended {
                    canonical: left, ..
                },
                ValueRepresentation::Extended {
                    canonical: right, ..
                },
            ) if self.value_type == CanonicalType::LocalDateTime => left.cmp(right),
            (
                ValueRepresentation::Extended {
                    canonical: left, ..
                },
                ValueRepresentation::Extended {
                    canonical: right, ..
                },
            ) if self.value_type == CanonicalType::ZonedDateTime => {
                let left = left
                    .split_once(' ')
                    .and_then(|(instant, _)| DateTime::parse_from_rfc3339(instant).ok())
                    .expect("canonical zoned date time");
                let right = right
                    .split_once(' ')
                    .and_then(|(instant, _)| DateTime::parse_from_rfc3339(instant).ok())
                    .expect("canonical zoned date time");
                left.cmp(&right)
            }
            (
                ValueRepresentation::Extended {
                    ordered: Some(left),
                    ..
                },
                ValueRepresentation::Extended {
                    ordered: Some(right),
                    ..
                },
            ) => left.cmp(right),
            _ => unreachable!("canonical value type and representation stay aligned"),
        };
        Ok(ordering)
    }
}

fn ensure_value_type(
    context: &'static str,
    expected: &CanonicalType,
    actual: &CanonicalValue,
) -> Result<(), ModelError> {
    if actual.value_type == *expected {
        Ok(())
    } else {
        Err(ModelError::TypeMismatch {
            context,
            expected: expected.clone(),
            actual: actual.value_type.clone(),
        })
    }
}

fn multiply_decimal(left: &CanonicalDecimal, right: &CanonicalDecimal) -> CanonicalDecimal {
    let mut result = CanonicalDecimal {
        coefficient: left.coefficient() * right.coefficient(),
        scale: left.scale() + right.scale(),
    };
    result.normalize();
    result
}

fn format_local_date_time(value: NaiveDateTime) -> String {
    let mut output = value.format("%Y-%m-%dT%H:%M:%S").to_string();
    if value.nanosecond() != 0 {
        output.push('.');
        output.push_str(format!("{:09}", value.nanosecond()).trim_end_matches('0'));
    }
    output
}

fn parse_calendar_duration(value: &str) -> Option<(i32, i32, i32)> {
    let body = value.strip_prefix('P')?;
    if body.is_empty() || body.contains('T') {
        return None;
    }
    let mut rest = body;
    let mut part = |suffix: char| -> Option<i32> {
        if !rest.contains(suffix) {
            return Some(0);
        }
        let (digits, after) = rest.split_once(suffix)?;
        if digits.is_empty() || !digits.bytes().all(|byte| byte.is_ascii_digit()) {
            return None;
        }
        rest = after;
        digits.parse().ok()
    };
    let years = part('Y')?;
    let months = part('M')?;
    let days = part('D')?;
    if !rest.is_empty() {
        return None;
    }
    Some((years, months, days))
}

fn format_calendar_duration(years: i32, months: i32, days: i32) -> String {
    if years == 0 && months == 0 && days == 0 {
        return "P0D".into();
    }
    let mut value = String::from("P");
    if years != 0 {
        value.push_str(&format!("{years}Y"));
    }
    if months != 0 {
        value.push_str(&format!("{months}M"));
    }
    if days != 0 {
        value.push_str(&format!("{days}D"));
    }
    value
}

fn apply_calendar_date(
    date: NaiveDate,
    years: i32,
    months: i32,
    days: i32,
) -> Result<NaiveDate, ModelError> {
    let month_index = (date.month0() as i32)
        .checked_add(
            years
                .checked_mul(12)
                .ok_or(ModelError::CalendarDateOverflow)?,
        )
        .and_then(|value| value.checked_add(months))
        .ok_or(ModelError::CalendarDateOverflow)?;
    let year = date
        .year()
        .checked_add(month_index.div_euclid(12))
        .ok_or(ModelError::CalendarDateOverflow)?;
    let month = month_index.rem_euclid(12) as u32 + 1;
    let shifted =
        NaiveDate::from_ymd_opt(year, month, date.day()).ok_or(ModelError::CalendarDateOverflow)?;
    shifted
        .checked_add_signed(chrono::Duration::days(days as i64))
        .ok_or(ModelError::CalendarDateOverflow)
}

fn coordinate_parts(value: &CanonicalValue) -> Result<(f64, f64), ModelError> {
    if value.value_type != CanonicalType::Coordinate {
        return Err(ModelError::UnsupportedOperation {
            operation: "coordinate distance",
            value_type: value.value_type.clone(),
        });
    }
    let canonical = value.canonical_text();
    let (latitude, longitude) = canonical
        .split_once(',')
        .expect("coordinate constructor canonicalizes");
    Ok((
        latitude.parse().expect("canonical decimal parses as f64"),
        longitude.parse().expect("canonical decimal parses as f64"),
    ))
}

fn canonical_uuid(value: &str) -> Option<String> {
    let bytes = value.as_bytes();
    (bytes.len() == 36
        && [8, 13, 18, 23]
            .into_iter()
            .all(|index| bytes[index] == b'-')
        && bytes
            .iter()
            .enumerate()
            .all(|(index, byte)| [8, 13, 18, 23].contains(&index) || byte.is_ascii_hexdigit()))
    .then(|| value.to_ascii_lowercase())
}
fn valid_email(value: &str) -> bool {
    let Some((local, domain)) = value.rsplit_once('@') else {
        return false;
    };
    !local.is_empty()
        && domain.contains('.')
        && !domain.starts_with('.')
        && !domain.ends_with('.')
        && !value.chars().any(char::is_whitespace)
}
fn valid_url(value: &str) -> bool {
    let Some((scheme, rest)) = value.split_once("://") else {
        return false;
    };
    matches!(scheme, "http" | "https")
        && !rest.is_empty()
        && !rest.starts_with('/')
        && !rest.chars().any(char::is_whitespace)
}
fn valid_phone(value: &str) -> bool {
    value.starts_with('+')
        && (8..=16).contains(&value.len())
        && value[1..].bytes().all(|byte| byte.is_ascii_digit())
}
fn canonical_cidr(value: &str) -> Option<String> {
    let (ip, prefix) = value.split_once('/')?;
    let ip = ip.parse::<std::net::IpAddr>().ok()?;
    let prefix = prefix.parse::<u8>().ok()?;
    match ip {
        std::net::IpAddr::V4(ip) if prefix <= 32 => {
            let mask = u32::MAX.checked_shl(32 - prefix as u32).unwrap_or(0);
            Some(format!(
                "{}/{}",
                std::net::Ipv4Addr::from(u32::from(ip) & mask),
                prefix
            ))
        }
        std::net::IpAddr::V6(ip) if prefix <= 128 => {
            let mask = u128::MAX.checked_shl(128 - prefix as u32).unwrap_or(0);
            Some(format!(
                "{}/{}",
                std::net::Ipv6Addr::from(u128::from(ip) & mask),
                prefix
            ))
        }
        _ => None,
    }
}

fn ip_in_cidr(network: std::net::IpAddr, prefix: u8, address: std::net::IpAddr) -> bool {
    match (network, address) {
        (std::net::IpAddr::V4(network), std::net::IpAddr::V4(address)) => {
            let mask = u32::MAX.checked_shl(32 - prefix as u32).unwrap_or(0);
            u32::from(network) & mask == u32::from(address) & mask
        }
        (std::net::IpAddr::V6(network), std::net::IpAddr::V6(address)) => {
            let mask = u128::MAX.checked_shl(128 - prefix as u32).unwrap_or(0);
            u128::from(network) & mask == u128::from(address) & mask
        }
        _ => false,
    }
}
fn valid_country_code(value: &str) -> bool {
    value.len() == 2 && value.bytes().all(|byte| byte.is_ascii_uppercase())
}
fn valid_language_code(value: &str) -> bool {
    value.len() == 2 && value.bytes().all(|byte| byte.is_ascii_lowercase())
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
