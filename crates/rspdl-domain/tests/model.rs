use rspdl_domain::{
    Atom, Backend, BooleanExpression, CanonicalId, CanonicalType, CanonicalValue, Cardinality,
    CurrencyCode, Domain, EnumType, EnumerationSupport, ModelError, PredicateSignature,
    QuantityDimension, SetExpression, SymbolicSupport, Term, Variable,
};

fn id(value: &str) -> CanonicalId {
    CanonicalId::new(value).expect("test IDs are canonical")
}

#[test]
fn parameterized_money_and_quantity_preserve_identity_and_exact_operations() {
    let won = CanonicalValue::money_from_str("10000 KRW").unwrap();
    let more_won = CanonicalValue::money_from_str("500 KRW").unwrap();
    assert_eq!(
        won.add(&more_won).unwrap().value_type(),
        &CanonicalType::Money(CurrencyCode::new("KRW").unwrap())
    );
    assert!(matches!(
        won.add(&CanonicalValue::money_from_str("10 USD").unwrap()),
        Err(ModelError::TypeMismatch { .. })
    ));

    let kilograms = CanonicalValue::quantity_from_str("1 kg").unwrap();
    let grams = CanonicalValue::quantity_from_str("500 g").unwrap();
    assert_eq!(
        CanonicalValue::quantity_from_str("1000 g")
            .unwrap()
            .compare_ordered(&kilograms)
            .unwrap(),
        std::cmp::Ordering::Equal
    );
    assert_eq!(
        kilograms
            .add(&grams)
            .unwrap()
            .compare_ordered(&CanonicalValue::quantity_from_str("1.5 kg").unwrap())
            .unwrap(),
        std::cmp::Ordering::Equal
    );
    assert!(matches!(
        kilograms.compare_ordered(&CanonicalValue::quantity_from_str("1 km").unwrap()),
        Err(ModelError::TypeMismatch { .. })
    ));
    assert_eq!(
        kilograms.value_type(),
        &CanonicalType::Quantity(QuantityDimension::Mass)
    );
}

#[test]
fn refinements_and_coordinates_reject_malformed_and_boundary_values() {
    assert_eq!(
        CanonicalValue::coordinate_from_str("90,-180")
            .unwrap()
            .value_type(),
        &CanonicalType::Coordinate
    );
    assert!(CanonicalValue::coordinate_from_str("90.1,0").is_err());
    assert!(
        CanonicalValue::refinement_from_str(
            CanonicalType::Uuid,
            "550e8400-e29b-41d4-a716-446655440000"
        )
        .is_ok()
    );
    assert!(CanonicalValue::refinement_from_str(CanonicalType::Email, "not-an-email").is_err());
}

