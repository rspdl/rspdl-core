use rspdl_sdk::{check_json, compile_json};
use serde_json::{Value, json};

const SOURCE: &str = "@모듈 일정(schedule)\n프로젝트(project)는 다음 필드들로 구성되어 있다.\n    시작일(start): 필수 날짜\n    마감일(end): 필수 날짜\n프로젝트의 마감일은 시작일보다 커야 한다.\n";

fn request(source: &str) -> Value {
    json!({"schema_version": 1, "locale": "ko-KR", "sources": [{"path": "schedule.rspdl", "text": source}]})
}

fn check(source: &str, record: Value) -> Value {
    let mut request = request(source);
    request["data"] = json!({"records": {"schedule.project": [record]}});
    serde_json::from_str(&check_json(&request.to_string()).unwrap()).unwrap()
}

fn record(start: &str, end: &str) -> Value {
    json!({"$id": "project-1", "start": start, "end": end})
}

fn assert_compiled(report: &Value) {
    assert!(
        report["result"]["compilation"]["files"][0]["diagnostics"]
            .as_array()
            .unwrap()
            .is_empty(),
        "{report}"
    );
    assert!(report["result"]["compilation"]["files"][0]["module"].is_object());
}

#[test]
fn strict_and_inclusive_boundaries_and_inversions_reach_the_sdk() {
    for (requirement, accepted) in [
        ("커야", [false, false, true]),
        ("크거나 같아야", [false, true, true]),
    ] {
        let source = SOURCE.replace("커야", requirement);
        for (end, accepted) in ["2026-10-04", "2026-10-05", "2026-10-06"]
            .into_iter()
            .zip(accepted)
        {
            let report = check(&source, record("2026-10-05", end));
            assert_compiled(&report);
            assert_eq!(report["result"]["runtime_diagnostics"], json!([]));
            assert_eq!(
                report["result"]["constraint_violations"]
                    .as_array()
                    .unwrap()
                    .is_empty(),
                accepted,
                "{requirement}: {end}"
            );
        }
    }
}

#[test]
fn violation_resolves_both_field_ids_and_the_utf8_rule_span() {
    let compiled: Value =
        serde_json::from_str(&compile_json(&request(SOURCE).to_string()).unwrap()).unwrap();
    let report = check(SOURCE, record("2026-10-06", "2026-10-05"));
    assert_compiled(&report);
    assert_eq!(compiled["result"], report["result"]["compilation"]);
    let violation = &report["result"]["constraint_violations"][0];
    assert_eq!(violation["model_id"], "schedule.project");
    assert_eq!(violation["record_id"], "project-1");
    let constraint = compiled["result"]["files"][0]["module"]["constraints"]
        .as_array()
        .unwrap()
        .iter()
        .find(|constraint| constraint["id"] == violation["constraint_id"])
        .unwrap();
    assert_eq!(
        constraint["left"],
        json!({"kind": "field", "value": "schedule.project.end"})
    );
    assert_eq!(
        constraint["right"],
        json!({"kind": "field", "value": "schedule.project.start"})
    );
    let start = constraint["span"]["start"].as_u64().unwrap() as usize;
    let end = constraint["span"]["end"].as_u64().unwrap() as usize;
    assert_eq!(
        &SOURCE[start..end],
        "프로젝트의 마감일은 시작일보다 커야 한다."
    );
    assert!(start > SOURCE[..start].chars().count());
}

#[test]
fn required_and_invalid_inputs_have_field_locations_for_the_form() {
    for (record, rule, key) in [
        (
            json!({"$id": "project-1", "start": "2026-10-05"}),
            "RSPDL-INPUT-014",
            "runtime.field.required_missing",
        ),
        (
            json!({"$id": "project-1", "start": "2026-10-05", "end": null}),
            "RSPDL-INPUT-014",
            "runtime.field.required_missing",
        ),
        (
            record("2026-10-05", "2026-02-30"),
            "RSPDL-INPUT-015",
            "runtime.value.type_mismatch",
        ),
        (
            json!({"$id": "project-1", "start": "2026-10-05", "end": 20261006}),
            "RSPDL-INPUT-015",
            "runtime.value.type_mismatch",
        ),
    ] {
        let report = check(SOURCE, record);
        assert_compiled(&report);
        let diagnostics = report["result"]["runtime_diagnostics"].as_array().unwrap();
        assert_eq!(diagnostics.len(), 1, "{report}");
        let diagnostic = &diagnostics[0];
        assert_eq!(diagnostic["rule_id"], rule);
        assert_eq!(diagnostic["message_key"], key);
        assert_eq!(diagnostic["severity"], "error");
        assert_eq!(diagnostic["path"], "$.records.schedule.project[0].end");
        assert_eq!(diagnostic["arguments"]["field_id"], "end");
        if rule == "RSPDL-INPUT-015" {
            assert_eq!(diagnostic["arguments"]["expected_type"], "date");
        }
    }
}

#[test]
fn changing_start_rechecks_the_previously_valid_end() {
    let original = check(SOURCE, record("2026-10-05", "2026-10-06"));
    let edited = check(SOURCE, record("2026-10-07", "2026-10-06"));
    assert_eq!(original["result"]["constraint_violations"], json!([]));
    assert_eq!(edited["result"]["runtime_diagnostics"], json!([]));
    assert_eq!(
        edited["result"]["constraint_violations"]
            .as_array()
            .unwrap()
            .len(),
        1
    );
    assert_eq!(edited, check(SOURCE, record("2026-10-07", "2026-10-06")));
}

#[test]
fn source_order_and_unsupported_sources_never_hide_failures() {
    let other = SOURCE.replace("schedule", "other");
    let mut first = request(SOURCE);
    first["sources"]
        .as_array_mut()
        .unwrap()
        .push(json!({"path": "other.rspdl", "text": other}));
    first["data"] = json!({"records": {"schedule.project": [record("2026-10-06", "2026-10-05")]}});
    let mut reversed = first.clone();
    reversed["sources"].as_array_mut().unwrap().reverse();
    assert_eq!(
        compile_json(&first_without_data(&first).to_string()).unwrap(),
        compile_json(&first_without_data(&reversed).to_string()).unwrap()
    );
    assert_eq!(
        check_json(&first.to_string()).unwrap(),
        check_json(&reversed.to_string()).unwrap()
    );
    for source in [
        SOURCE.replace("시작일보다", "없는 필드보다"),
        SOURCE.replace("필수 날짜", "필수 불리언"),
    ] {
        let report = check(&source, record("2026-10-05", "2026-10-06"));
        assert!(report["result"]["compilation"]["files"][0]["module"].is_null());
        assert!(
            report["result"]["compilation"]["files"][0]["diagnostics"]
                .as_array()
                .unwrap()
                .iter()
                .any(|diagnostic| diagnostic["severity"] == "error"),
            "{report}"
        );
    }
}

fn first_without_data(request: &Value) -> Value {
    let mut request = request.clone();
    request.as_object_mut().unwrap().remove("data");
    request
}
