use rspdl_core::{
    Atom, CanonicalId, CanonicalType, CanonicalValue, DerivationRule, Domain, Fact, LogicProgram,
    PredicateApplication, PredicateSignature, RuleLiteral, SetExpression, Term, Variable,
};
use rspdl_datalog::DatalogEvaluator;
fn id(s: &str) -> CanonicalId {
    CanonicalId::new(s).unwrap()
}
#[test]
fn derives_transitive_closure_to_a_fixed_point() {
    let edge = PredicateSignature::new(
        id("edge"),
        vec![CanonicalType::Integer, CanonicalType::Integer],
    );
    let path = PredicateSignature::new(
        id("path"),
        vec![CanonicalType::Integer, CanonicalType::Integer],
    );
    let c = |n| Term::Constant(CanonicalValue::integer(n));
    let x = Term::Variable(Variable::new(id("x"), CanonicalType::Integer));
    let y = Term::Variable(Variable::new(id("y"), CanonicalType::Integer));
    let z = Term::Variable(Variable::new(id("z"), CanonicalType::Integer));
    let app =
        |p: &PredicateSignature, a: Vec<Term>| PredicateApplication::new(p.clone(), a).unwrap();
    let facts = vec![
        Fact::new(app(&edge, vec![c(1), c(2)])).unwrap(),
        Fact::new(app(&edge, vec![c(2), c(3)])).unwrap(),
    ];
    let r1 = DerivationRule::new(
        id("base"),
        app(&path, vec![x.clone(), y.clone()]),
        vec![RuleLiteral::Positive(app(
            &edge,
            vec![x.clone(), y.clone()],
        ))],
    );
    let r2 = DerivationRule::new(
        id("step"),
        app(&path, vec![x.clone(), z.clone()]),
        vec![
            RuleLiteral::Positive(app(&path, vec![x, y])),
            RuleLiteral::Positive(app(
                &edge,
                vec![
                    Term::Variable(Variable::new(id("y"), CanonicalType::Integer)),
                    z,
                ],
            )),
        ],
    );
    let p = LogicProgram::new(vec![edge, path.clone()], facts, vec![r1, r2]).unwrap();
    let (db, stats) = DatalogEvaluator::evaluate_with_stats(&p).unwrap();
    assert_eq!(db.tuples(path.id()).unwrap().len(), 3);
    assert!(stats.delta_rule_evaluations > 0);
}

#[test]
fn rejects_an_indirect_negative_cycle() {
    let base = PredicateSignature::new(id("base"), vec![CanonicalType::Integer]);
    let p = PredicateSignature::new(id("p"), vec![CanonicalType::Integer]);
    let q = PredicateSignature::new(id("q"), vec![CanonicalType::Integer]);
    let x = Term::Variable(Variable::new(id("x"), CanonicalType::Integer));
    let app =
        |s: &PredicateSignature| PredicateApplication::new(s.clone(), vec![x.clone()]).unwrap();
    let r1 = DerivationRule::new(
        id("p_rule"),
        app(&p),
        vec![
            RuleLiteral::Positive(app(&base)),
            RuleLiteral::Negative(app(&q)),
        ],
    );
    let r2 = DerivationRule::new(id("q_rule"), app(&q), vec![RuleLiteral::Positive(app(&p))]);
    let program = LogicProgram::new(vec![base, p, q], vec![], vec![r1, r2]).unwrap();
    assert!(DatalogEvaluator::evaluate(&program).is_err());
}

#[test]
fn completes_lower_stratum_before_applying_negation() {
    let base = PredicateSignature::new(id("base"), vec![CanonicalType::Integer]);
    let forbidden = PredicateSignature::new(id("forbidden"), vec![CanonicalType::Integer]);
    let allowed = PredicateSignature::new(id("allowed"), vec![CanonicalType::Integer]);
    let x = Term::Variable(Variable::new(id("x"), CanonicalType::Integer));
    let app =
        |s: &PredicateSignature| PredicateApplication::new(s.clone(), vec![x.clone()]).unwrap();
    let fact = Fact::new(
        PredicateApplication::new(
            base.clone(),
            vec![Term::Constant(CanonicalValue::integer(1))],
        )
        .unwrap(),
    )
    .unwrap();
    let lower = DerivationRule::new(
        id("forbid"),
        app(&forbidden),
        vec![RuleLiteral::Positive(app(&base))],
    );
    let higher = DerivationRule::new(
        id("allow"),
        app(&allowed),
        vec![
            RuleLiteral::Positive(app(&base)),
            RuleLiteral::Negative(app(&forbidden)),
        ],
    );
    let program = LogicProgram::new(
        vec![base, forbidden, allowed.clone()],
        vec![fact],
        vec![higher, lower],
    )
    .unwrap();
    let db = DatalogEvaluator::evaluate(&program).unwrap();
    assert!(db.tuples(allowed.id()).is_none_or(|xs| xs.is_empty()));
}

