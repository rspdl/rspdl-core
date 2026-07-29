use rspdl_core::{
    Atom, BooleanExpression, CanonicalId, CanonicalModel, CanonicalType, CanonicalValue,
    ComparisonOperator, ConstraintProblem, ConstraintSolver, Domain, EnumType, PredicateSignature,
    SolveOptions, SolveResult, SolverContractError, Term, Variable, VariableDomain,
};
use rspdl_solver_z3::Z3Solver;
use rspdl_solver_z3::Z3SolverError;
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
    let p =
        ConstraintProblem::new(vec![d.clone()], BooleanExpression::and([gt.clone(), lt])).unwrap();
    let SolveResult::Sat(CanonicalModel(model)) =
        Z3Solver::new().solve(&p, SolveOptions::default()).unwrap()
    else {
        panic!()
    };
    let value = model[&id("x")].as_integer().unwrap().as_bigint();
    assert!(value > &num_bigint::BigInt::from(10) && value < &num_bigint::BigInt::from(20));
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
    )
    .unwrap();
    assert!(matches!(
        Z3Solver::new()
            .solve(&bad, SolveOptions::default())
            .unwrap(),
        SolveResult::Unsat
    ));
}

#[test]
fn arbitrary_precision_integer_models_preserve_sign_and_magnitude() {
    for decimal in ["-1", "-9223372036854775809", "18446744073709551616"] {
        let variable = Variable::new(id("x"), CanonicalType::Integer);
        let expected = CanonicalValue::integer_from_decimal(decimal).unwrap();
        let problem = ConstraintProblem::new(
            vec![VariableDomain::new(id("x"), Domain::integers())],
            BooleanExpression::atom(
                Atom::equal(Term::Variable(variable), Term::Constant(expected.clone())).unwrap(),
            ),
        )
        .unwrap();

        let SolveResult::Sat(CanonicalModel(model)) = Z3Solver::new()
            .solve(&problem, SolveOptions::default())
            .unwrap()
        else {
            panic!("expected SAT for {decimal}")
        };
        assert_eq!(model.get(&id("x")), Some(&expected));
    }
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
    )
    .unwrap();
    let SolveResult::Sat(CanonicalModel(m)) =
        Z3Solver::new().solve(&p, SolveOptions::default()).unwrap()
    else {
        panic!()
    };
    assert_eq!(m.get(&id("s")), Some(&CanonicalValue::string("ok")));
    assert_eq!(m.get(&id("b")), Some(&CanonicalValue::boolean(true)));
}

#[test]
fn invalid_z3_string_constants_return_an_error() {
    let variable = Variable::new(id("s"), CanonicalType::String);
    let problem = ConstraintProblem::new(
        vec![VariableDomain::new(id("s"), Domain::strings())],
        BooleanExpression::atom(
            Atom::equal(
                Term::Variable(variable),
                Term::Constant(CanonicalValue::string("nul\0byte")),
            )
            .unwrap(),
        ),
    )
    .unwrap();

    assert!(matches!(
        Z3Solver::new().solve(&problem, SolveOptions::default()),
        Err(Z3SolverError::Unsupported(_))
    ));
}

#[test]
fn prime_and_predicate_are_unsupported() {
    let prime = ConstraintProblem::new(
        vec![VariableDomain::new(id("x"), Domain::primes())],
        BooleanExpression::literal(true),
    )
    .unwrap();
    assert!(matches!(
        Z3Solver::new().solve(&prime, SolveOptions::default()),
        Err(Z3SolverError::Unsupported(_))
    ));
    let sig = PredicateSignature::new(id("p"), vec![]);
    let pred = ConstraintProblem::new(
        vec![],
        BooleanExpression::atom(Atom::predicate(sig, vec![]).unwrap()),
    )
    .unwrap();
    assert!(matches!(
        Z3Solver::new().solve(&pred, SolveOptions::default()),
        Err(Z3SolverError::Unsupported(_))
    ));
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

#[test]
fn model_completion_returns_all_declared_types_in_key_order() {
    let e = EnumType::new(id("enum_type"), [id("a"), id("b")]).unwrap();
    let p = ConstraintProblem::new(
        vec![
            VariableDomain::new(
                id("z_bool"),
                Domain::finite(
                    CanonicalType::Boolean,
                    [
                        CanonicalValue::boolean(false),
                        CanonicalValue::boolean(true),
                    ],
                )
                .unwrap(),
            ),
            VariableDomain::new(id("a_int"), Domain::integers()),
            VariableDomain::new(id("m_string"), Domain::strings()),
            VariableDomain::new(
                id("n_enum"),
                Domain::finite(
                    CanonicalType::Enum(e.clone()),
                    [
                        CanonicalValue::enum_variant(e.clone(), id("a")).unwrap(),
                        CanonicalValue::enum_variant(e.clone(), id("b")).unwrap(),
                    ],
                )
                .unwrap(),
            ),
        ],
        BooleanExpression::literal(true),
    )
    .unwrap();
    let SolveResult::Sat(CanonicalModel(model)) =
        Z3Solver::new().solve(&p, SolveOptions::default()).unwrap()
    else {
        panic!()
    };
    let keys: Vec<_> = model.keys().cloned().collect();
    assert_eq!(
        keys,
        vec![id("a_int"), id("m_string"), id("n_enum"), id("z_bool")]
    );
    assert_eq!(model.len(), 4);
}

#[test]
fn declaration_validation_is_structured() {
    let duplicate = ConstraintProblem::new(
        vec![
            VariableDomain::new(id("x"), Domain::integers()),
            VariableDomain::new(id("x"), Domain::integers()),
        ],
        BooleanExpression::literal(true),
    );
    assert!(matches!(
        duplicate,
        Err(SolverContractError::DuplicateVariable(ref x)) if x == &id("x")
    ));
    let b = Variable::new(id("x"), CanonicalType::Boolean);
    let mismatch = ConstraintProblem::new(
        vec![VariableDomain::new(id("x"), Domain::integers())],
        BooleanExpression::atom(
            Atom::equal(
                Term::Variable(b),
                Term::Constant(CanonicalValue::boolean(true)),
            )
            .unwrap(),
        ),
    )
    .unwrap();
    assert!(matches!(
        Z3Solver::new().solve(&mismatch, SolveOptions::default()),
        Err(Z3SolverError::VariableTypeMismatch {
            declared: CanonicalType::Integer,
            actual: CanonicalType::Boolean,
            ..
        })
    ));
}
