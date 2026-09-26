use rspdl_compiler::{KoSource, compile_ko_files};

const POLICY: &str =
    include_str!("../../../conformance/ko-KR/source-provenance/normal-policy-lines/input.rspdl");
const LOOKUP: &str =
    include_str!("../../../conformance/ko-KR/action-outcomes/normal-lookup/input.rspdl");
const UNKNOWN_MODEL: &str =
    include_str!("../../../conformance/ko-KR/action-inputs/failure-unknown-model/input.rspdl");
const NO_REFERENCES: &str = r#"@모듈 재고(inventory)

재고 항목(item)은 다음 필드들로 구성되어 있다.
    이름(name): 필수 문자열
"#;

#[test]
fn normal_policy_references_are_typed_and_navigable() {
    let compilation = compile_ko_files(vec![KoSource::new("policy.rspdl", POLICY)]);
    assert!(!compilation.has_errors());

    let model_edges = compilation
        .references
        .iter()
        .filter(|reference| {
            reference.path == "policy.rspdl"
                && reference.from.kind == "policies"
                && reference.to.kind == "models"
                && reference.field == "model_id"
        })
        .collect::<Vec<_>>();
    assert_eq!(model_edges.len(), 2);
    assert!(
        model_edges
            .iter()
            .all(|edge| edge.to.id == "policy_locations.document")
    );
}

#[test]
fn failed_modules_do_not_invent_reference_edges() {
    let compilation = compile_ko_files(vec![KoSource::new("broken.rspdl", UNKNOWN_MODEL)]);
    assert!(compilation.has_errors());
    assert!(compilation.files[0].module.is_none());
    assert!(compilation.references.is_empty());
}

#[test]
fn local_ids_remain_distinct_across_workspace_files() {
    let second = LOOKUP.replacen("(booking)", "(second)", 1);
    let compilation = compile_ko_files(vec![
        KoSource::new("a.rspdl", LOOKUP),
        KoSource::new("b.rspdl", second),
    ]);
    assert!(!compilation.has_errors());

    let lookups = compilation
        .references
        .iter()
        .filter(|reference| reference.from.kind == "lookup_results")
        .map(|reference| {
            (
                reference.path.as_str(),
                reference.from.id.as_str(),
                reference.from.owner_id.as_deref(),
            )
        })
        .collect::<Vec<_>>();
    assert!(lookups.contains(&("a.rspdl", "reservation_lookup", Some("booking"))));
    assert!(lookups.contains(&("b.rspdl", "reservation_lookup", Some("second"))));
}

#[test]
fn declaration_ids_and_plain_strings_are_not_false_references() {
    let compilation = compile_ko_files(vec![KoSource::new("inventory.rspdl", NO_REFERENCES)]);
    assert!(!compilation.has_errors());
    assert!(compilation.references.is_empty());
}
