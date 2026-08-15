use std::fmt;

use serde::Serialize;

/// Stable identity for one source supplied to the compiler.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(transparent)]
pub struct SourceId(String);

impl SourceId {
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    pub fn inline() -> Self {
        Self("<inline>".into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for SourceId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

/// A UTF-8 byte range in one source file.
#[derive(Clone, Copy, Debug, Default, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
pub struct TextRange {
    pub start: usize,
    pub end: usize,
}

impl TextRange {
    pub fn join(self, other: Self) -> Self {
        Self {
            start: self.start.min(other.start),
            end: self.end.max(other.end),
        }
    }
}