#[test]
fn temporal_spatial_and_collection_contracts_are_deterministic() {
    let local = CanonicalValue::local_date_time_from_iso("2026-08-13T14:30:00").unwrap();
    assert_eq!(local.canonical_text(), "2026-08-13T14:30:00");
    assert!(CanonicalValue::local_date_time_from_iso("2026-08-13T14:30:00Z").is_err());
    let early =
        CanonicalValue::zoned_date_time_from_str("2026-11-01T01:30:00-04:00 America/New_York")
            .unwrap();
    let late =
        CanonicalValue::zoned_date_time_from_str("2026-11-01T01:30:00-05:00 America/New_York")
            .unwrap();
    assert_ne!(early.canonical_text(), late.canonical_text());
    assert_eq!(
        early.compare_ordered(&late).unwrap(),
        std::cmp::Ordering::Less
    );
    assert!(
        CanonicalValue::zoned_date_time_from_str("2026-03-08T02:30:00-05:00 America/New_York")
            .is_err()
    );
    let duration = CanonicalValue::calendar_duration_from_iso("P1M").unwrap();
    assert!(
        CanonicalValue::date_from_iso("2026-01-31")
            .unwrap()
            .apply_calendar_duration_to_date(&duration)
            .is_err()
    );
    let extreme = CanonicalValue::calendar_duration_from_iso("P2147483647Y").unwrap();
    assert!(matches!(
        CanonicalValue::date_from_iso("2026-01-01")
            .unwrap()
            .apply_calendar_duration_to_date(&extreme),
        Err(ModelError::CalendarDateOverflow { .. })
    ));

    let seoul = CanonicalValue::coordinate_from_str("37.5665,126.9780").unwrap();
    assert!(
        seoul
            .is_within_radius(&seoul, &CanonicalValue::quantity_from_str("0 m").unwrap())
            .unwrap()
    );
    assert!(matches!(
        seoul.is_within_radius(&seoul, &CanonicalValue::quantity_from_str("-1 m").unwrap()),
        Err(ModelError::InvalidRadius)
    ));
    assert_eq!(
        CanonicalValue::coordinate_from_str("0,0")
            .unwrap()
            .distance_to(&CanonicalValue::coordinate_from_str("0,1").unwrap())
            .unwrap()
            .canonical_text(),
        "111195.080233533 m"
    );
    let antipodal_distance = CanonicalValue::coordinate_from_str("0,0")
        .unwrap()
        .distance_to(&CanonicalValue::coordinate_from_str("0,180").unwrap())
        .unwrap();
    assert_eq!(
        antipodal_distance
            .compare_ordered(&CanonicalValue::quantity_from_str("20015114 m").unwrap())
            .unwrap(),
        std::cmp::Ordering::Greater
    );
    let cidr = CanonicalValue::refinement_from_str(CanonicalType::Cidr, "192.0.2.15/24").unwrap();
    assert_eq!(cidr.canonical_text(), "192.0.2.0/24");
    assert!(
        cidr.cidr_contains(
            &CanonicalValue::refinement_from_str(CanonicalType::IpAddress, "192.0.2.99").unwrap()
        )
        .unwrap()
    );

    let list =
        CanonicalValue::list(CanonicalType::String, vec![CanonicalValue::string("x")]).unwrap();
    assert!(list.list_contains(&CanonicalValue::string("x")).unwrap());
    assert!(
        CanonicalValue::set(
            CanonicalType::String,
            [CanonicalValue::string("x"), CanonicalValue::string("x")]
        )
        .is_err()
    );
    assert!(matches!(
        CanonicalType::map(CanonicalType::Integer, CanonicalType::String),
        Err(ModelError::UnsupportedMapKeyType { .. })
    ));
    let reference = CanonicalValue::reference(id("model.payment"), "p-1").unwrap();
    assert_ne!(
        reference.value_type(),
        CanonicalValue::reference(id("model.customer"), "p-1")
            .unwrap()
            .value_type()
    );
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

#[test]
fn extended_scalars_validate_and_normalize_canonical_values() {
    let decimal = CanonicalValue::decimal_from_str("42.5000").unwrap();
    assert_eq!(decimal.as_decimal().unwrap().to_string(), "42.5");
    assert!(CanonicalValue::decimal_from_str("1e3").is_err());
    assert!(CanonicalValue::decimal_from_str("01.5").is_err());

    assert!(CanonicalValue::date_from_iso("2024-02-29").is_ok());
    assert!(CanonicalValue::date_from_iso("2026-02-29").is_err());
    assert!(CanonicalValue::time_from_iso("23:59:59.999999999").is_ok());
    assert!(CanonicalValue::time_from_iso("24:00:00").is_err());

    let utc = CanonicalValue::date_time_from_rfc3339("2026-08-13T05:30:00Z").unwrap();
    let offset = CanonicalValue::date_time_from_rfc3339("2026-08-13T14:30:00+09:00").unwrap();
    assert_eq!(utc, offset);
    assert_eq!(
        offset.as_date_time().unwrap().to_string(),
        "2026-08-13T05:30:00Z"
    );
    let serialized = serde_json::to_value(&offset).unwrap();
    assert_eq!(serialized["value_type"]["kind"], "date_time");
    assert_eq!(serialized["representation"]["kind"], "date_time");
    assert_eq!(
        serialized["representation"]["value"],
        "2026-08-13T05:30:00Z"
    );

    let duration = CanonicalValue::duration_from_iso("-PT1.500000000S").unwrap();
    assert_eq!(duration.as_duration().unwrap().to_string(), "-PT1.5S");
    assert!(CanonicalValue::duration_from_iso("P1D").is_err());

    assert!(CanonicalValue::latitude_from_decimal("-90").is_ok());
    assert!(CanonicalValue::latitude_from_decimal("90.0001").is_err());
    assert!(CanonicalValue::longitude_from_decimal("180").is_ok());
    assert!(CanonicalValue::longitude_from_decimal("-180.0001").is_err());
}

#[test]
fn ordered_comparison_accepts_only_ordered_values_of_the_same_type() {
    let earlier = CanonicalValue::date_from_iso("2026-08-12").unwrap();
    let later = CanonicalValue::date_from_iso("2026-08-13").unwrap();
    assert_eq!(
        earlier.compare_ordered(&later).unwrap(),
        std::cmp::Ordering::Less
    );
    Atom::ordered_comparison(
        rspdl_domain::ComparisonOperator::Lt,
        Term::Constant(earlier),
        Term::Constant(later),
    )
    .expect("date is ordered");

    assert!(matches!(
        Atom::ordered_comparison(
            rspdl_domain::ComparisonOperator::Lt,
            Term::Constant(CanonicalValue::string("a")),
            Term::Constant(CanonicalValue::string("b")),
        ),
        Err(ModelError::UnsupportedOperation { .. })
    ));
    assert!(matches!(
        CanonicalValue::latitude_from_decimal("1")
            .unwrap()
            .compare_ordered(&CanonicalValue::longitude_from_decimal("1").unwrap()),
        Err(ModelError::TypeMismatch { .. })
    ));
}
