//! Deterministic active-domain evaluator for safe stratified Datalog programs.
use rspdl_core::{
    AtomView, CanonicalId, CanonicalValue, DerivationRule, LogicProgram, ModelError,
    PredicateApplication, RuleLiteral, Term,
};
use std::collections::{BTreeMap, BTreeSet};
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct MaterializedDatabase {
    relations: BTreeMap<CanonicalId, BTreeSet<Vec<CanonicalValue>>>,
}
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct EvaluationStats {
    pub rounds: usize,
    pub delta_rule_evaluations: usize,
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
    #[error("unsupported constraint in rule `{rule}`")]
    UnsupportedConstraint { rule: CanonicalId },
    #[error("constraint evaluation failed in rule `{rule}`: {source}")]
    ConstraintEvaluation {
        rule: CanonicalId,
        source: Box<ModelError>,
    },
}
pub struct DatalogEvaluator;
impl DatalogEvaluator {
    pub fn evaluate(program: &LogicProgram) -> Result<MaterializedDatabase, DatalogError> {
        Self::evaluate_with_stats(program).map(|(database, _)| database)
    }
    pub fn evaluate_with_stats(
        program: &LogicProgram,
    ) -> Result<(MaterializedDatabase, EvaluationStats), DatalogError> {
        Self::validate(program)?;
        let mut stats = EvaluationStats::default();
        let mut db = MaterializedDatabase::default();
        for f in program.facts() {
            let a = f.application();
            let tuple = ground(a).ok_or(DatalogError::NonGroundFact)?;
            db.relations
                .entry(a.signature().id().clone())
                .or_default()
                .insert(tuple);
        }
        let strata = strata(program);
        let highest = strata.values().copied().max().unwrap_or(0);
        for stratum in 0..=highest {
            let rules: Vec<_> = program
                .rules()
                .iter()
                .filter(|rule| strata[rule.head().signature().id()] == stratum)
                .collect();
            let mut delta = BTreeMap::new();
            for rule in &rules {
                for tuple in derive(rule, &db)? {
                    if db
                        .relations
                        .entry(rule.head().signature().id().clone())
                        .or_default()
                        .insert(tuple.clone())
                    {
                        delta
                            .entry(rule.head().signature().id().clone())
                            .or_insert_with(BTreeSet::new)
                            .insert(tuple);
                    }
                }
            }
            loop {
                let mut next = BTreeMap::new();
                for rule in &rules {
                    for (index, literal) in rule.body().iter().enumerate() {
                        let RuleLiteral::Positive(application) = literal else {
                            continue;
                        };
                        if strata[application.signature().id()] != stratum
                            || !delta.contains_key(application.signature().id())
                        {
                            continue;
                        };
                        stats.delta_rule_evaluations += 1;
                        for tuple in derive_delta(rule, &db, &delta, Some(index))? {
                            if !db
                                .relations
                                .get(rule.head().signature().id())
                                .is_some_and(|all| all.contains(&tuple))
                            {
                                next.entry(rule.head().signature().id().clone())
                                    .or_insert_with(BTreeSet::new)
                                    .insert(tuple);
                            }
                        }
                    }
                }
                if next.is_empty() {
                    break;
                }
                stats.rounds += 1;
                for (p, ts) in &next {
                    db.relations
                        .entry(p.clone())
                        .or_default()
                        .extend(ts.iter().cloned());
                }
                delta = next;
            }
        }
        Ok((db, stats))
    }
    fn validate(program: &LogicProgram) -> Result<(), DatalogError> {
        let mut edges: BTreeMap<CanonicalId, BTreeSet<CanonicalId>> = BTreeMap::new();
        let mut negative = Vec::new();
        for rule in program.rules() {
            let head = rule.head().signature().id().clone();
            for literal in rule.body() {
                match literal {
                    RuleLiteral::Positive(application) => {
                        edges
                            .entry(head.clone())
                            .or_default()
                            .insert(application.signature().id().clone());
                    }
                    RuleLiteral::Negative(application) => {
                        edges
                            .entry(head.clone())
                            .or_default()
                            .insert(application.signature().id().clone());
                        negative.push((
                            rule.id().clone(),
                            head.clone(),
                            application.signature().id().clone(),
                        ));
                    }
                    RuleLiteral::Constraint(_) => {}
                }
            }
        }
        for (rule, head, target) in negative {
            if reachable(&edges, &target, &head, &mut BTreeSet::new()) {
                return Err(DatalogError::NegativeCycle {
                    rule,
                    predicate: target,
                });
            }
        }
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
                        AtomView::Predicate(_, _) => {
                            return Err(DatalogError::UnsupportedConstraint {
                                rule: rule.id().clone(),
                            });
                        }
                        _ => {
                            return Err(DatalogError::UnsupportedConstraint {
                                rule: rule.id().clone(),
                            });
                        }
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
        }
        Ok(())
    }
}

