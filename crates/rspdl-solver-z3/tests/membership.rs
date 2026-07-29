use rspdl_core::{
    Atom, BooleanExpression, CanonicalId, CanonicalType, CanonicalValue, ConstraintProblem,
    ConstraintSolver, Domain, SetExpression, SolveOptions, Term, Variable, VariableDomain,
};
use rspdl_solver_z3::Z3Solver;
fn id(s: &str) -> CanonicalId {
    CanonicalId::new(s).unwrap()
}
#[test]
fn finite_and_composite_membership_is_solved() {
    let x = Variable::new(id("x"), CanonicalType::Integer);
    let set = SetExpression::difference(
        SetExpression::union([
            SetExpression::literal(
                CanonicalType::Integer,
                [CanonicalValue::integer(1), CanonicalValue::integer(2)],
            )
            .unwrap(),
            SetExpression::literal(CanonicalType::Integer, [CanonicalValue::integer(3)]).unwrap(),
        ])
        .unwrap(),
        SetExpression::literal(CanonicalType::Integer, [CanonicalValue::integer(2)]).unwrap(),
    )
    .unwrap();
    let p = ConstraintProblem::new(
        vec![VariableDomain::new(
            id("x"),
            Domain::finite(
                CanonicalType::Integer,
                [
                    CanonicalValue::integer(1),
                    CanonicalValue::integer(2),
                    CanonicalValue::integer(3),
                ],
            )
            .unwrap(),
        )],
        BooleanExpression::atom(Atom::member_of(Term::Variable(x), set).unwrap()),
    )
    .unwrap();
    let rspdl_core::SolveResult::Sat(rspdl_core::CanonicalModel(values)) =
        Z3Solver::new().solve(&p, SolveOptions::default()).unwrap()
    else {
        panic!("expected SAT")
    };
    assert!(matches!(
        values.get(&id("x")),
        Some(value)
            if value == &CanonicalValue::integer(1)
                || value == &CanonicalValue::integer(3)
    ));
}
