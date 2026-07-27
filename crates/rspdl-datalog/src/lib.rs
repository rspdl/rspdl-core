//! Deterministic active-domain evaluator for safe stratified Datalog programs.
use rspdl_core::{
    AtomView, CanonicalId, CanonicalValue, DerivationRule, LogicProgram, PredicateApplication,
    RuleLiteral, Term,
};
use std::collections::{BTreeMap, BTreeSet};
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct MaterializedDatabase {
    relations: BTreeMap<CanonicalId, BTreeSet<Vec<CanonicalValue>>>,
}
impl MaterializedDatabase {
    pub fn tuples(&self, predicate: &CanonicalId) -> Option<&BTreeSet<Vec<CanonicalValue>>> {
        self.relations.get(predicate)
    }
    pub fn contains(&self, app: &PredicateApplication) -> bool {
        ground(app).is_some_and(|x| {
            self.relations
                .get(app.signature().id())
                .is_some_and(|r| r.contains(&x))
        })
    }
}
#[derive(Clone, Debug, thiserror::Error, Eq, PartialEq)]
pub enum DatalogError {
    #[error("unsafe variable `{variable}` in rule `{rule}`")]
    UnsafeVariable {
        rule: CanonicalId,
        variable: CanonicalId,
    },
    #[error("negative dependency cycle in rule `{rule}` involving `{predicate}`")]
    NegativeCycle {
        rule: CanonicalId,
        predicate: CanonicalId,
    },
    #[error("non-ground fact")]
    NonGroundFact,
}
pub struct DatalogEvaluator;
impl DatalogEvaluator {
    pub fn evaluate(program: &LogicProgram) -> Result<MaterializedDatabase, DatalogError> {
        Self::validate(program)?;
        let mut db = MaterializedDatabase::default();
        for f in program.facts() {
            let a = f.application();
            let tuple = ground(a).ok_or(DatalogError::NonGroundFact)?;
            db.relations
                .entry(a.signature().id().clone())
                .or_default()
                .insert(tuple);
        }
        let mut changed = true;
        while changed {
            changed = false;
            for rule in program.rules() {
                for tuple in derive(rule, &db) {
                    if db
                        .relations
                        .entry(rule.head().signature().id().clone())
                        .or_default()
                        .insert(tuple)
                    {
                        changed = true;
                    }
                }
            }
        }
        Ok(db)
    }
    fn validate(program: &LogicProgram) -> Result<(), DatalogError> {
        for rule in program.rules() {
            let mut bound = BTreeSet::new();
            for lit in rule.body() {
                if let RuleLiteral::Positive(a) = lit {
                    for t in a.arguments() {
                        if let Term::Variable(v) = t {
                            bound.insert(v.id().clone());
                        }
                    }
                }
            }
            let mut need = Vec::new();
            terms(rule.head(), &mut need);
            for lit in rule.body() {
                match lit {
                    RuleLiteral::Negative(a) => terms(a, &mut need),
                    RuleLiteral::Constraint(a) => match a.view() {
                        AtomView::Equal(a, b) => {
                            term(a, &mut need);
                            term(b, &mut need)
                        }
                        AtomView::MemberOf(a, _) => term(a, &mut need),
                        AtomView::IntegerComparison(_, a, b) => {
                            term(a, &mut need);
                            term(b, &mut need)
                        }
                        _ => {}
                    },
                    _ => {}
                }
            }
            for v in need {
                if !bound.contains(&v) {
                    return Err(DatalogError::UnsafeVariable {
                        rule: rule.id().clone(),
                        variable: v,
                    });
                }
            }
            for lit in rule.body() {
                if let RuleLiteral::Negative(a) = lit
                    && a.signature().id() == rule.head().signature().id()
                {
                    return Err(DatalogError::NegativeCycle {
                        rule: rule.id().clone(),
                        predicate: a.signature().id().clone(),
                    });
                }
            }
        }
        Ok(())
    }
}
fn term(t: &Term, out: &mut Vec<CanonicalId>) {
    if let Term::Variable(v) = t {
        out.push(v.id().clone())
    }
}
fn terms(a: &PredicateApplication, out: &mut Vec<CanonicalId>) {
    for t in a.arguments() {
        term(t, out)
    }
}
fn ground(a: &PredicateApplication) -> Option<Vec<CanonicalValue>> {
    a.arguments()
        .iter()
        .map(|t| {
            if let Term::Constant(v) = t {
                Some(v.clone())
            } else {
                None
            }
        })
        .collect()
}
fn derive(rule: &DerivationRule, db: &MaterializedDatabase) -> Vec<Vec<CanonicalValue>> {
    let mut envs = vec![BTreeMap::new()];
    for lit in rule.body() {
        match lit {
            RuleLiteral::Positive(a) => {
                let mut next = vec![];
                for tuple in db.relations.get(a.signature().id()).into_iter().flatten() {
                    for e in &envs {
                        if let Some(x) = unify(a, tuple, e) {
                            next.push(x)
                        }
                    }
                }
                envs = next;
            }
            RuleLiteral::Negative(a) => envs.retain(|e| {
                !db.relations
                    .get(a.signature().id())
                    .into_iter()
                    .flatten()
                    .any(|t| unify(a, t, e).is_some())
            }),
            RuleLiteral::Constraint(atom) => envs.retain(|e| constraint(atom, e)),
        }
    }
    envs.into_iter()
        .filter_map(|e| {
            rule.head()
                .arguments()
                .iter()
                .map(|t| match t {
                    Term::Constant(v) => Some(v.clone()),
                    Term::Variable(v) => e.get(v.id()).cloned(),
                })
                .collect()
        })
        .collect()
}
fn unify(
    a: &PredicateApplication,
    t: &[CanonicalValue],
    e: &BTreeMap<CanonicalId, CanonicalValue>,
) -> Option<BTreeMap<CanonicalId, CanonicalValue>> {
    let mut out = e.clone();
    for (x, v) in a.arguments().iter().zip(t) {
        match x {
            Term::Constant(c) if c != v => return None,
            Term::Constant(_) => {}
            Term::Variable(var) => match out.get(var.id()) {
                Some(old) if old != v => return None,
                _ => {
                    out.insert(var.id().clone(), v.clone());
                }
            },
        }
    }
    Some(out)
}
fn val(t: &Term, e: &BTreeMap<CanonicalId, CanonicalValue>) -> Option<CanonicalValue> {
    match t {
        Term::Constant(v) => Some(v.clone()),
        Term::Variable(v) => e.get(v.id()).cloned(),
    }
}
fn constraint(a: &rspdl_core::Atom, e: &BTreeMap<CanonicalId, CanonicalValue>) -> bool {
    match a.view() {
        AtomView::Equal(x, y) => val(x, e) == val(y, e),
        AtomView::MemberOf(x, s) => val(x, e).is_some_and(|v| s.contains(&v).unwrap_or(false)),
        AtomView::IntegerComparison(op, x, y) => {
            let (Some(x), Some(y)) = (val(x, e), val(y, e)) else {
                return false;
            };
            match op {
                rspdl_core::ComparisonOperator::Lt => x < y,
                rspdl_core::ComparisonOperator::Le => x <= y,
                rspdl_core::ComparisonOperator::Gt => x > y,
                rspdl_core::ComparisonOperator::Ge => x >= y,
            }
        }
        AtomView::Predicate(_, _) => false,
    }
}
