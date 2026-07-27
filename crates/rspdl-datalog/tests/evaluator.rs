use rspdl_core::{
    CanonicalId, CanonicalType, CanonicalValue, DerivationRule, Fact, LogicProgram,
    PredicateApplication, PredicateSignature, RuleLiteral, Term, Variable,
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
        Fact::new(app(&edge, vec![c(1), c(2)])),
        Fact::new(app(&edge, vec![c(2), c(3)])),
    ];
    let r1 = DerivationRule::new(
        id("base"),
        app(&path, vec![x.clone(), y.clone()]),
        vec![RuleLiteral::Positive(app(
            &edge,
            vec![x.clone(), y.clone()],
        ))],
    )
    .unwrap();
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
    )
    .unwrap();
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
    )
    .unwrap();
    let r2 =
        DerivationRule::new(id("q_rule"), app(&q), vec![RuleLiteral::Positive(app(&p))]).unwrap();
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
    );
    let lower = DerivationRule::new(
        id("forbid"),
        app(&forbidden),
        vec![RuleLiteral::Positive(app(&base))],
    )
    .unwrap();
    let higher = DerivationRule::new(
        id("allow"),
        app(&allowed),
        vec![
            RuleLiteral::Positive(app(&base)),
            RuleLiteral::Negative(app(&forbidden)),
        ],
    )
    .unwrap();
    let program = LogicProgram::new(
        vec![base, forbidden, allowed.clone()],
        vec![fact],
        vec![higher, lower],
    )
    .unwrap();
    let db = DatalogEvaluator::evaluate(&program).unwrap();
    assert!(db.tuples(allowed.id()).is_none_or(|xs| xs.is_empty()));
}
