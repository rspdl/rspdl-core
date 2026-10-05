use rspdl_ko::{DeclarationAst, DocumentAst, lower, parse};
use serde_json::json;

const NORMAL: &str =
    include_str!("../../../conformance/ko-KR/statements/normal-nested/input.rspdl");

fn document(source: &str) -> DocumentAst {
    let parsed = parse(source);
    assert!(parsed.diagnostics.is_empty(), "{:?}", parsed.diagnostics);
    parsed.document.unwrap()
}

#[test]
fn parsed_statement_json_uses_recursive_tagged_contract() {
    let doc = document(&format!("{NORMAL}    대상을 삭제한다.\n"));
    let statement = doc
        .declarations
        .iter()
        .find_map(|declaration| match declaration {
            DeclarationAst::Statement(statement) => Some(statement),
            _ => None,
        })
        .unwrap();
    let serialized = serde_json::to_value(statement).unwrap();
    let condition = &serialized["condition"];
    assert_eq!(condition["kind"], "and");
    let comparisons = &condition["definition"];
    assert_eq!(comparisons[0]["kind"], "compare");
    assert_eq!(
        comparisons[0]["definition"]["left"],
        json!({"kind":"input", "value":"값"})
    );
    assert_eq!(
        comparisons[0]["definition"]["right"],
        json!({"kind":"literal", "value":{"kind":"integer", "value":"0"}})
    );
    let or = &comparisons[1];
    assert_eq!(or["kind"], "or");
    let compare = &or["definition"][0]["definition"];
    assert_eq!(
        compare["left"],
        json!({"kind":"input_field", "value":{"binding":"대상", "field":"완료"}})
    );
    assert_eq!(
        compare["right"],
        json!({"kind":"literal", "value":{"kind":"boolean", "value":false}})
    );
    let not = &or["definition"][1];
    assert_eq!(not["kind"], "not");
    assert_eq!(not["definition"]["kind"], "compare");
    assert_eq!(
        not["definition"]["definition"]["right"],
        json!({"kind":"literal", "value":{"kind":"string", "value":"잠김"}})
    );
    assert_eq!(serialized["effects"][0]["kind"], "read");
    assert_eq!(serialized["effects"][0]["definition"]["binding"], "대상");
    assert_eq!(serialized["effects"][1]["kind"], "update");
    assert_eq!(
        serialized["effects"][1]["definition"]["assignments"][0]["value"]["kind"],
        "input"
    );
    assert_eq!(serialized["effects"][2]["kind"], "delete");
    assert_eq!(serialized["effects"][2]["definition"]["binding"], "대상");
}

#[test]
fn failed_binding_input_is_diagnosed_once_across_all_field_uses() {
    for (binding, key) in [
        (
            "대상(target)은 없는 행동의 대상 기록 입력을 사용한다.",
            "ko.reference.not_found",
        ),
        (
            "대상(target)은 변경의 없는 입력을 사용한다.",
            "ko.reference.not_found",
        ),
        (
            "대상(target)은 다른 행동의 대상 기록 입력을 사용한다.",
            "ko.statement.binding_trigger_mismatch",
        ),
    ] {
        let source = NORMAL
            .replace(
                "변경(change)은 행동이다.",
                "변경(change)은 행동이다.\n다른 행동(other)은 행동이다.",
            )
            .replace("대상(target)은 변경의 대상 기록 입력을 사용한다.", binding);
        let result = lower(&document(&source));
        assert_eq!(
            result.diagnostics.len(),
            1,
            "{binding}: {:?}",
            result.diagnostics
        );
        assert_eq!(result.diagnostics[0].message_key, key);
    }
}

#[test]
fn valid_binding_keeps_input_model_and_field_ids() {
    let result = lower(&document(NORMAL));
    assert!(result.diagnostics.is_empty(), "{:?}", result.diagnostics);
    let statement = &result.module.unwrap().statements[0];
    assert_eq!(statement.bindings[0].input.id(), "target_record");
    let rspdl_domain::UnlinkedStatementEffect::Update { assignments, .. } = &statement.effects[1]
    else {
        panic!("update")
    };
    assert_eq!(assignments[0].field.id(), "count");
    assert_eq!(assignments[1].field.id(), "title");
}

#[test]
fn duplicate_binding_ids_do_not_choose_another_bindings_model() {
    let source = NORMAL.replace("값(value)은", "값(target)은");
    let result = lower(&document(&source));
    // Duplicate IDs remain for semantic validation; field resolution must not silently
    // borrow the first declaration's model when either alias shares that ID.
    let statement = &result.module.unwrap().statements[0];
    assert_eq!(statement.bindings.len(), 2);
    let rspdl_domain::UnlinkedStatementEffect::Update { assignments, .. } = &statement.effects[1]
    else {
        panic!("update")
    };
    assert_ne!(assignments[0].field.id(), "count");
}

#[test]
fn ambiguous_binding_input_stays_ambiguous_and_reports_once() {
    let source = NORMAL.replace(
        "변경은 기존 기록을 대상 기록(target_record)으로 입력받는다.",
        "변경은 기존 기록을 대상 기록(target_record)으로 입력받는다.\n변경은 기존 기록을 대상 기록(other_record)으로 입력받는다.",
    );
    let result = lower(&document(&source));
    assert_eq!(result.diagnostics.len(), 1, "{:?}", result.diagnostics);
    assert_eq!(result.diagnostics[0].message_key, "ko.reference.ambiguous");
    assert!(result.module.is_none());
}

#[test]
fn event_bindings_use_the_same_cached_resolution_contract() {
    let source = NORMAL
        .replace("변경(change)은 행동이다.", "변경(change)은 사건이다.")
        .replace("으로 입력받는다.", "으로 담는다.")
        .replace("로 입력받는다.", "로 담는다.")
        .replace("실행될 때", "발생할 때");
    let valid = lower(&document(&source));
    assert!(valid.diagnostics.is_empty(), "{:?}", valid.diagnostics);
    assert_eq!(
        valid.module.unwrap().statements[0].bindings[0].input.id(),
        "target_record"
    );
    let bad = source.replace(
        "대상(target)은 변경의 대상 기록 입력을 사용한다.",
        "대상(target)은 변경의 없는 입력을 사용한다.",
    );
    let invalid = lower(&document(&bad));
    assert_eq!(invalid.diagnostics.len(), 1, "{:?}", invalid.diagnostics);
    assert_eq!(invalid.diagnostics[0].message_key, "ko.reference.not_found");
}