#[test]
fn mutual_recursion_is_independent_of_rule_order() {
    let seed = PredicateSignature::new(id("seed"), vec![CanonicalType::Integer]);
    let p = PredicateSignature::new(id("p"), vec![CanonicalType::Integer]);
    let q = PredicateSignature::new(id("q"), vec![CanonicalType::Integer]);
    let x = Term::Variable(Variable::new(id("x"), CanonicalType::Integer));
    let app =
        |s: &PredicateSignature| PredicateApplication::new(s.clone(), vec![x.clone()]).unwrap();
    let fact = Fact::new(
        PredicateApplication::new(
            seed.clone(),
            vec![Term::Constant(CanonicalValue::integer(1))],
        )
        .unwrap(),
    )
    .unwrap();
    let rp = DerivationRule::new(id("p"), app(&p), vec![RuleLiteral::Positive(app(&seed))]);
    let rq = DerivationRule::new(id("q"), app(&q), vec![RuleLiteral::Positive(app(&p))]);
    let back = DerivationRule::new(id("back"), app(&p), vec![RuleLiteral::Positive(app(&q))]);
    let a = LogicProgram::new(
        vec![seed.clone(), p.clone(), q.clone()],
        vec![fact.clone()],
        vec![rp.clone(), rq.clone(), back.clone()],
    )
    .unwrap();
    let b = LogicProgram::new(
        vec![seed, p.clone(), q.clone()],
        vec![fact],
        vec![back, rq, rp],
    )
    .unwrap();
    let da = DatalogEvaluator::evaluate(&a).unwrap();
    let db = DatalogEvaluator::evaluate(&b).unwrap();
    assert_eq!(da, db);
    assert_eq!(da.tuples(q.id()).unwrap().len(), 1);
}

#[test]
fn body_filter_order_does_not_change_derivation() {
    let p = PredicateSignature::new(id("p"), vec![CanonicalType::Integer]);
    let q = PredicateSignature::new(id("q"), vec![CanonicalType::Integer]);
    let x = Term::Variable(Variable::new(id("x"), CanonicalType::Integer));
    let a = PredicateApplication::new(p.clone(), vec![x.clone()]).unwrap();
    let h = PredicateApplication::new(q.clone(), vec![x.clone()]).unwrap();
    let eq = Atom::equal(x.clone(), Term::Constant(CanonicalValue::integer(1))).unwrap();
    let rule = DerivationRule::new(
        id("r"),
        h,
        vec![RuleLiteral::Constraint(eq), RuleLiteral::Positive(a)],
    );
    let fact = Fact::new(
        PredicateApplication::new(p.clone(), vec![Term::Constant(CanonicalValue::integer(1))])
            .unwrap(),
    )
    .unwrap();
    let db = DatalogEvaluator::evaluate(
        &LogicProgram::new(vec![p, q.clone()], vec![fact], vec![rule]).unwrap(),
    )
    .unwrap();
    assert_eq!(db.tuples(q.id()).unwrap().len(), 1);
}

#[test]
fn duplicate_facts_and_rules_are_deduplicated() {
    let p = PredicateSignature::new(id("p"), vec![CanonicalType::Integer]);
    let q = PredicateSignature::new(id("q"), vec![CanonicalType::Integer]);
    let x = Term::Variable(Variable::new(id("x"), CanonicalType::Integer));
    let rule = DerivationRule::new(
        id("r"),
        PredicateApplication::new(q.clone(), vec![x.clone()]).unwrap(),
        vec![RuleLiteral::Positive(
            PredicateApplication::new(p.clone(), vec![x]).unwrap(),
        )],
    );
    let fact = Fact::new(
        PredicateApplication::new(p.clone(), vec![Term::Constant(CanonicalValue::integer(1))])
            .unwrap(),
    )
    .unwrap();
    let db = DatalogEvaluator::evaluate(
        &LogicProgram::new(
            vec![p, q.clone()],
            vec![fact.clone(), fact],
            vec![rule.clone(), rule],
        )
        .unwrap(),
    )
    .unwrap();
    assert_eq!(db.tuples(q.id()).unwrap().len(), 1);
}

#[test]
fn three_relation_join_excludes_nonmatching_tuples() {
    let a = PredicateSignature::new(id("a"), vec![CanonicalType::Integer]);
    let b = PredicateSignature::new(id("b"), vec![CanonicalType::Integer]);
    let c = PredicateSignature::new(id("c"), vec![CanonicalType::Integer]);
    let out = PredicateSignature::new(id("out"), vec![CanonicalType::Integer]);
    let x = Term::Variable(Variable::new(id("x"), CanonicalType::Integer));
    let app =
        |p: &PredicateSignature| PredicateApplication::new(p.clone(), vec![x.clone()]).unwrap();
    let rule = DerivationRule::new(
        id("join"),
        app(&out),
        vec![
            RuleLiteral::Positive(app(&a)),
            RuleLiteral::Positive(app(&b)),
            RuleLiteral::Positive(app(&c)),
        ],
    );
    let f = |p: &PredicateSignature, n| {
        Fact::new(
            PredicateApplication::new(p.clone(), vec![Term::Constant(CanonicalValue::integer(n))])
                .unwrap(),
        )
        .unwrap()
    };
    let db = DatalogEvaluator::evaluate(
        &LogicProgram::new(
            vec![a.clone(), b.clone(), c.clone(), out.clone()],
            vec![f(&a, 1), f(&a, 2), f(&b, 1), f(&c, 1), f(&c, 3)],
            vec![rule],
        )
        .unwrap(),
    )
    .unwrap();
    assert_eq!(db.tuples(out.id()).unwrap().len(), 1);
}

