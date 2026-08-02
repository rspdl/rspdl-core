use rspdl_domain::{
    Atom, Backend, BooleanExpression, CanonicalId, CanonicalType, CanonicalValue, Cardinality,
    Domain, EnumType, EnumerationSupport, ModelError, PredicateSignature, SetExpression,
    SymbolicSupport, Term, Variable,
};

fn id(value: &str) -> CanonicalId {
    CanonicalId::new(value).expect("test IDs are canonical")
}

#[test]
fn canonical_ids_reject_locale_labels_and_ambiguous_forms() {
    for invalid in ["", "Actor.Admin", "actor..admin", "관리자", "_actor"] {
        assert!(matches!(
            CanonicalId::new(invalid),
            Err(ModelError::InvalidCanonicalId { .. })
        ));
    }
    assert_eq!(
        CanonicalId::new("actor.accounting_manager")
            .expect("valid ID")
            .as_str(),
        "actor.accounting_manager"
    );
}

#[test]
fn finite_domains_are_sorted_deduplicated_and_type_checked() {
    let domain = Domain::finite(
        CanonicalType::Integer,
        [
            CanonicalValue::integer(3_i64),
            CanonicalValue::integer(1_i64),
            CanonicalValue::integer(3_i64),
        ],
    )
    .expect("homogeneous finite domain");

    let values = domain.finite_values().expect("finite values");
    assert_eq!(values.len(), 2);
    assert_eq!(domain.capabilities().cardinality, Cardinality::Finite(2));
    assert_eq!(domain.capabilities().enumeration, EnumerationSupport::Exact);
    assert!(domain.contains(&CanonicalValue::integer(1_i64)).unwrap());
    assert!(!domain.contains(&CanonicalValue::integer(2_i64)).unwrap());

    let error = Domain::finite(
        CanonicalType::Integer,
        [CanonicalValue::string("not an integer")],
    )
    .expect_err("mixed types must fail");
    assert!(matches!(error, ModelError::TypeMismatch { .. }));
}

#[test]
fn enum_values_are_closed_and_canonical() {
    let status = EnumType::new(id("type.expense_status"), [id("submitted"), id("draft")])
        .expect("non-empty enum");
    assert_eq!(
        status
            .variants()
            .iter()
            .map(CanonicalId::as_str)
            .collect::<Vec<_>>(),
        vec!["draft", "submitted"]
    );
    assert!(CanonicalValue::enum_variant(status.clone(), id("submitted")).is_ok());
    assert!(matches!(
        CanonicalValue::enum_variant(status, id("approved")),
        Err(ModelError::UnknownEnumVariant { .. })
    ));
}

#[test]
fn enum_types_reject_duplicate_variants() {
    assert!(matches!(
        EnumType::new(
            id("type.expense_status"),
            [id("submitted"), id("submitted")]
        ),
        Err(ModelError::DuplicateEnumVariant { .. })
    ));
}

#[test]
fn infinite_domains_expose_exact_ground_semantics_and_backend_limits() {
    let integers = Domain::integers();
    assert!(
        integers
            .contains(&CanonicalValue::integer(-42_i64))
            .unwrap()
    );
    assert_eq!(
        integers.symbolic_support(Backend::Datalog),
        SymbolicSupport::RequiresFiniteGrounding
    );
    assert_eq!(
        integers.symbolic_support(Backend::Smt),
        SymbolicSupport::Exact
    );

    let primes = Domain::primes();
    let prime = CanonicalValue::prime(104_729_i64).expect("known prime");
    assert!(primes.contains(&prime).unwrap());
    assert!(CanonicalValue::prime(104_730_i64).is_err());
    assert_eq!(
        primes.symbolic_support(Backend::Smt),
        SymbolicSupport::Unsupported
    );
}

#[test]
fn set_algebra_is_typed_and_ground_membership_is_computable() {
    let left = SetExpression::literal(
        CanonicalType::Integer,
        [
            CanonicalValue::integer(1_i64),
            CanonicalValue::integer(2_i64),
        ],
    )
    .unwrap();
    let right = SetExpression::literal(
        CanonicalType::Integer,
        [
            CanonicalValue::integer(2_i64),
            CanonicalValue::integer(3_i64),
        ],
    )
    .unwrap();

    let union = SetExpression::union([left.clone(), right.clone()]).unwrap();
    let intersection = SetExpression::intersection([left.clone(), right.clone()]).unwrap();
    let difference = SetExpression::difference(left, right).unwrap();

    assert!(union.contains(&CanonicalValue::integer(3_i64)).unwrap());
    assert!(
        intersection
            .contains(&CanonicalValue::integer(2_i64))
            .unwrap()
    );
    assert!(
        !intersection
            .contains(&CanonicalValue::integer(1_i64))
            .unwrap()
    );
    assert!(
        difference
            .contains(&CanonicalValue::integer(1_i64))
            .unwrap()
    );
    assert!(
        !difference
            .contains(&CanonicalValue::integer(2_i64))
            .unwrap()
    );

    let strings =
        SetExpression::literal(CanonicalType::String, [CanonicalValue::string("one")]).unwrap();
    assert!(matches!(
        SetExpression::union([union, strings]),
        Err(ModelError::TypeMismatch { .. })
    ));
}

