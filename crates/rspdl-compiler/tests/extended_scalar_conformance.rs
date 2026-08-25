use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

use rspdl_compiler::compile_ko;
use rspdl_domain::{CanonicalType, CanonicalValue, ConstraintOperand};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
struct Case {
    spec_version: String,
    locale: String,
    category: String,
    expected_module: bool,
    expected_diagnostics: Vec<ExpectedDiagnostic>,
    expected_field_types: BTreeMap<String, String>,
    expected_constraint_values: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct ExpectedDiagnostic {
    rule_id: String,
    message_key: String,
}

#[test]
fn extended_scalar_conformance_suite() {
    let root = repository_root().join("conformance/ko-KR/extended-scalars");
    let mut directories = fs::read_dir(&root)
        .unwrap_or_else(|error| panic!("failed to read {}: {error}", root.display()))
        .map(|entry| entry.expect("conformance entry should be readable").path())
        .filter(|path| path.is_dir())
        .collect::<Vec<_>>();
    directories.sort();

    let mut categories = BTreeSet::new();
    for directory in directories {
        let name = directory.file_name().unwrap().to_string_lossy();
        let case = read_case(&directory);
        assert_eq!(case.spec_version, "0.4.0", "case {name}");
        assert_eq!(case.locale, "ko-KR", "case {name}");
        categories.insert(case.category.clone());

        let input = fs::read_to_string(directory.join("input.rspdl")).unwrap();
        let compilation = compile_ko(&input);
        assert_eq!(compilation, compile_ko(&input), "case {name} determinism");
        assert_eq!(
            compilation.module.is_some(),
            case.expected_module,
            "case {name}"
        );
        assert_eq!(
            compilation.diagnostics.len(),
            case.expected_diagnostics.len(),
            "case {name}"
        );
        for (actual, expected) in compilation
            .diagnostics
            .iter()
            .zip(&case.expected_diagnostics)
        {
            assert_eq!(actual.rule_id, expected.rule_id, "case {name}");
            assert_eq!(actual.message_key, expected.message_key, "case {name}");
        }

        let Some(module) = compilation.module else {
            assert!(case.expected_field_types.is_empty(), "case {name}");
            assert!(case.expected_constraint_values.is_empty(), "case {name}");
            continue;
        };
        let actual_types = module
            .models
            .iter()
            .flat_map(|model| model.fields.iter())
            .map(|field| (field.id.to_string(), field.value_type.to_string()))
            .collect::<BTreeMap<_, _>>();
        assert_eq!(actual_types, case.expected_field_types, "case {name}");
        let actual_values = module
            .constraints
            .iter()
            .filter_map(|constraint| match &constraint.right {
                ConstraintOperand::Constant(value) => Some(canonical_text(value)),
                ConstraintOperand::Field(_) => None,
            })
            .collect::<Vec<_>>();
        assert_eq!(
            actual_values, case.expected_constraint_values,
            "case {name}"
        );
    }

    assert_eq!(
        categories,
        ["boundary", "failure", "false_positive", "normal"]
            .into_iter()
            .map(str::to_owned)
            .collect()
    );
}

fn canonical_text(value: &CanonicalValue) -> String {
    match value.value_type() {
        CanonicalType::Boolean => value.as_boolean().unwrap().to_string(),
        CanonicalType::Integer | CanonicalType::Refinement(_) => {
            value.as_integer().unwrap().to_string()
        }
        CanonicalType::Decimal => value.as_decimal().unwrap().to_string(),
        CanonicalType::String => value.as_string().unwrap().to_owned(),
        CanonicalType::Date => value.as_date().unwrap().to_string(),
        CanonicalType::Time => value.as_time().unwrap().to_string(),
        CanonicalType::DateTime => value.as_date_time().unwrap().to_string(),
        CanonicalType::Duration => value.as_duration().unwrap().to_string(),
        CanonicalType::Latitude => value.as_latitude().unwrap().to_string(),
        CanonicalType::Longitude => value.as_longitude().unwrap().to_string(),
        CanonicalType::Enum(_) => value.as_enum_variant().unwrap().to_string(),
        _ => value.canonical_text(),
    }
}

fn read_case(directory: &Path) -> Case {
    serde_json::from_str(&fs::read_to_string(directory.join("case.json")).unwrap()).unwrap()
}

fn repository_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .unwrap()
        .to_path_buf()
}
