//! Z3-backed typed constraint solving; Z3 values do not escape this crate.
use rspdl_core::{
    AtomView, BooleanExpression, BooleanExpressionView, CanonicalId, CanonicalModel, CanonicalType,
    CanonicalValue, ConstraintProblem, ConstraintSolver, InfiniteDomain, SetExpression,
    SetExpressionView, SolveOptions, SolveResult, Term,
};
use std::{collections::BTreeMap, str::FromStr};
use z3::{
    Params, SatResult, Solver,
    ast::{Bool, Dynamic, Int, String as Z3String},
};
#[derive(Debug, thiserror::Error)]
pub enum Z3SolverError {
    #[error("unsupported construct: {0}")]
    Unsupported(String),
    #[error("unknown variable `{0}`")]
    UnknownVariable(CanonicalId),
    #[error("invalid Z3 value")]
    InvalidModel,
}
#[derive(Default)]
pub struct Z3Solver;
impl Z3Solver {
    pub fn new() -> Self {
        Self
    }
}
impl ConstraintSolver for Z3Solver {
    type Error = Z3SolverError;
    fn solve(&self, p: &ConstraintProblem, o: SolveOptions) -> Result<SolveResult, Self::Error> {
        let solver = Solver::new();
        let mut params = Params::new();
        params.set_u32(
            "timeout",
            o.timeout().as_millis().min(u128::from(u32::MAX)) as u32,
        );
        solver.set_params(&params);
        let mut vars = BTreeMap::new();
        for v in p.variables() {
            let name = format!("rspdl_v_{}_{}", v.id.as_str().len(), v.id);
            let ast = match v.domain.value_type() {
                CanonicalType::Boolean => Dynamic::from_ast(&Bool::new_const(name)),
                CanonicalType::Integer => Dynamic::from_ast(&Int::new_const(name)),
                CanonicalType::String => Dynamic::from_ast(&Z3String::new_const(name)),
                _ => return Err(Z3SolverError::Unsupported("enum/refinement domain".into())),
            };
            vars.insert(v.id.clone(), ast);
        }
        for variable in p.variables() {
            solver.assert(Self::membership(
                vars.get(&variable.id).expect("declared variable"),
                &SetExpression::domain(variable.domain.clone()),
            )?);
        }
        solver.assert(Self::lower(p.assertion(), &vars)?);
        match solver.check() {
            SatResult::Unsat => Ok(SolveResult::Unsat),
            SatResult::Unknown => Ok(SolveResult::Unknown {
                reason: solver
                    .get_reason_unknown()
                    .unwrap_or_else(|| "unknown".into()),
            }),
            SatResult::Sat => {
                let m = solver.get_model().ok_or(Z3SolverError::InvalidModel)?;
                let mut out = BTreeMap::new();
                for (id, a) in vars {
                    let value = m.eval(&a, true).ok_or(Z3SolverError::InvalidModel)?;
                    let cv = if let Some(x) = value.as_bool() {
                        CanonicalValue::boolean(x.as_bool().ok_or(Z3SolverError::InvalidModel)?)
                    } else if let Some(x) = value.as_int() {
                        CanonicalValue::integer_from_decimal(x.to_string())
                            .map_err(|_| Z3SolverError::InvalidModel)?
                    } else if let Some(x) = value.as_string() {
                        CanonicalValue::string(x.as_string().ok_or(Z3SolverError::InvalidModel)?)
                    } else {
                        return Err(Z3SolverError::InvalidModel);
                    };
                    out.insert(id, cv);
                }
                Ok(SolveResult::Sat(CanonicalModel(out)))
            }
        }
    }
}
impl Z3Solver {
    fn lower(
        x: &BooleanExpression,
        v: &BTreeMap<CanonicalId, Dynamic>,
    ) -> Result<Bool, Z3SolverError> {
        match x.view() {
            BooleanExpressionView::Literal(x) => Ok(Bool::from_bool(x)),
            BooleanExpressionView::And(xs) => {
                let values = xs
                    .iter()
                    .map(|x| Self::lower(x, v))
                    .collect::<Result<Vec<_>, _>>()?;
                Ok(Bool::and(&values))
            }
            BooleanExpressionView::Or(xs) => {
                let values = xs
                    .iter()
                    .map(|x| Self::lower(x, v))
                    .collect::<Result<Vec<_>, _>>()?;
                Ok(Bool::or(&values))
            }
            BooleanExpressionView::Not(x) => Ok(Self::lower(x, v)?.not()),
            BooleanExpressionView::Atom(a) => match a.view() {
                AtomView::Equal(a, b) => Self::term(a, v)?
                    .safe_eq(&Self::term(b, v)?)
                    .map_err(|_| Z3SolverError::Unsupported("type mismatch".into())),
                AtomView::IntegerComparison(op, a, b) => {
                    let a = Self::term(a, v)?
                        .as_int()
                        .ok_or_else(|| Z3SolverError::Unsupported("integer comparison".into()))?;
                    let b = Self::term(b, v)?
                        .as_int()
                        .ok_or_else(|| Z3SolverError::Unsupported("integer comparison".into()))?;
                    Ok(match op {
                        rspdl_core::ComparisonOperator::Lt => a.lt(&b),
                        rspdl_core::ComparisonOperator::Le => a.le(&b),
                        rspdl_core::ComparisonOperator::Gt => a.gt(&b),
                        rspdl_core::ComparisonOperator::Ge => a.ge(&b),
                    })
                }
                AtomView::MemberOf(term, set) => Self::membership(&Self::term(term, v)?, set),
                AtomView::Predicate(_, _) => {
                    Err(Z3SolverError::Unsupported("predicate application".into()))
                }
            },
        }
    }
    fn term(t: &Term, v: &BTreeMap<CanonicalId, Dynamic>) -> Result<Dynamic, Z3SolverError> {
        match t {
            Term::Variable(x) => v
                .get(x.id())
                .cloned()
                .ok_or_else(|| Z3SolverError::UnknownVariable(x.id().clone())),
            Term::Constant(x) => match x.value_type() {
                CanonicalType::Boolean => {
                    Ok(Dynamic::from_ast(&Bool::from_bool(x.as_boolean().unwrap())))
                }
                CanonicalType::Integer => Ok(Dynamic::from_ast(
                    &Int::from_str(&x.as_integer().unwrap().to_string()).unwrap(),
                )),
                CanonicalType::String => Ok(Dynamic::from_ast(
                    &Z3String::from_str(x.as_string().unwrap()).unwrap(),
                )),
                _ => Err(Z3SolverError::Unsupported("enum/refinement value".into())),
            },
        }
    }
    fn membership(value: &Dynamic, set: &SetExpression) -> Result<Bool, Z3SolverError> {
        match set.view() {
            SetExpressionView::Domain(domain) => match domain.infinite_kind() {
                Some(InfiniteDomain::Integers | InfiniteDomain::Strings) => {
                    Ok(Bool::from_bool(true))
                }
                Some(InfiniteDomain::Primes) => {
                    Err(Z3SolverError::Unsupported("prime domain".into()))
                }
                None => Self::finite_membership(value, domain.finite_values().expect("finite")),
            },
            SetExpressionView::Literal(values) => Self::finite_membership(value, values),
            SetExpressionView::Union(xs) => {
                let ys = xs
                    .iter()
                    .map(|x| Self::membership(value, x))
                    .collect::<Result<Vec<_>, _>>()?;
                Ok(Bool::or(&ys))
            }
            SetExpressionView::Intersection(xs) => {
                let ys = xs
                    .iter()
                    .map(|x| Self::membership(value, x))
                    .collect::<Result<Vec<_>, _>>()?;
                Ok(Bool::and(&ys))
            }
            SetExpressionView::Difference(a, b) => {
                let left = Self::membership(value, a)?;
                let right = Self::membership(value, b)?.not();
                Ok(Bool::and(&[&left, &right]))
            }
        }
    }
    fn finite_membership(
        value: &Dynamic,
        values: &std::collections::BTreeSet<CanonicalValue>,
    ) -> Result<Bool, Z3SolverError> {
        let tests = values
            .iter()
            .map(|x| {
                Self::term(&Term::Constant(x.clone()), &BTreeMap::new()).and_then(|c| {
                    value
                        .safe_eq(&c)
                        .map_err(|_| Z3SolverError::Unsupported("set type".into()))
                })
            })
            .collect::<Result<Vec<_>, _>>()?;
        Ok(Bool::or(&tests))
    }
}