#[test]
fn commutative_expressions_have_one_canonical_serialization() {
    let one =
        SetExpression::literal(CanonicalType::Integer, [CanonicalValue::integer(1_i64)]).unwrap();
    let two =
        SetExpression::literal(CanonicalType::Integer, [CanonicalValue::integer(2_i64)]).unwrap();

    let first = SetExpression::union([one.clone(), two.clone(), one.clone()]).unwrap();
    let second =
        SetExpression::union([SetExpression::union([two, one]).unwrap(), first.clone()]).unwrap();

    assert_eq!(first, second);
    assert_eq!(
        serde_json::to_vec(&first).unwrap(),
        serde_json::to_vec(&second).unwrap()
    );
}

#[test]
fn logical_atoms_reject_type_and_arity_errors() {
    let subject = Term::Variable(Variable::new(id("subject"), CanonicalType::Integer));
    let one = Term::Constant(CanonicalValue::integer(1_i64));
    let text = Term::Constant(CanonicalValue::string("one"));

    Atom::equal(subject.clone(), one.clone()).expect("same type");
    assert!(matches!(
        Atom::equal(subject.clone(), text),
        Err(ModelError::TypeMismatch { .. })
    ));

    let signature = PredicateSignature::new(
        id("policy.can_approve"),
        vec![CanonicalType::Integer, CanonicalType::Integer],
    );
    assert!(matches!(
        Atom::predicate(signature.clone(), vec![subject.clone()]),
        Err(ModelError::ArityMismatch { .. })
    ));
    let atom =
        Atom::predicate(signature, vec![subject, one]).expect("matching predicate arguments");

    let expression = BooleanExpression::and([
        BooleanExpression::literal(true),
        BooleanExpression::atom(atom.clone()),
        BooleanExpression::atom(atom),
    ]);
    let expected = BooleanExpression::atom(
        Atom::predicate(
            PredicateSignature::new(
                id("policy.can_approve"),
                vec![CanonicalType::Integer, CanonicalType::Integer],
            ),
            vec![
                Term::Variable(Variable::new(id("subject"), CanonicalType::Integer)),
                Term::Constant(CanonicalValue::integer(1_i64)),
            ],
        )
        .unwrap(),
    );
    assert_eq!(expression, expected);
}

#[test]
fn equality_has_a_canonical_operand_order() {
    let variable = Term::Variable(Variable::new(id("subject"), CanonicalType::Integer));
    let constant = Term::Constant(CanonicalValue::integer(1_i64));

    let forward = Atom::equal(variable.clone(), constant.clone()).unwrap();
    let reverse = Atom::equal(constant, variable).unwrap();

    assert_eq!(forward, reverse);
    assert_eq!(
        serde_json::to_vec(&forward).unwrap(),
        serde_json::to_vec(&reverse).unwrap()
    );
}

#[test]
fn logical_serialization_exposes_the_tagged_expression_directly() {
    let atom = Atom::equal(
        Term::Constant(CanonicalValue::integer(1)),
        Term::Constant(CanonicalValue::integer(1)),
    )
    .unwrap();
    let atom_json = serde_json::to_value(&atom).unwrap();
    assert_eq!(atom_json["kind"], "equal");
    assert!(atom_json.get("atom").is_none());

    let expression_json = serde_json::to_value(BooleanExpression::atom(atom)).unwrap();
    assert_eq!(expression_json["kind"], "atom");
    assert!(expression_json.get("expression").is_none());
}

#[test]
fn canonical_integer_text_has_one_representation() {
    assert!(CanonicalValue::integer_from_decimal("42").is_ok());
    for invalid in ["+42", "042", "-0", " 42"] {
        assert!(
            matches!(
                CanonicalValue::integer_from_decimal(invalid),
                Err(ModelError::InvalidInteger { .. })
            ),
            "{invalid}"
        );
    }
}