#[test]
fn unsafe_variables_include_rule_and_variable() {
    let p = PredicateSignature::new(id("p"), vec![CanonicalType::Integer]);
    let q = PredicateSignature::new(id("q"), vec![CanonicalType::Integer]);
    let x = Term::Variable(Variable::new(id("x"), CanonicalType::Integer));
    let rule = DerivationRule::new(
        id("unsafe"),
        PredicateApplication::new(q.clone(), vec![x.clone()]).unwrap(),
        vec![],
    );
    let error =
        DatalogEvaluator::evaluate(&LogicProgram::new(vec![p, q], vec![], vec![rule]).unwrap())
            .unwrap_err();
    assert!(
        matches!(error,rspdl_datalog::DatalogError::UnsafeVariable{rule,variable} if rule==id("unsafe")&&variable==id("x"))
    );
}

#[test]
fn membership_filters_bound_values_without_enumeration() {
    let p = PredicateSignature::new(id("p"), vec![CanonicalType::prime()]);
    let prime = PredicateSignature::new(id("prime_out"), vec![CanonicalType::prime()]);
    let x = Term::Variable(Variable::new(id("x"), CanonicalType::prime()));
    let rule = DerivationRule::new(
        id("prime_filter"),
        PredicateApplication::new(prime.clone(), vec![x.clone()]).unwrap(),
        vec![
            RuleLiteral::Positive(PredicateApplication::new(p.clone(), vec![x.clone()]).unwrap()),
            RuleLiteral::Constraint(
                Atom::member_of(x, SetExpression::domain(Domain::primes())).unwrap(),
            ),
        ],
    );
    let facts = [2, 5]
        .into_iter()
        .map(|n| {
            Fact::new(
                PredicateApplication::new(
                    p.clone(),
                    vec![Term::Constant(CanonicalValue::prime(n).unwrap())],
                )
                .unwrap(),
            )
            .unwrap()
        })
        .collect();
    let db = DatalogEvaluator::evaluate(
        &LogicProgram::new(vec![p, prime.clone()], facts, vec![rule]).unwrap(),
    )
    .unwrap();
    assert_eq!(db.tuples(prime.id()).unwrap().len(), 2);
    assert!(CanonicalValue::prime(4).is_err());
}

#[test]
fn integer_filters_select_exact_bound_tuples() {
    let p = PredicateSignature::new(id("numbers"), vec![CanonicalType::Integer]);
    let finite = PredicateSignature::new(id("finite_out"), vec![CanonicalType::Integer]);
    let all = PredicateSignature::new(id("all_out"), vec![CanonicalType::Integer]);
    let eq = PredicateSignature::new(id("eq_out"), vec![CanonicalType::Integer]);
    let x = Term::Variable(Variable::new(id("x"), CanonicalType::Integer));
    let make = |h: &PredicateSignature, set| {
        DerivationRule::new(
            id("filter"),
            PredicateApplication::new(h.clone(), vec![x.clone()]).unwrap(),
            vec![
                RuleLiteral::Positive(
                    PredicateApplication::new(p.clone(), vec![x.clone()]).unwrap(),
                ),
                RuleLiteral::Constraint(Atom::member_of(x.clone(), set).unwrap()),
            ],
        )
    };
    let f = |n| {
        Fact::new(
            PredicateApplication::new(p.clone(), vec![Term::Constant(CanonicalValue::integer(n))])
                .unwrap(),
        )
        .unwrap()
    };
    let rules = vec![
        make(
            &finite,
            SetExpression::literal(
                CanonicalType::Integer,
                [CanonicalValue::integer(1), CanonicalValue::integer(4)],
            )
            .unwrap(),
        ),
        make(&all, SetExpression::domain(Domain::integers())),
        make(
            &eq,
            SetExpression::literal(CanonicalType::Integer, [CanonicalValue::integer(2)]).unwrap(),
        ),
    ];
    let db = DatalogEvaluator::evaluate(
        &LogicProgram::new(
            vec![p.clone(), finite.clone(), all.clone(), eq.clone()],
            vec![f(1), f(2), f(4), f(5)],
            rules,
        )
        .unwrap(),
    )
    .unwrap();
    assert_eq!(db.tuples(finite.id()).unwrap().len(), 2);
    assert_eq!(db.tuples(all.id()).unwrap().len(), 4);
    assert_eq!(db.tuples(eq.id()).unwrap().len(), 1);
}
