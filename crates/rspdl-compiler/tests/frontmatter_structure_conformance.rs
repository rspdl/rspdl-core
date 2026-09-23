use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

use rspdl_compiler::compile_ko;
use serde::Deserialize;

#[derive(Deserialize)]
struct Case {
    spec_version: String,
    locale: String,
    category: String,
    expected_module: bool,
    expected_diagnostics: Vec<ExpectedDiagnostic>,
}

#[derive(Clone, Deserialize)]
struct ExpectedDiagnostic {
    rule_id: String,
    severity: String,
    message_key: String,
    arguments: BTreeMap<String, String>,
    /// 사례가 위치 자체를 고정하고 싶을 때만 적는다. 대개는 규칙과 인자로 충분하지만,
    /// 어느 줄을 짚느냐가 요점인 진단이 있다.
    #[serde(default)]
    span: Option<[usize; 2]>,
}

#[test]
fn frontmatter_structure_conformance_suite() {
    let root = repository_root().join("conformance/ko-KR/frontmatter-structure");
    let mut dirs = fs::read_dir(root)
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .collect::<Vec<_>>();
    dirs.sort();
    assert!(!dirs.is_empty(), "the family should have cases");

    let mut categories = BTreeSet::new();
    for dir in dirs {
        let name = dir.file_name().unwrap().to_string_lossy().into_owned();
        let case: Case =
            serde_json::from_str(&fs::read_to_string(dir.join("case.json")).unwrap()).unwrap();
        assert!(
            matches!(case.spec_version.as_str(), "0.5.0" | "0.7.0"),
            "case {name}"
        );
        assert_eq!(case.locale, "ko-KR", "case {name}");
        categories.insert(case.category.clone());

        let source = fs::read_to_string(dir.join("input.rspdl")).unwrap();
        let compiled = compile_ko(&source);
        assert_eq!(compiled, compile_ko(&source), "{name} repeat");
        assert_eq!(compiled.module.is_some(), case.expected_module, "{name}");
        assert_eq!(
            actual_diagnostics(&compiled.diagnostics),
            expected_diagnostics(&case.expected_diagnostics),
            "{name}"
        );

        // 위치를 적어 둔 사례는 위치까지 본다. 진단이 맞아도 엉뚱한 줄을 짚으면 사람은
        // 멀쩡한 줄을 고치게 된다.
        for (expected, actual) in case
            .expected_diagnostics
            .iter()
            .zip(compiled.diagnostics.iter())
        {
            if let Some([start, end]) = expected.span {
                assert_eq!(
                    [actual.span.start, actual.span.end],
                    [start, end],
                    "{name} span for {}",
                    expected.rule_id
                );
            }
        }

        // 포맷은 뜻을 바꾸지 않는다. 사례마다 다시 써 낸 원문을 한 번 더 컴파일해 같은 IR 과
        // 같은 진단이 나오는지 본다.
        //
        // 이 가족이 formatter 를 덮는 유일한 자리다. 단위 테스트만 두면 감싸야 할 글자를
        // 하나 빠뜨렸을 때 그 글자를 쓴 사례가 생길 때까지 아무도 모르고, 그동안 `rspdl
        // format` 은 사용자의 선언을 조용히 지운다.
        if case.expected_module {
            let document = rspdl_ko::parse(&source)
                .document
                .unwrap_or_else(|| panic!("{name} should parse"));
            let formatted = rspdl_ko::format_document(&document)
                .unwrap_or_else(|error| panic!("{name} should format: {error}"));
            let reformatted = compile_ko(&formatted);
            assert_eq!(
                without_spans(&reformatted.module),
                without_spans(&compiled.module),
                "{name}: 포맷 뒤 IR 이 달라졌다"
            );
            assert_eq!(
                actual_diagnostics(&reformatted.diagnostics),
                actual_diagnostics(&compiled.diagnostics),
                "{name}: 포맷 뒤 진단이 달라졌다"
            );
        }

        // 오탐 방지 사례의 요점은 개수다. 원인 하나에 진단이 둘이면 사람은 없는 문제를
        // 쫓는다. 목록이 맞는지와 별개로 이 조건을 따로 고정한다.
        if case.category == "false_positive" {
            assert_eq!(
                compiled.diagnostics.len(),
                1,
                "{name} should report exactly one diagnostic"
            );
        }

        // 진단만 맞고 구조가 비어 있어도 위의 단언은 전부 통과한다. 정상 사례에서는
        // 머리말이 실제로 IR 에 도달했는지까지 본다.
        if name == "normal" {
            let module = compiled.module.as_ref().expect("normal compiles");
            // 분류는 선언한 중첩의 전위 순회 순서로 나온다. 정보구조에서 형제의 순서는
            // 메뉴 순서이고 그것은 기획자가 적은 순서로 정한 것이므로, 정렬해 버리면
            // 선언된 의도를 버리는 것이 된다.
            assert_eq!(
                module
                    .information_architecture
                    .iter()
                    .map(|category| (
                        category.id.to_string(),
                        category
                            .parent_id
                            .as_ref()
                            .map(ToString::to_string)
                            .unwrap_or_default()
                    ))
                    .collect::<Vec<_>>(),
                [
                    ("shopping.order".to_owned(), String::new()),
                    ("shopping.add".to_owned(), "shopping.order".to_owned()),
                    ("shopping.review".to_owned(), "shopping.order".to_owned()),
                ]
            );
            assert_eq!(
                module
                    .screen_categories
                    .iter()
                    .map(|assignment| (
                        assignment.screen_id.to_string(),
                        assignment.category_id.to_string()
                    ))
                    .collect::<Vec<_>>(),
                // 소속도 같은 이유로 선언 순서다.
                [
                    ("shopping.create_item".to_owned(), "shopping.add".to_owned()),
                    (
                        "shopping.cart_detail".to_owned(),
                        "shopping.review".to_owned()
                    ),
                ]
            );
            // 레이아웃과 경로도 선언 순서다.
            assert_eq!(
                module
                    .screen_layouts
                    .iter()
                    .map(|layout| layout.screen_id.to_string())
                    .collect::<Vec<_>>(),
                ["shopping.create_item", "shopping.cart_detail"]
            );
            assert_eq!(module.screen_paths.len(), 1);
            let path = &module.screen_paths[0];
            assert_eq!(path.source_screen_id.to_string(), "shopping.create_item");
            assert_eq!(path.source_element_id, "submit");
            assert_eq!(
                path.target_screen_id.as_ref().unwrap().to_string(),
                "shopping.cart_detail"
            );
            assert_eq!(path.label.as_deref(), Some("담기 성공"));
        }

        // 적지 않은 것이 기본값으로 채워지지 않는지. 이 불변식은 조용히 깨지므로
        // 진단이 아니라 IR 을 직접 본다.
        if name == "boundary-unstated-kind" {
            let module = compiled.module.as_ref().expect("boundary compiles");
            assert!(
                module
                    .screen_layouts
                    .iter()
                    .all(|layout| layout.kind.is_none()),
                "an unstated screen kind must stay unstated"
            );
        }
        if name == "boundary-stable-element-ids" {
            let module = compiled.module.as_ref().expect("stable elements compile");
            let json = serde_json::to_value(&module.screen_layouts[0].elements).unwrap();
            let text = json.to_string();
            for id in [
                "header",
                "title",
                "content",
                "product_form",
                "name_input",
                "products",
                "preview",
                "submit",
            ] {
                assert!(text.contains(&format!("\"id\":\"{id}\"")), "missing {id}");
            }
        }
    }

    assert_eq!(
        categories,
        ["boundary", "failure", "false_positive", "normal"]
            .into_iter()
            .map(str::to_owned)
            .collect::<BTreeSet<_>>()
    );
}

