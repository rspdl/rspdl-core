use serde::Serialize;

use crate::domain::ensure_type;
use crate::error::ModelError;
use crate::set::SetExpression;
use crate::types::{CanonicalId, CanonicalType};
use crate::value::CanonicalValue;

#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
pub struct Variable {
    id: CanonicalId,
    value_type: CanonicalType,
}

impl Variable {
    pub fn new(id: CanonicalId, value_type: CanonicalType) -> Self {
        Self { id, value_type }
    }

    pub fn id(&self) -> &CanonicalId {
        &self.id
    }

    pub fn value_type(&self) -> &CanonicalType {
        &self.value_type
    }
}

#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(tag = "kind", content = "definition", rename_all = "snake_case")]
pub enum Term {
    Variable(Variable),
    Constant(CanonicalValue),
}

impl Term {
    pub fn value_type(&self) -> &CanonicalType {
        match self {
            Self::Variable(variable) => variable.value_type(),
            Self::Constant(value) => value.value_type(),
        }
    }
}

#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
pub struct PredicateSignature {
    id: CanonicalId,
    parameter_types: Vec<CanonicalType>,
}

impl PredicateSignature {
    pub fn new(id: CanonicalId, parameter_types: Vec<CanonicalType>) -> Self {
        Self {
            id,
            parameter_types,
        }
    }

    pub fn id(&self) -> &CanonicalId {
        &self.id
    }

    pub fn parameter_types(&self) -> &[CanonicalType] {
        &self.parameter_types
    }
}

#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(tag = "kind", content = "definition", rename_all = "snake_case")]
enum AtomKind {
    Equal {
        left: Term,
        right: Term,
    },
    MemberOf {
        term: Term,
        set: SetExpression,
    },
    Predicate {
        signature: PredicateSignature,
        arguments: Vec<Term>,
    },
    IntegerComparison {
        operator: ComparisonOperator,
        left: Term,
        right: Term,
    },
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ComparisonOperator {
    Lt,
    Le,
    Gt,
    Ge,
}

pub enum AtomView<'a> {
    Equal(&'a Term, &'a Term),
    MemberOf(&'a Term, &'a SetExpression),
    Predicate(&'a PredicateSignature, &'a [Term]),
    IntegerComparison(ComparisonOperator, &'a Term, &'a Term),
}

/// A type-checked atomic logical proposition.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
pub struct Atom {
    atom: AtomKind,
}

impl Atom {
    pub fn equal(mut left: Term, mut right: Term) -> Result<Self, ModelError> {
        ensure_type("equality", left.value_type(), right.value_type())?;
        if right < left {
            std::mem::swap(&mut left, &mut right);
        }
        Ok(Self {
            atom: AtomKind::Equal { left, right },
        })
    }

    pub fn member_of(term: Term, set: SetExpression) -> Result<Self, ModelError> {
        ensure_type("membership atom", term.value_type(), set.value_type())?;
        Ok(Self {
            atom: AtomKind::MemberOf { term, set },
        })
    }

    pub fn predicate(
        signature: PredicateSignature,
        arguments: Vec<Term>,
    ) -> Result<Self, ModelError> {
        if signature.parameter_types.len() != arguments.len() {
            return Err(ModelError::ArityMismatch {
                predicate: signature.id.clone(),
                expected: signature.parameter_types.len(),
                actual: arguments.len(),
            });
        }

        for (expected, argument) in signature.parameter_types.iter().zip(&arguments) {
            ensure_type("predicate argument", expected, argument.value_type())?;
        }

        Ok(Self {
            atom: AtomKind::Predicate {
                signature,
                arguments,
            },
        })
    }
    pub fn integer_comparison(
        operator: ComparisonOperator,
        left: Term,
        right: Term,
    ) -> Result<Self, ModelError> {
        ensure_type(
            "integer comparison",
            &CanonicalType::Integer,
            left.value_type(),
        )?;
        ensure_type(
            "integer comparison",
            &CanonicalType::Integer,
            right.value_type(),
        )?;
        Ok(Self {
            atom: AtomKind::IntegerComparison {
                operator,
                left,
                right,
            },
        })
    }
    pub fn view(&self) -> AtomView<'_> {
        match &self.atom {
            AtomKind::Equal { left, right } => AtomView::Equal(left, right),
            AtomKind::MemberOf { term, set } => AtomView::MemberOf(term, set),
            AtomKind::Predicate {
                signature,
                arguments,
            } => AtomView::Predicate(signature, arguments),
            AtomKind::IntegerComparison {
                operator,
                left,
                right,
            } => AtomView::IntegerComparison(*operator, left, right),
        }
    }
}

#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(tag = "kind", content = "definition", rename_all = "snake_case")]
enum BooleanKind {
    Literal(bool),
    Atom(Atom),
    And(Vec<BooleanExpression>),
    Or(Vec<BooleanExpression>),
    Not(Box<BooleanExpression>),
}
pub enum BooleanExpressionView<'a> {
    Literal(bool),
    Atom(&'a Atom),
    And(&'a [BooleanExpression]),
    Or(&'a [BooleanExpression]),
    Not(&'a BooleanExpression),
}

/// A normalized boolean expression shared by rules, policies, and constraints.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
pub struct BooleanExpression {
    expression: BooleanKind,
}

impl BooleanExpression {
    pub const fn literal(value: bool) -> Self {
        Self {
            expression: BooleanKind::Literal(value),
        }
    }

    pub fn atom(atom: Atom) -> Self {
        Self {
            expression: BooleanKind::Atom(atom),
        }
    }

    pub fn and(operands: impl IntoIterator<Item = Self>) -> Self {
        Self::commutative(operands.into_iter().collect(), true)
    }

    pub fn or(operands: impl IntoIterator<Item = Self>) -> Self {
        Self::commutative(operands.into_iter().collect(), false)
    }

    fn commutative(operands: Vec<Self>, and: bool) -> Self {
        let mut normalized = Vec::new();
        for operand in operands {
            match (and, operand.expression) {
                (true, BooleanKind::Literal(false)) | (false, BooleanKind::Literal(true)) => {
                    return Self::literal(!and);
                }
                (true, BooleanKind::Literal(true)) | (false, BooleanKind::Literal(false)) => {}
                (true, BooleanKind::And(nested)) | (false, BooleanKind::Or(nested)) => {
                    normalized.extend(nested);
                }
                (_, expression) => normalized.push(Self { expression }),
            }
        }

        normalized.sort();
        normalized.dedup();
        match normalized.len() {
            0 => Self::literal(and),
            1 => normalized
                .pop()
                .expect("a one-element boolean expression must contain its element"),
            _ => Self {
                expression: if and {
                    BooleanKind::And(normalized)
                } else {
                    BooleanKind::Or(normalized)
                },
            },
        }
    }

    pub fn negate(expression: Self) -> Self {
        match expression.expression {
            BooleanKind::Literal(value) => Self::literal(!value),
            BooleanKind::Not(inner) => *inner,
            expression => Self {
                expression: BooleanKind::Not(Box::new(Self { expression })),
            },
        }
    }
    pub fn view(&self) -> BooleanExpressionView<'_> {
        match &self.expression {
            BooleanKind::Literal(v) => BooleanExpressionView::Literal(*v),
            BooleanKind::Atom(a) => BooleanExpressionView::Atom(a),
            BooleanKind::And(xs) => BooleanExpressionView::And(xs),
            BooleanKind::Or(xs) => BooleanExpressionView::Or(xs),
            BooleanKind::Not(x) => BooleanExpressionView::Not(x),
        }
    }
}
