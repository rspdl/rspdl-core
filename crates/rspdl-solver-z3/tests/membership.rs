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
    assert!(matches!(
        Z3Solver::new().solve(&p, SolveOptions::default()).unwrap(),
        rspdl_core::SolveResult::Sat(_)
    ));
}
