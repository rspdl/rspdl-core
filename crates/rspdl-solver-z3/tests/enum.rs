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

#[test]
fn enum_distinct_equalities_are_unsat_and_reusable() {
    let e = EnumType::new(id("kind"), [id("a"), id("b")]).unwrap();
    let v = Variable::new(id("kind"), CanonicalType::Enum(e.clone()));
    let a = CanonicalValue::enum_variant(e.clone(), id("a")).unwrap();
    let b = CanonicalValue::enum_variant(e.clone(), id("b")).unwrap();
    let p = ConstraintProblem::new(
        vec![VariableDomain::new(
            id("kind"),
            Domain::finite(CanonicalType::Enum(e), [a.clone(), b.clone()]).unwrap(),
        )],
        BooleanExpression::and([
            BooleanExpression::atom(
                Atom::equal(Term::Variable(v.clone()), Term::Constant(a)).unwrap(),
            ),
            BooleanExpression::atom(Atom::equal(Term::Variable(v), Term::Constant(b)).unwrap()),
        ]),
    );
    let s = Z3Solver::new();
    assert!(matches!(
        s.solve(&p, SolveOptions::default()).unwrap(),
        rspdl_core::SolveResult::Unsat
    ));
    assert!(matches!(
        s.solve(&p, SolveOptions::default()).unwrap(),
        rspdl_core::SolveResult::Unsat
    ));
}
