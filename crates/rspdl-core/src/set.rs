use std::collections::BTreeSet;

use serde::Serialize;

use crate::domain::{Backend, Domain, SymbolicSupport, ensure_type};
use crate::error::ModelError;
use crate::types::CanonicalType;
use crate::value::CanonicalValue;

#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(tag = "kind", content = "definition", rename_all = "snake_case")]
enum SetKind {
    Domain(Domain),
    Literal(BTreeSet<CanonicalValue>),
    Union(Vec<SetExpression>),
    Intersection(Vec<SetExpression>),
    Difference {
        minuend: Box<SetExpression>,
        subtrahend: Box<SetExpression>,
    },
}
pub enum SetExpressionView<'a> {
    Domain(&'a Domain),
    Literal(&'a BTreeSet<CanonicalValue>),
    Union(&'a [SetExpression]),
    Intersection(&'a [SetExpression]),
    Difference(&'a SetExpression, &'a SetExpression),
}

/// A normalized, typed set expression.
///
/// Union and intersection operands are flattened, sorted, and deduplicated at
/// construction time. Difference preserves operand order.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
pub struct SetExpression {
    value_type: CanonicalType,
    expression: SetKind,
}

impl SetExpression {
    pub fn domain(domain: Domain) -> Self {
        Self {
            value_type: domain.value_type().clone(),
            expression: SetKind::Domain(domain),
        }
    }

    pub fn literal(
        value_type: CanonicalType,
        values: impl IntoIterator<Item = CanonicalValue>,
    ) -> Result<Self, ModelError> {
        let mut normalized = BTreeSet::new();
        for value in values {
            ensure_type("set literal", &value_type, value.value_type())?;
            normalized.insert(value);
        }
        Ok(Self {
            value_type,
            expression: SetKind::Literal(normalized),
        })
    }

    pub fn union(operands: impl IntoIterator<Item = SetExpression>) -> Result<Self, ModelError> {
        Self::commutative("union", operands.into_iter().collect(), true)
    }

    pub fn intersection(
        operands: impl IntoIterator<Item = SetExpression>,
    ) -> Result<Self, ModelError> {
        Self::commutative("intersection", operands.into_iter().collect(), false)
    }

    fn commutative(
        operation: &'static str,
        operands: Vec<SetExpression>,
        union: bool,
    ) -> Result<Self, ModelError> {
        let Some(first) = operands.first() else {
            return Err(ModelError::EmptyOperands { operation });
        };
        let value_type = first.value_type.clone();
        let mut normalized = Vec::new();

        for operand in operands {
            ensure_type(operation, &value_type, &operand.value_type)?;
            match (union, operand.expression) {
                (true, SetKind::Union(nested)) | (false, SetKind::Intersection(nested)) => {
                    normalized.extend(nested);
                }
                (_, expression) => normalized.push(Self {
                    value_type: value_type.clone(),
                    expression,
                }),
            }
        }

        normalized.sort();
        normalized.dedup();
        if normalized.len() == 1 {
            return Ok(normalized
                .pop()
                .expect("a one-element normalized set must contain its element"));
        }

        Ok(Self {
            value_type,
            expression: if union {
                SetKind::Union(normalized)
            } else {
                SetKind::Intersection(normalized)
            },
        })
    }

    pub fn difference(
        minuend: SetExpression,
        subtrahend: SetExpression,
    ) -> Result<Self, ModelError> {
        ensure_type("difference", &minuend.value_type, &subtrahend.value_type)?;
        Ok(Self {
            value_type: minuend.value_type.clone(),
            expression: SetKind::Difference {
                minuend: Box::new(minuend),
                subtrahend: Box::new(subtrahend),
            },
        })
    }

    pub fn value_type(&self) -> &CanonicalType {
        &self.value_type
    }
    pub fn view(&self) -> SetExpressionView<'_> {
        match &self.expression {
            SetKind::Domain(x) => SetExpressionView::Domain(x),
            SetKind::Literal(x) => SetExpressionView::Literal(x),
            SetKind::Union(x) => SetExpressionView::Union(x),
            SetKind::Intersection(x) => SetExpressionView::Intersection(x),
            SetKind::Difference {
                minuend,
                subtrahend,
            } => SetExpressionView::Difference(minuend, subtrahend),
        }
    }

    pub fn contains(&self, value: &CanonicalValue) -> Result<bool, ModelError> {
        ensure_type("set membership", &self.value_type, value.value_type())?;
        match &self.expression {
            SetKind::Domain(domain) => domain.contains(value),
            SetKind::Literal(values) => Ok(values.contains(value)),
            SetKind::Union(operands) => {
                for operand in operands {
                    if operand.contains(value)? {
                        return Ok(true);
                    }
                }
                Ok(false)
            }
            SetKind::Intersection(operands) => {
                for operand in operands {
                    if !operand.contains(value)? {
                        return Ok(false);
                    }
                }
                Ok(true)
            }
            SetKind::Difference {
                minuend,
                subtrahend,
            } => Ok(minuend.contains(value)? && !subtrahend.contains(value)?),
        }
    }

    pub fn symbolic_support(&self, backend: Backend) -> SymbolicSupport {
        match &self.expression {
            SetKind::Domain(domain) => domain.symbolic_support(backend),
            SetKind::Literal(_) => SymbolicSupport::Exact,
            SetKind::Union(operands) | SetKind::Intersection(operands) => operands
                .iter()
                .fold(SymbolicSupport::Exact, |support, operand| {
                    support.combine(operand.symbolic_support(backend))
                }),
            SetKind::Difference {
                minuend,
                subtrahend,
            } => minuend
                .symbolic_support(backend)
                .combine(subtrahend.symbolic_support(backend)),
        }
    }
}