fn expected_diagnostics(
    values: &[ExpectedDiagnostic],
) -> Vec<(String, String, String, BTreeMap<String, String>)> {
    values
        .iter()
        .map(|diagnostic| {
            (
                diagnostic.rule_id.clone(),
                diagnostic.severity.clone(),
                diagnostic.message_key.clone(),
                diagnostic.arguments.clone(),
            )
        })
        .collect()
}

fn actual_diagnostics(
    values: &[rspdl_domain::Diagnostic],
) -> Vec<(String, String, String, BTreeMap<String, String>)> {
    values
        .iter()
        .map(|diagnostic| {
            (
                diagnostic.rule_id.clone(),
                serde_json::to_value(diagnostic.severity)
                    .unwrap()
                    .as_str()
                    .unwrap()
                    .to_owned(),
                diagnostic.message_key.clone(),
                diagnostic.arguments.clone(),
            )
        })
        .collect()
}

/// 포맷은 글자의 자리를 옮기므로 span 은 당연히 달라진다. 뜻이 같은지만 본다.
fn without_spans<T: serde::Serialize>(value: &T) -> serde_json::Value {
    fn strip(value: &mut serde_json::Value) {
        match value {
            serde_json::Value::Object(object) => {
                object.remove("span");
                for value in object.values_mut() {
                    strip(value);
                }
            }
            serde_json::Value::Array(values) => values.iter_mut().for_each(strip),
            _ => {}
        }
    }
    let mut value = serde_json::to_value(value).unwrap();
    strip(&mut value);
    value
}

fn repository_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .unwrap()
        .to_path_buf()
}
