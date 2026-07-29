use rspdl_core::{
    Atom, BooleanExpression, CanonicalId, CanonicalType, CanonicalValue, ConstraintProblem,
    ConstraintSolver, Domain, EnumType, SolveOptions, Term, Variable, VariableDomain,
};
use rspdl_solver_z3::Z3Solver;
fn id(x: &str) -> CanonicalId {
    CanonicalId::new(x).unwrap()
}

#[test]
fn enum_symbols_are_collision_free() {
    let one = EnumType::new(id("a_b"), [id("c")]).unwrap();
    let two = EnumType::new(id("a"), [id("b_c")]).unwrap();
    let x = Variable::new(id("x"), CanonicalType::Enum(one.clone()));
    let y = Variable::new(id("y"), CanonicalType::Enum(two.clone()));
    let xv = CanonicalValue::enum_variant(one.clone(), id("c")).unwrap();
    let yv = CanonicalValue::enum_variant(two.clone(), id("b_c")).unwrap();
    let p = ConstraintProblem::new(
        vec![
            VariableDomain::new(
                id("x"),
                Domain::finite(CanonicalType::Enum(one), [xv.clone()]).unwrap(),
            ),
            VariableDomain::new(
                id("y"),
                Domain::finite(CanonicalType::Enum(two), [yv.clone()]).unwrap(),
            ),
        ],
        BooleanExpression::and([
            BooleanExpression::atom(Atom::equal(Term::Variable(x), Term::Constant(xv)).unwrap()),
            BooleanExpression::atom(Atom::equal(Term::Variable(y), Term::Constant(yv)).unwrap()),
        ]),
    )
    .unwrap();
    assert!(matches!(
        Z3Solver::new().solve(&p, SolveOptions::default()).unwrap(),
        rspdl_core::SolveResult::Sat(_)
    ));
}

#[test]
fn constant_only_enum_equality_is_exact() {
    let e = EnumType::new(id("constant_role"), [id("admin"), id("user")]).unwrap();
    let a = CanonicalValue::enum_variant(e.clone(), id("admin")).unwrap();
    let u = CanonicalValue::enum_variant(e, id("user")).unwrap();
    let solve = |x, y| {
        Z3Solver::new()
            .solve(
                &ConstraintProblem::new(
                    vec![],
                    BooleanExpression::atom(
                        Atom::equal(Term::Constant(x), Term::Constant(y)).unwrap(),
                    ),
                )
                .unwrap(),
                SolveOptions::default(),
            )
            .unwrap()
    };
    assert!(matches!(
        solve(a.clone(), a),
        rspdl_core::SolveResult::Sat(_)
    ));
    assert!(matches!(
        solve(
            u.clone(),
            CanonicalValue::enum_variant(
                EnumType::new(id("constant_role"), [id("admin"), id("user")]).unwrap(),
                id("admin")
            )
            .unwrap()
        ),
        rspdl_core::SolveResult::Unsat
    ));
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
    )
    .unwrap();
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
    )
    .unwrap();
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
