use rspdl_compiler::compile_ko;
use rspdl_domain::semantic_references;

/// Conditions and assignments must expose the same public operand reference shape.
#[test]
fn direct_inputs_in_conditions_and_assignments_have_operand_dependencies() {
    let source = r#"@모듈 참조(references)
상태(status)는 다음 값 중 하나다.
    대기(waiting)
    완료(done)
기록(record)은 다음 필드들로 구성되어 있다.
    상태(status): 필수 상태
변경(change)은 행동이다.
변경은 기존 기록을 대상 기록(target_record)으로 입력받는다.
변경은 상태를 새 상태(new_status)로 입력받는다.
처리(process)는 다음과 같이 정한다.
    변경이 실행될 때 적용한다.
    시스템이 수행한다.
    대상(target)은 변경의 대상 기록 입력을 사용한다.
    새 값(value)은 변경의 새 상태 입력을 사용한다.
    다음 조건을 모두 만족할 때 적용한다.
        새 값이 대기와 같다.
    이 처리를 할 수 있다.
    대상을 수정한다.
        상태를 새 값으로 정한다.
"#;
    let compilation = compile_ko(source);
    assert!(
        compilation.diagnostics.is_empty(),
        "{:?}",
        compilation.diagnostics
    );
    let module = compilation.module.unwrap();
    let references = semantic_references(&module);
    let bindings = references
        .iter()
        .filter(|reference| {
            reference.from.kind == "statements"
                && reference.to.kind == "statements.bindings"
                && reference.to.id == "value"
        })
        .collect::<Vec<_>>();
    assert_eq!(bindings.len(), 2);
    for reference in &bindings {
        assert_eq!(reference.field, "operand.binding_id");
        assert_eq!(reference.to.owner_id.as_deref(), Some("references.process"));
        assert!(references.iter().any(|type_ref| {
            type_ref.from == reference.from
                && type_ref.to.kind == "enums"
                && type_ref.to.id == "references.status"
                && type_ref.field == "operand.value_type"
                && type_ref.span == reference.span
        }));
    }
    let clauses = bindings
        .iter()
        .map(|reference| &source[reference.span.start..reference.span.end])
        .collect::<Vec<_>>();
    assert!(clauses.contains(&"새 값이 대기와 같다."));
    assert!(clauses.contains(&"상태를 새 값으로 정한다."));
    assert!(
        !references
            .iter()
            .any(|reference| reference.field.starts_with("condition."))
    );
    assert_eq!(references, semantic_references(&module));
}
