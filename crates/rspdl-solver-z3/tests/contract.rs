use rspdl_core::*;
use rspdl_solver_z3::Z3Solver;
fn id(x: &str) -> CanonicalId {
    CanonicalId::new(x).unwrap()
}
#[test]
fn unbounded_integer_sat_and_unsat() {
    let x = Variable::new(id("x"), CanonicalType::Integer);
    let d = VariableDomain::new(id("x"), Domain::integers());
    let gt = BooleanExpression::atom(
        Atom::integer_comparison(
            ComparisonOperator::Gt,
            Term::Variable(x.clone()),
            Term::Constant(CanonicalValue::integer(10)),
        )
        .unwrap(),
    );
    let lt = BooleanExpression::atom(
        Atom::integer_comparison(
            ComparisonOperator::Lt,
            Term::Variable(x.clone()),
            Term::Constant(CanonicalValue::integer(20)),
        )
        .unwrap(),
    );
    let p = ConstraintProblem::new(vec![d.clone()], BooleanExpression::and([gt.clone(), lt]));
    assert!(matches!(
        Z3Solver::new().solve(&p, SolveOptions::default()).unwrap(),
        SolveResult::Sat(_)
    ));
    let bad = ConstraintProblem::new(
        vec![d],
        BooleanExpression::and([
            gt,
            BooleanExpression::atom(
                Atom::integer_comparison(
                    ComparisonOperator::Lt,
                    Term::Variable(x),
                    Term::Constant(CanonicalValue::integer(5)),
                )
                .unwrap(),
            ),
        ]),
    );
    assert!(matches!(
        Z3Solver::new()
            .solve(&bad, SolveOptions::default())
            .unwrap(),
        SolveResult::Unsat
    ));
}

#[test]
fn string_and_boolean_models_are_exact() {
    let s = Variable::new(id("s"), CanonicalType::String);
    let b = Variable::new(id("b"), CanonicalType::Boolean);
    let p = ConstraintProblem::new(
        vec![
            VariableDomain::new(id("s"), Domain::strings()),
            VariableDomain::new(
                id("b"),
                Domain::finite(
                    CanonicalType::Boolean,
                    [
                        CanonicalValue::boolean(true),
                        CanonicalValue::boolean(false),
                    ],
                )
                .unwrap(),
            ),
        ],
        BooleanExpression::and([
            BooleanExpression::atom(
                Atom::equal(
                    Term::Variable(s),
                    Term::Constant(CanonicalValue::string("ok")),
                )
                .unwrap(),
            ),
            BooleanExpression::atom(
                Atom::equal(
                    Term::Variable(b),
                    Term::Constant(CanonicalValue::boolean(true)),
                )
                .unwrap(),
            ),
        ]),
    );
    let SolveResult::Sat(CanonicalModel(m)) =
        Z3Solver::new().solve(&p, SolveOptions::default()).unwrap()
    else {
        panic!()
    };
    assert_eq!(m.get(&id("s")), Some(&CanonicalValue::string("ok")));
    assert_eq!(m.get(&id("b")), Some(&CanonicalValue::boolean(true)));
}

#[test]
fn prime_and_predicate_are_unsupported() {
    let prime = ConstraintProblem::new(
        vec![VariableDomain::new(id("x"), Domain::primes())],
        BooleanExpression::literal(true),
    );
    assert!(
        Z3Solver::new()
            .solve(&prime, SolveOptions::default())
            .is_err()
    );
    let sig = PredicateSignature::new(id("p"), vec![]);
    let pred = ConstraintProblem::new(
        vec![],
        BooleanExpression::atom(Atom::predicate(sig, vec![]).unwrap()),
    );
    assert!(
        Z3Solver::new()
            .solve(&pred, SolveOptions::default())
            .is_err()
    );
}

#[test]
fn options_contract() {
    assert_eq!(SolveOptions::default().timeout().as_millis(), 5000);
    assert!(SolveOptions::with_timeout(std::time::Duration::ZERO).is_err());
    assert_eq!(
        SolveOptions::with_timeout(std::time::Duration::from_millis(7))
            .unwrap()
            .timeout()
            .as_millis(),
        7
    );
}
