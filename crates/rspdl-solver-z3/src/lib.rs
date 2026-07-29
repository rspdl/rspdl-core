//! Z3-backed typed constraint solving; Z3 values do not escape this crate.
use rspdl_core::{
    AtomView, BooleanExpression, BooleanExpressionView, CanonicalId, CanonicalModel, CanonicalType,
    CanonicalValue, ConstraintProblem, ConstraintSolver, EnumType, InfiniteDomain, SetExpression,
    SetExpressionView, SolveOptions, SolveResult, Term,
};
use std::{
    collections::{BTreeMap, BTreeSet},
    str::FromStr,
};
use z3::{
    Config, FuncDecl, Params, SatResult, Solver, Sort, Symbol,
    ast::{Bool, Dynamic, Int, String as Z3String},
    with_z3_config,
};
#[derive(Debug, thiserror::Error)]
pub enum Z3SolverError {
    #[error("variable `{variable}` declared `{declared}` but used as `{actual}`")]
    VariableTypeMismatch {
        variable: CanonicalId,
        declared: CanonicalType,
        actual: CanonicalType,
    },
    #[error("unsupported construct: {0}")]
    Unsupported(String),
    #[error("unknown variable `{0}`")]
    UnknownVariable(CanonicalId),
    #[error("invalid Z3 value")]
    InvalidModel,
}
#[derive(Default)]
pub struct Z3Solver;
struct EnumEncoding {
    sort: Sort,
    constructors: BTreeMap<CanonicalId, FuncDecl>,
    testers: BTreeMap<CanonicalId, FuncDecl>,
}
impl Z3Solver {
    fn collect_enum_types(x: &BooleanExpression, out: &mut BTreeSet<EnumType>) {
        match x.view() {
            BooleanExpressionView::Atom(a) => match a.view() {
                AtomView::Equal(a, b) | AtomView::IntegerComparison(_, a, b) => {
                    Self::collect_term(a, out);
                    Self::collect_term(b, out)
                }
                AtomView::MemberOf(a, s) => {
                    Self::collect_term(a, out);
                    if let CanonicalType::Enum(e) = s.value_type() {
                        out.insert(e.clone());
                    }
                }
                AtomView::Predicate(_, xs) => {
                    for x in xs {
                        Self::collect_term(x, out)
                    }
                }
                _ => {}
            },
            BooleanExpressionView::And(xs) | BooleanExpressionView::Or(xs) => {
                for x in xs {
                    Self::collect_enum_types(x, out)
                }
            }
            BooleanExpressionView::Not(x) => Self::collect_enum_types(x, out),
            _ => {}
        }
    }
    fn collect_term(t: &Term, out: &mut BTreeSet<EnumType>) {
        if let CanonicalType::Enum(e) = t.value_type() {
            out.insert(e.clone());
        }
    }
    fn insert_enum(enums: &mut BTreeMap<EnumType, EnumEncoding>, kind: &EnumType) {
        let names: Vec<Symbol> = kind
            .variants()
            .iter()
            .map(|x| Self::enum_symbol(kind, x).into())
            .collect();
        let (sort, constructors, testers) =
            Sort::enumeration(Self::symbol("rspdl_t", kind.id()).into(), &names);
        enums.insert(
            kind.clone(),
            EnumEncoding {
                sort,
                constructors: kind.variants().iter().cloned().zip(constructors).collect(),
                testers: kind.variants().iter().cloned().zip(testers).collect(),
            },
        );
    }
    fn symbol(prefix: &str, id: &CanonicalId) -> String {
        format!("{prefix}_{}_{}", id.as_str().len(), id)
    }
    fn enum_symbol(kind: &EnumType, variant: &CanonicalId) -> String {
        format!(
            "rspdl_e_{}_{}_{}_{}",
            kind.id().as_str().len(),
            kind.id(),
            variant.as_str().len(),
            variant
        )
    }
    fn canonical_integer_text(value: &Int) -> Result<String, Z3SolverError> {
        if let Some(value) = value.as_i64() {
            return Ok(value.to_string());
        }

        let rendered = value.to_string();
        if rendered.bytes().all(|byte| byte.is_ascii_digit()) {
            return Ok(rendered);
        }
        if let Some(magnitude) = rendered
            .strip_prefix("(- ")
            .and_then(|value| value.strip_suffix(')'))
            && !magnitude.is_empty()
            && magnitude.bytes().all(|byte| byte.is_ascii_digit())
        {
            return Ok(format!("-{magnitude}"));
        }
        Err(Z3SolverError::InvalidModel)
    }
    fn validate_expression(
        x: &BooleanExpression,
        d: &BTreeMap<CanonicalId, CanonicalType>,
    ) -> Result<(), Z3SolverError> {
        match x.view() {
            BooleanExpressionView::Atom(a) => match a.view() {
                AtomView::Equal(a, b) | AtomView::IntegerComparison(_, a, b) => {
                    Self::validate_term(a, d)?;
                    Self::validate_term(b, d)?;
                }
                AtomView::MemberOf(a, _) => Self::validate_term(a, d)?,
                AtomView::Predicate(_, args) => {
                    for a in args {
                        Self::validate_term(a, d)?
                    }
                }
                _ => {
                    return Err(Z3SolverError::Unsupported("unknown atom expression".into()));
                }
            },
            BooleanExpressionView::And(xs) | BooleanExpressionView::Or(xs) => {
                for x in xs {
                    Self::validate_expression(x, d)?
                }
            }
            BooleanExpressionView::Not(x) => Self::validate_expression(x, d)?,
            _ => {}
        }
        Ok(())
    }
    fn validate_term(
        t: &Term,
        d: &BTreeMap<CanonicalId, CanonicalType>,
    ) -> Result<(), Z3SolverError> {
        if let Term::Variable(v) = t {
            let declared = d
                .get(v.id())
                .ok_or_else(|| Z3SolverError::UnknownVariable(v.id().clone()))?;
            if declared != v.value_type() {
                return Err(Z3SolverError::VariableTypeMismatch {
                    variable: v.id().clone(),
                    declared: declared.clone(),
                    actual: v.value_type().clone(),
                });
            }
        }
        Ok(())
    }
    pub fn new() -> Self {
        Self
    }
}
impl ConstraintSolver for Z3Solver {
    type Error = Z3SolverError;
    fn solve(&self, p: &ConstraintProblem, o: SolveOptions) -> Result<SolveResult, Self::Error> {
        let config = Config::new();
        with_z3_config(&config, || Self::solve_in_context(p, o))
    }
}
impl Z3Solver {
    fn solve_in_context(
        p: &ConstraintProblem,
        o: SolveOptions,
    ) -> Result<SolveResult, Z3SolverError> {
        let declared = p
            .variables()
            .iter()
            .map(|variable| {
                (
                    variable.id().clone(),
                    variable.domain().value_type().clone(),
                )
            })
            .collect();
        Self::validate_expression(p.assertion(), &declared)?;
        let solver = Solver::new();
        let mut params = Params::new();
        params.set_u32(
            "timeout",
            o.timeout().as_millis().min(u128::from(u32::MAX)) as u32,
        );
        solver.set_params(&params);
        let mut enum_types = BTreeSet::new();
        Self::collect_enum_types(p.assertion(), &mut enum_types);
        let mut enums = BTreeMap::new();
        for variable in p.variables() {
            if let CanonicalType::Enum(kind) = variable.domain().value_type()
                && !enums.contains_key(kind)
            {
                Self::insert_enum(&mut enums, kind);
            }
        }
        for kind in enum_types {
            if !enums.contains_key(&kind) {
                Self::insert_enum(&mut enums, &kind);
            }
        }
        let mut vars = BTreeMap::new();
        for v in p.variables() {
            let name = Self::symbol("rspdl_v", v.id());
            let ast = match v.domain().value_type() {
                CanonicalType::Boolean => Dynamic::from_ast(&Bool::new_const(name)),
                CanonicalType::Integer => Dynamic::from_ast(&Int::new_const(name)),
                CanonicalType::String => Dynamic::from_ast(&Z3String::new_const(name)),
                CanonicalType::Enum(kind) => Dynamic::new_const(
                    name,
                    &enums.get(kind).ok_or(Z3SolverError::InvalidModel)?.sort,
                ),
                _ => return Err(Z3SolverError::Unsupported("refinement domain".into())),
            };
            vars.insert(v.id().clone(), ast);
        }
        for variable in p.variables() {
            solver.assert(Self::membership(
                vars.get(variable.id()).expect("declared variable"),
                &SetExpression::domain(variable.domain().clone()),
                &enums,
            )?);
        }
        solver.assert(Self::lower(p.assertion(), &vars, &enums)?);
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
                    let kind = declared.get(&id).ok_or(Z3SolverError::InvalidModel)?;
                    let cv = if let CanonicalType::Enum(kind) = kind {
                        let encoding = enums.get(kind).ok_or(Z3SolverError::InvalidModel)?;
                        let variant = kind
                            .variants()
                            .iter()
                            .find(|variant| {
                                encoding
                                    .testers
                                    .get(*variant)
                                    .and_then(|tester| m.eval(&tester.apply(&[&value]), true))
                                    .and_then(|x| x.as_bool())
                                    .and_then(|x| x.as_bool())
                                    .unwrap_or(false)
                            })
                            .ok_or(Z3SolverError::InvalidModel)?;
                        CanonicalValue::enum_variant(kind.clone(), variant.clone())
                            .map_err(|_| Z3SolverError::InvalidModel)?
                    } else if let Some(x) = value.as_bool() {
                        CanonicalValue::boolean(x.as_bool().ok_or(Z3SolverError::InvalidModel)?)
                    } else if let Some(x) = value.as_int() {
                        CanonicalValue::integer_from_decimal(Self::canonical_integer_text(&x)?)
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
        e: &BTreeMap<EnumType, EnumEncoding>,
    ) -> Result<Bool, Z3SolverError> {
        match x.view() {
            BooleanExpressionView::Literal(x) => Ok(Bool::from_bool(x)),
            BooleanExpressionView::And(xs) => {
                let values = xs
                    .iter()
                    .map(|x| Self::lower(x, v, e))
                    .collect::<Result<Vec<_>, _>>()?;
                Ok(Bool::and(&values))
            }
            BooleanExpressionView::Or(xs) => {
                let values = xs
                    .iter()
                    .map(|x| Self::lower(x, v, e))
                    .collect::<Result<Vec<_>, _>>()?;
                Ok(Bool::or(&values))
            }
            BooleanExpressionView::Not(x) => Ok(Self::lower(x, v, e)?.not()),
            BooleanExpressionView::Atom(a) => match a.view() {
                AtomView::Equal(a, b) => Self::term(a, v, e)?
                    .safe_eq(&Self::term(b, v, e)?)
                    .map_err(|_| Z3SolverError::Unsupported("type mismatch".into())),
                AtomView::IntegerComparison(op, a, b) => {
                    let a = Self::term(a, v, e)?
                        .as_int()
                        .ok_or_else(|| Z3SolverError::Unsupported("integer comparison".into()))?;
                    let b = Self::term(b, v, e)?
                        .as_int()
                        .ok_or_else(|| Z3SolverError::Unsupported("integer comparison".into()))?;
                    Ok(match op {
                        rspdl_core::ComparisonOperator::Lt => a.lt(&b),
                        rspdl_core::ComparisonOperator::Le => a.le(&b),
                        rspdl_core::ComparisonOperator::Gt => a.gt(&b),
                        rspdl_core::ComparisonOperator::Ge => a.ge(&b),
                    })
                }
                AtomView::MemberOf(term, set) => Self::membership(&Self::term(term, v, e)?, set, e),
                AtomView::Predicate(_, _) => {
                    Err(Z3SolverError::Unsupported("predicate application".into()))
                }
                _ => Err(Z3SolverError::Unsupported("unknown atom expression".into())),
            },
            _ => Err(Z3SolverError::Unsupported(
                "unknown boolean expression".into(),
            )),
        }
    }
    fn term(
        t: &Term,
        v: &BTreeMap<CanonicalId, Dynamic>,
        e: &BTreeMap<EnumType, EnumEncoding>,
    ) -> Result<Dynamic, Z3SolverError> {
        match t {
            Term::Variable(x) => v
                .get(x.id())
                .cloned()
                .ok_or_else(|| Z3SolverError::UnknownVariable(x.id().clone())),
            Term::Constant(x) => match x.value_type() {
                CanonicalType::Boolean => Ok(Dynamic::from_ast(&Bool::from_bool(
                    x.as_boolean().ok_or(Z3SolverError::InvalidModel)?,
                ))),
                CanonicalType::Integer => {
                    let text = x
                        .as_integer()
                        .ok_or(Z3SolverError::InvalidModel)?
                        .to_string();
                    let value = Int::from_str(&text).map_err(|_| {
                        Z3SolverError::Unsupported(format!("integer constant `{text}`"))
                    })?;
                    Ok(Dynamic::from_ast(&value))
                }
                CanonicalType::String => {
                    let text = x.as_string().ok_or(Z3SolverError::InvalidModel)?;
                    let value = Z3String::from_str(text)
                        .map_err(|_| Z3SolverError::Unsupported("string constant".into()))?;
                    Ok(Dynamic::from_ast(&value))
                }
                CanonicalType::Enum(kind) => {
                    let encoding = e.get(kind).ok_or(Z3SolverError::InvalidModel)?;
                    let variant = x.as_enum_variant().ok_or(Z3SolverError::InvalidModel)?;
                    Ok(Dynamic::from_ast(
                        &encoding
                            .constructors
                            .get(variant)
                            .ok_or(Z3SolverError::InvalidModel)?
                            .apply(&[]),
                    ))
                }
                _ => Err(Z3SolverError::Unsupported("refinement value".into())),
            },
        }
    }
    fn membership(
        value: &Dynamic,
        set: &SetExpression,
        e: &BTreeMap<EnumType, EnumEncoding>,
    ) -> Result<Bool, Z3SolverError> {
        match set.view() {
            SetExpressionView::Domain(domain) => match domain.infinite_kind() {
                Some(InfiniteDomain::Integers | InfiniteDomain::Strings) => {
                    Ok(Bool::from_bool(true))
                }
                Some(InfiniteDomain::Primes) => {
                    Err(Z3SolverError::Unsupported("prime domain".into()))
                }
                None => Self::finite_membership(value, domain.finite_values().expect("finite"), e),
            },
            SetExpressionView::Literal(values) => Self::finite_membership(value, values, e),
            SetExpressionView::Union(xs) => {
                let ys = xs
                    .iter()
                    .map(|x| Self::membership(value, x, e))
                    .collect::<Result<Vec<_>, _>>()?;
                Ok(Bool::or(&ys))
            }
            SetExpressionView::Intersection(xs) => {
                let ys = xs
                    .iter()
                    .map(|x| Self::membership(value, x, e))
                    .collect::<Result<Vec<_>, _>>()?;
                Ok(Bool::and(&ys))
            }
            SetExpressionView::Difference(a, b) => {
                let left = Self::membership(value, a, e)?;
                let right = Self::membership(value, b, e)?.not();
                Ok(Bool::and(&[&left, &right]))
            }
            _ => Err(Z3SolverError::Unsupported("unknown set expression".into())),
        }
    }
    fn finite_membership(
        value: &Dynamic,
        values: &std::collections::BTreeSet<CanonicalValue>,
        e: &BTreeMap<EnumType, EnumEncoding>,
    ) -> Result<Bool, Z3SolverError> {
        let tests = values
            .iter()
            .map(|x| {
                Self::term(&Term::Constant(x.clone()), &BTreeMap::new(), e).and_then(|c| {
                    value
                        .safe_eq(&c)
                        .map_err(|_| Z3SolverError::Unsupported("set type".into()))
                })
            })
            .collect::<Result<Vec<_>, _>>()?;
        Ok(Bool::or(&tests))
    }
}
