use std::fs;
use std::path::{Path, PathBuf};

use rspdl_compiler::compile_ko;
use rspdl_domain::{FieldProducerSource, TemplatePart};

fn normal() -> String {
    fs::read_to_string(
        repository_root().join("conformance/ko-KR/message-templates/normal/input.rspdl"),
    )
    .unwrap()
}

fn rule_ids(source: &str) -> Vec<String> {
    compile_ko(source)
        .diagnostics
        .into_iter()
        .map(|diagnostic| diagnostic.rule_id)
        .collect()
}

#[test]
fn message_template_conformance_suite() {
    let source = normal();
    let normal_output = compile_ko(&source);
    assert!(
        normal_output.diagnostics.is_empty(),
        "{:?}",
        normal_output.diagnostics
    );
    let production = &normal_output.module.unwrap().conditional_productions[0];
    let template = production
        .field_producers
        .iter()
        .find(|producer| producer.id.as_str().ends_with("content_template"))
        .unwrap();
    assert!(matches!(
        &template.source,
        FieldProducerSource::Template { parts }
            if matches!(parts.as_slice(), [
                TemplatePart::OutputField { field_id },
                TemplatePart::Text { value },
            ] if field_id.as_str().ends_with(".title") && value == " 점검이 전달되었습니다.")
    ));
    assert_eq!(
        production
            .field_evaluation_order
            .iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>(),
        vec!["notification.notice.title", "notification.notice.content"]
    );

    assert_eq!(
        rule_ids(&source.replace("{제목}", "{없는 필드}")),
        vec!["RSPDL-KO-REF-001"]
    );
    assert_eq!(
        rule_ids(&source.replace("{제목}", "{입력.제목}")),
        vec!["RSPDL-KO-SYN-079"]
    );
    assert_eq!(
        rule_ids(&source.replace("{제목}", "{}")),
        vec!["RSPDL-KO-SYN-079"]
    );
    assert_eq!(
        rule_ids(&source.replace("{제목}", "{{제목}")),
        vec!["RSPDL-KO-SYN-079"]
    );
    let conditional_template = source.replace(
        "알림 내용 조합(content_template)은 점검 요청 전달이 실행될 때",
        "알림 내용 조합(content_template)은 점검 요청 전달의 요청 상태가 전달됨이면",
    );
    assert!(
        rule_ids(&conditional_template)
            .iter()
            .any(|rule_id| rule_id.starts_with("RSPDL-KO-SYN-"))
    );

    let unknown = source.replace("{제목}", "{없는필드}");
    assert_eq!(rule_ids(&unknown), vec!["RSPDL-KO-REF-001"]);
    let multiword_field = source
        .replace("제목(title): 필수 문자열", "알림 제목(title): 필수 문자열")
        .replace("알림의 제목으로", "알림의 알림 제목으로")
        .replace("{제목}", "{알림 제목}");
    assert!(
        compile_ko(&multiword_field).diagnostics.is_empty(),
        "{:?}",
        compile_ko(&multiword_field).diagnostics
    );
    assert!(
        rule_ids(&source.replace("내용(content): 필수 문자열", "내용(content): 필수 정수"))
            .contains(&"RSPDL-PROD-002".into())
    );
    let non_string_placeholder = source
        .replace("제목(title): 필수 문자열", "제목(title): 필수 정수")
        .replace("문자열을 알림 제목", "정수를 알림 제목");
    let non_string_placeholder = compile_ko(&non_string_placeholder);
    assert!(non_string_placeholder.diagnostics.iter().any(|diagnostic| {
        diagnostic.rule_id == "RSPDL-PROD-002"
            && diagnostic.message_key == "semantic.template.placeholder_not_string"
            && diagnostic.argument("dependency_field_id") == Some("notification.notice.title")
    }));
    assert!(rule_ids(&source.replace("{제목}", "{내용}")).contains(&"RSPDL-PROD-008".into()));

    let optional_missing = source
        .replace("제목(title): 필수 문자열", "제목(title): 선택 문자열")
        .replace("알림 제목 기록(title_binding)은 점검 요청 전달이 실행될 때 알림 제목을 점검 전달 알림의 제목으로 기록한다.\n", "");
    assert!(rule_ids(&optional_missing).contains(&"RSPDL-PROD-003".into()));
    let repeated_missing = optional_missing.replace("{제목}", "{제목} / {제목}");
    assert_eq!(
        compile_ko(&repeated_missing)
            .diagnostics
            .iter()
            .filter(|diagnostic| {
                diagnostic.message_key == "semantic.template.dependency_producer_missing"
            })
            .count(),
        1
    );

    let conflict = format!(
        "{}알림 내용 직접 기록(content_direct)은 점검 요청 전달이 실행될 때 알림 제목을 점검 전달 알림의 내용으로 기록한다.\n",
        source
    );
    assert!(rule_ids(&conflict).contains(&"RSPDL-PROD-004".into()));

    let all_skip = source.replace(
        "점검 전달 알림을 하나 생성한다",
        "점검 전달 알림을 생성하지 않는다",
    );
    assert!(compile_ko(&all_skip).diagnostics.is_empty());

    let escaped = source.replace("{제목} 점검", "{{제목}} {제목} 점검");
    let escaped_output = compile_ko(&escaped);
    assert!(escaped_output.diagnostics.is_empty());
    let escaped_module = escaped_output.module.unwrap();
    let escaped_parts = match &escaped_module.conditional_productions[0].field_producers[0].source {
        FieldProducerSource::Template { parts } => parts,
        _ => unreachable!(),
    };
    assert!(matches!(escaped_parts[0], TemplatePart::Text { ref value } if value == "{제목} "));

    let unrelated_optional = source.replace(
        "내용(content): 필수 문자열",
        "내용(content): 필수 문자열\n    부가 정보(extra): 선택 문자열",
    );
    assert!(compile_ko(&unrelated_optional).diagnostics.is_empty());

    let constant_braces = source.replace(
        "알림 제목 기록(title_binding)은 점검 요청 전달이 실행될 때 알림 제목을 점검 전달 알림의 제목으로 기록한다.",
        "알림 제목 기록(title_binding)은 점검 요청 전달이 실행될 때 상수 \"{제목}\"을 점검 전달 알림의 제목으로 기록한다.",
    );
    let constant_output = compile_ko(&constant_braces);
    assert!(constant_output.diagnostics.is_empty());
    assert!(matches!(
        constant_output.module.unwrap().conditional_productions[0].field_producers[1].source,
        FieldProducerSource::Constant { .. }
    ));

    let mut reordered_lines = source.lines().map(str::to_owned).collect::<Vec<_>>();
    let template_line = reordered_lines.pop().unwrap();
    let title_line = reordered_lines.pop().unwrap();
    reordered_lines.push(template_line);
    reordered_lines.push(title_line);
    let reordered = format!("{}\n", reordered_lines.join("\n"));
    let reordered_output = compile_ko(&reordered);
    assert_eq!(rule_ids(&source), rule_ids(&reordered));
    assert_eq!(
        compile_ko(&source).module.unwrap().conditional_productions[0].field_evaluation_order,
        reordered_output.module.unwrap().conditional_productions[0].field_evaluation_order,
    );
}

fn repository_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .unwrap()
        .to_path_buf()
}
