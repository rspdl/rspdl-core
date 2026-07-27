use rspdl_core::{
    Atom, BooleanExpression, CanonicalId, CanonicalType, CanonicalValue, ConstraintProblem,
    ConstraintSolver, Domain, EnumType, SolveOptions, Term, Variable, VariableDomain,
};
use rspdl_solver_z3::Z3Solver;
fn id(x: &str) -> CanonicalId {
    CanonicalId::new(x).unwrap()
}
#[test]
fn enum_sat_returns_canonical_variant() {
    let e = EnumType::new(id("role"), [id("admin"), id("user")]).unwrap();
    let v = Variable::new(id("role"), CanonicalType::Enum(e.clone()));
    let admin = CanonicalValue::enum_variant(e.clone(), id("admin")).unwrap();
    let p = ConstraintProblem::new(
        vec![VariableDomain::new(
            id("role"),
            Domain::finite(CanonicalType::Enum(e), [admin.clone()]).unwrap(),
        )],
        BooleanExpression::atom(
            Atom::equal(Term::Variable(v), Term::Constant(admin.clone())).unwrap(),
        ),
    );
    let rspdl_core::SolveResult::Sat(rspdl_core::CanonicalModel(values)) =
        Z3Solver::new().solve(&p, SolveOptions::default()).unwrap()
    else {
        panic!("expected SAT")
    };
    assert_eq!(values.get(&id("role")), Some(&admin));
}