/// Computes strata for a program that has already passed negative-cycle validation.
fn strata(program: &LogicProgram) -> BTreeMap<CanonicalId, usize> {
    let mut result: BTreeMap<CanonicalId, usize> = program
        .predicates()
        .keys()
        .cloned()
        .map(|id| (id, 0))
        .collect();
    for _ in 0..program.predicates().len().max(1) {
        let mut changed = false;
        for rule in program.rules() {
            let head = rule.head().signature().id();
            for literal in rule.body() {
                let (target, extra) = match literal {
                    RuleLiteral::Positive(a) => (a.signature().id(), 0),
                    RuleLiteral::Negative(a) => (a.signature().id(), 1),
                    RuleLiteral::Constraint(_) => continue,
                };
                let required = result[target] + extra;
                if result[head] < required {
                    result.insert(head.clone(), required);
                    changed = true;
                }
            }
        }
        if !changed {
            return result;
        }
    }
    panic!("validated Datalog program exceeded the stratification iteration bound")
}

fn reachable(
    edges: &BTreeMap<CanonicalId, BTreeSet<CanonicalId>>,
    current: &CanonicalId,
    goal: &CanonicalId,
    seen: &mut BTreeSet<CanonicalId>,
) -> bool {
    if current == goal {
        return true;
    }
    seen.insert(current.clone())
        && edges
            .get(current)
            .is_some_and(|next| next.iter().any(|node| reachable(edges, node, goal, seen)))
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
fn derive(
    rule: &DerivationRule,
    db: &MaterializedDatabase,
) -> Result<Vec<Vec<CanonicalValue>>, DatalogError> {
    derive_delta(rule, db, &BTreeMap::new(), None)
}
fn derive_delta(
    rule: &DerivationRule,
    db: &MaterializedDatabase,
    delta: &BTreeMap<CanonicalId, BTreeSet<Vec<CanonicalValue>>>,
    delta_index: Option<usize>,
) -> Result<Vec<Vec<CanonicalValue>>, DatalogError> {
    let mut envs = vec![BTreeMap::new()];
    let mut plan: Vec<_> = rule.body().iter().enumerate().collect();
    plan.sort_by_key(|(_, literal)| match literal {
        RuleLiteral::Positive(_) => 0_u8,
        RuleLiteral::Negative(_) => 1,
        RuleLiteral::Constraint(_) => 2,
    });
    for (index, lit) in plan {
        match lit {
            RuleLiteral::Positive(a) => {
                let mut next = vec![];
                let tuples = if Some(index) == delta_index {
                    delta.get(a.signature().id())
                } else {
                    db.relations.get(a.signature().id())
                };
                join(a, tuples, &envs, &mut next);
                envs = next;
            }
            RuleLiteral::Negative(a) => envs.retain(|e| {
                !db.relations
                    .get(a.signature().id())
                    .into_iter()
                    .flatten()
                    .any(|t| unify(a, t, e).is_some())
            }),
            RuleLiteral::Constraint(atom) => {
                let mut next = Vec::with_capacity(envs.len());
                for environment in envs {
                    if constraint(rule, atom, &environment)? {
                        next.push(environment);
                    }
                }
                envs = next;
            }
        }
    }
    Ok(envs
        .into_iter()
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
        .collect())
}
fn join(
    application: &PredicateApplication,
    tuples: Option<&BTreeSet<Vec<CanonicalValue>>>,
    environments: &[BTreeMap<CanonicalId, CanonicalValue>],
    out: &mut Vec<BTreeMap<CanonicalId, CanonicalValue>>,
) {
    let bound_positions: Vec<_> = application
        .arguments()
        .iter()
        .enumerate()
        .filter_map(|(index, term)| match term {
            Term::Variable(variable)
                if environments
                    .first()
                    .is_some_and(|environment| environment.contains_key(variable.id())) =>
            {
                Some((index, variable.id()))
            }
            _ => None,
        })
        .collect();

    if bound_positions.is_empty() {
        for tuple in tuples.into_iter().flatten() {
            for environment in environments {
                if let Some(joined) = unify(application, tuple, environment) {
                    out.push(joined);
                }
            }
        }
        return;
    }

    let mut index: BTreeMap<Vec<CanonicalValue>, Vec<&[CanonicalValue]>> = BTreeMap::new();
    for tuple in tuples.into_iter().flatten() {
        let key = bound_positions
            .iter()
            .map(|(position, _)| tuple[*position].clone())
            .collect();
        index.entry(key).or_default().push(tuple);
    }
    for environment in environments {
        let key: Vec<_> = bound_positions
            .iter()
            .map(|(_, variable)| {
                environment
                    .get(*variable)
                    .expect("join key variables are bound")
                    .clone()
            })
            .collect();
        for tuple in index.get(&key).into_iter().flatten() {
            if let Some(joined) = unify(application, tuple, environment) {
                out.push(joined);
            }
        }
    }
}
fn unify(
    a: &PredicateApplication,
    t: &[CanonicalValue],
    e: &BTreeMap<CanonicalId, CanonicalValue>,
) -> Option<BTreeMap<CanonicalId, CanonicalValue>> {
    let mut pending: BTreeMap<&CanonicalId, &CanonicalValue> = BTreeMap::new();
    for (x, v) in a.arguments().iter().zip(t) {
        match x {
            Term::Constant(c) if c != v => return None,
            Term::Constant(_) => {}
            Term::Variable(var) => {
                if e.get(var.id()).is_some_and(|old| old != v)
                    || pending.get(var.id()).is_some_and(|old| *old != v)
                {
                    return None;
                }
                if !e.contains_key(var.id()) && !pending.contains_key(var.id()) {
                    pending.insert(var.id(), v);
                }
            }
        }
    }
    let mut out = e.clone();
    out.extend(
        pending
            .into_iter()
            .map(|(variable, value)| (variable.clone(), value.clone())),
    );
    Some(out)
}
fn val(t: &Term, e: &BTreeMap<CanonicalId, CanonicalValue>) -> Option<CanonicalValue> {
    match t {
        Term::Constant(v) => Some(v.clone()),
        Term::Variable(v) => e.get(v.id()).cloned(),
    }
}
fn constraint(
    rule: &DerivationRule,
    a: &rspdl_core::Atom,
    e: &BTreeMap<CanonicalId, CanonicalValue>,
) -> Result<bool, DatalogError> {
    match a.view() {
        AtomView::Equal(x, y) => Ok(match (val(x, e), val(y, e)) {
            (Some(x), Some(y)) => x == y,
            _ => false,
        }),
        AtomView::MemberOf(x, s) => {
            let Some(value) = val(x, e) else {
                return Ok(false);
            };
            s.contains(&value)
                .map_err(|source| DatalogError::ConstraintEvaluation {
                    rule: rule.id().clone(),
                    source: Box::new(source),
                })
        }
        AtomView::IntegerComparison(op, x, y) => {
            let (Some(x), Some(y)) = (val(x, e), val(y, e)) else {
                return Ok(false);
            };
            Ok(match op {
                rspdl_core::ComparisonOperator::Lt => x < y,
                rspdl_core::ComparisonOperator::Le => x <= y,
                rspdl_core::ComparisonOperator::Gt => x > y,
                rspdl_core::ComparisonOperator::Ge => x >= y,
            })
        }
        AtomView::Predicate(_, _) => Err(DatalogError::UnsupportedConstraint {
            rule: rule.id().clone(),
        }),
        _ => Err(DatalogError::UnsupportedConstraint {
            rule: rule.id().clone(),
        }),
    }
}
