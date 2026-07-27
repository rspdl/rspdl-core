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
    let db = DatalogEvaluator::evaluate(&p).unwrap();
    assert_eq!(db.tuples(path.id()).unwrap().len(), 3);
}
