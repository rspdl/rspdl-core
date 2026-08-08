use rspdl_domain::Diagnostic;

/// Renders a locale-neutral diagnostic for the Korean CLI surface.
pub fn render_diagnostic(diagnostic: &Diagnostic) -> String {
    match diagnostic.message_key.as_str() {
        "ko.lex.tab_indentation" => "들여쓰기에 tab을 사용할 수 없습니다.".into(),
        "ko.lex.inconsistent_dedent" => "이전 블록과 일치하지 않는 들여쓰기입니다.".into(),
        "ko.lex.unclosed_quoted_identifier" => "닫히지 않은 backtick 식별자입니다.".into(),
        "ko.lex.unclosed_stable_id" => format!(
            "{}로 닫히지 않은 stable ID입니다.",
            argument(diagnostic, "closing")
        ),
        "ko.lex.invalid_string_literal" => "문자열 literal 형식이 올바르지 않습니다.".into(),
        "ko.syntax.module_required" => "문서는 모듈 선언으로 시작해야 합니다.".into(),
        "ko.syntax.module_header_required" => {
            "문서는 @모듈 표시 이름(stable_id) 선언으로 시작해야 합니다.".into()
        }
        "ko.syntax.unexpected_top_level_indent" => {
            "최상위 선언이 아닌 위치에 들여쓴 항목이 있습니다.".into()
        }
        "ko.syntax.unknown_top_level_declaration" => "알 수 없는 최상위 선언입니다.".into(),
        "ko.syntax.item_period_forbidden" => "선언 항목에는 마침표를 사용하지 않습니다.".into(),
        "ko.syntax.enum_value_required" => "열거형 값이 필요합니다.".into(),
        "ko.syntax.field_colon_required" => "필드 표시 이름 뒤에 :이 필요합니다.".into(),
        "ko.syntax.field_shape_required" => {
            "필드는 표시 이름(local_id): 필수|선택 타입 형식이어야 합니다.".into()
        }
        "ko.syntax.field_requiredness_required" => {
            "필드는 필수 또는 선택을 선언해야 합니다.".into()
        }
        "ko.syntax.screen_must_be_sentence" => {
            "화면 동작은 들여쓰기 블록 없이 한 문장으로 작성해야 합니다.".into()
        }
        "ko.syntax.screen_stable_id_required" => "화면 stable ID가 필요합니다.".into(),
        "ko.syntax.screen_field_operation_required" => "화면의 필드 동작이 누락되었습니다.".into(),
        "ko.syntax.screen_field_operation_invalid" => {
            "필드 동작은 입력할, 조회할, 수정할 중 하나여야 합니다.".into()
        }
        "ko.syntax.screen_model_operation_invalid" => {
            "데이터 동작은 생성할, 조회할, 수정할, 삭제할 중 하나여야 합니다.".into()
        }
        "ko.syntax.field_list_required" => "필드 목록이 필요합니다.".into(),
        "ko.syntax.field_list_empty_name" => "빈 필드 이름은 사용할 수 없습니다.".into(),
        "ko.syntax.field_list_invalid" => "필드 목록 형식이 올바르지 않습니다.".into(),
        "ko.syntax.field_list_final_marker_required" => {
            "마지막 필드에는 을 또는 를이 필요합니다.".into()
        }
        "ko.syntax.field_intent_invalid" => {
            "필드는 내부 관리 또는 사용자 화면 비표시 중 하나로 분류해야 합니다.".into()
        }
        "ko.syntax.sentence_block_forbidden" => format!(
            "{} 문장 아래에는 들여쓰기 블록을 둘 수 없습니다.",
            syntax_kind(argument(diagnostic, "kind"))
        ),
        "ko.syntax.constraint_block_forbidden" => {
            "제약 문장 아래에는 별도 블록을 둘 수 없습니다.".into()
        }
        "ko.syntax.policy_block_forbidden" => {
            "정책 문장 아래에는 별도 블록을 둘 수 없습니다.".into()
        }
        "ko.syntax.policy_effect_invalid" => "정책은 수 있다 또는 수 없다로 끝나야 합니다.".into(),
        "ko.syntax.field_comparison_invalid" => {
            "필드 비교는 같아야 또는 달라야를 사용해야 합니다.".into()
        }
        "ko.syntax.period_required" => "완전한 문장은 마침표로 끝나야 합니다.".into(),
        "ko.syntax.annotated_declaration_required" => format!(
            "{} 표시 이름({}) 선언이 필요합니다.",
            argument(diagnostic, "keyword"),
            argument(diagnostic, "id_kind")
        ),
        "ko.syntax.declaration_punctuation_forbidden" => {
            "선언 줄에는 마침표나 콜론을 사용하지 않습니다.".into()
        }
        "ko.syntax.declaration_id_required" => "선언 ID가 필요합니다.".into(),
        "ko.syntax.natural_header_period_required" => {
            "데이터와 열거형 header는 마침표로 끝나는 문장이어야 합니다.".into()
        }
        "ko.syntax.declaration_topic_marker_required" => {
            "선언 이름 뒤에 은 또는 는이 필요합니다.".into()
        }
        "ko.syntax.stable_id_required" => "선언에 stable ID가 필요합니다.".into(),
        "ko.syntax.display_name_required" => "표시 이름이 필요합니다.".into(),
        "ko.syntax.display_name_invalid" => "표시 이름 형식이 올바르지 않습니다.".into(),
        "ko.syntax.block_item_required" => "블록에는 하나 이상의 항목이 필요합니다.".into(),
        "ko.syntax.block_indent_inconsistent" => {
            "블록 항목의 들여쓰기 깊이가 일정하지 않습니다.".into()
        }
        "ko.syntax.field_type_required" => "필드 타입이 필요합니다.".into(),
        "ko.syntax.field_type_invalid" => "필드 타입 형식이 올바르지 않습니다.".into(),
        "ko.syntax.reference_and_marker_required" => {
            "문장에 필요한 이름과 조사가 누락되었습니다.".into()
        }
        "ko.syntax.quoted_reference_marker_required" => {
            "인용된 이름 뒤에 구조 marker가 필요합니다.".into()
        }
        "ko.syntax.reference_marker_invalid" => format!(
            "{} 뒤에는 {} 중 하나가 필요합니다.",
            argument(diagnostic, "reference"),
            argument(diagnostic, "expected")
        ),
        "ko.syntax.reference_marker_missing" => format!(
            "{}에서 {} marker를 찾을 수 없습니다.",
            argument(diagnostic, "reference"),
            argument(diagnostic, "expected")
        ),
        "ko.syntax.surface_name_required" => "표면 이름이 필요합니다.".into(),
        "ko.syntax.comparison_value_required" => "제약의 비교 값이 누락되었습니다.".into(),
        "ko.syntax.not_equal_shape_required" => "과/와 달라야 한다 문형이 필요합니다.".into(),
        "ko.syntax.integer_order_shape_required" => {
            "<정수>보다 커야/작아야 한다 문형이 필요합니다.".into()
        }
        "ko.syntax.order_suffix_required" => "보다 커야/작아야가 필요합니다.".into(),
        "ko.syntax.integer_comparison_unsupported" => "지원하지 않는 정수 비교 문형입니다.".into(),
        "ko.syntax.literal_unsupported" => "지원하지 않는 literal입니다.".into(),
        "ko.syntax.word_required" => {
            format!("{}가 필요합니다.", argument(diagnostic, "expected"))
        }
        "ko.syntax.trailing_expression" => "문장 뒤에 예상하지 못한 표현이 있습니다.".into(),
        "ko.lint.marker_preference" => format!(
            "{}{}보다 {}{}이 자연스럽습니다.",
            argument(diagnostic, "name"),
            argument(diagnostic, "actual"),
            argument(diagnostic, "name"),
            argument(diagnostic, "expected")
        ),
        "ko.reference.not_found" => format!(
            "{} 참조 {}에 대응하는 stable ID를 찾을 수 없습니다.",
            argument(diagnostic, "kind"),
            argument(diagnostic, "reference")
        ),
        "ko.reference.ambiguous" => format!(
            "{} 참조 {}가 둘 이상의 stable ID와 일치합니다.",
            argument(diagnostic, "kind"),
            argument(diagnostic, "reference")
        ),
        "compiler.source.duplicate_path" => {
            format!(
                "source 경로 {}가 중복 지정되었습니다.",
                argument(diagnostic, "path")
            )
        }
        "compiler.module.duplicate_id" => format!(
            "모듈 ID {}가 여러 파일에 선언되었습니다.",
            argument(diagnostic, "module_id")
        ),
        "compiler.symbol.duplicate_id" => format!(
            "stable ID {}가 여러 파일에 선언되었습니다.",
            argument(diagnostic, "symbol_id")
        ),
        "semantic.enum.duplicate_variant_id" => format!(
            "열거형 값 ID {}가 중복 선언되었습니다.",
            argument(diagnostic, "id")
        ),
        "semantic.enum.not_found" => format!(
            "열거형 {}을 찾을 수 없습니다.",
            argument(diagnostic, "reference")
        ),
        "semantic.enum.variant_not_found" => format!(
            "열거형 값 {}을 찾을 수 없습니다.",
            argument(diagnostic, "reference")
        ),
        "semantic.constraint.operand_type_mismatch" => {
            "제약의 양쪽 operand 타입이 다릅니다.".into()
        }
        "semantic.constraint.order_requires_integer" => {
            "대소 비교는 정수 필드에만 사용할 수 있습니다.".into()
        }
        "semantic.screen.id_name_conflict" => format!(
            "화면 ID {}가 {}와 {} 두 이름으로 사용되었습니다.",
            argument(diagnostic, "screen_id"),
            argument(diagnostic, "existing_name"),
            argument(diagnostic, "new_name")
        ),
        "semantic.screen.duplicate_operation" => format!(
            "화면 {}의 데이터 동작이 중복 선언되었습니다.",
            argument(diagnostic, "screen_id")
        ),
        "semantic.derivation.sum_requires_integer" => {
            "합계의 원본과 결과 필드는 모두 정수여야 합니다.".into()
        }
        "semantic.derivation.multiple_producers" => format!(
            "계산 필드 {}은 화면 입력과 계산 결과를 동시에 생산자로 가질 수 없습니다.",
            argument(diagnostic, "field_id")
        ),
        "semantic.derivation.duplicate_target" => format!(
            "필드 {}의 계산식이 중복 선언되었습니다.",
            argument(diagnostic, "field_id")
        ),
        "semantic.derivation.cross_model_scope_unknown" => {
            "교차 모델 합계의 레코드 선택 관계가 정의되지 않아 계산 범위는 unknown입니다.".into()
        }
        "semantic.recalculation.exactly_one_required" => format!(
            "계산 필드 {}은 재계산 시점을 정확히 하나 선언해야 합니다. 현재 {}개입니다.",
            argument(diagnostic, "field_id"),
            argument(diagnostic, "actual")
        ),
        "semantic.recalculation.source_mismatch" => format!(
            "재계산 원본 필드가 다릅니다. 기대 {}, 실제 {}.",
            argument(diagnostic, "expected_field_id"),
            argument(diagnostic, "actual_field_id")
        ),
        "semantic.recalculation.derivation_missing" => format!(
            "필드 {}의 재계산 조건에 대응하는 계산식이 없습니다.",
            argument(diagnostic, "field_id")
        ),
        "semantic.field_intent.duplicate" => format!(
            "필드 {}의 사용 의도가 중복 선언되었습니다.",
            argument(diagnostic, "field_id")
        ),
        "semantic.field_intent.conflict" => format!(
            "필드 {}에 내부 관리와 비표시 의도를 함께 선언할 수 없습니다.",
            argument(diagnostic, "field_id")
        ),
        "semantic.lifecycle.field_producer_missing" => format!(
            "필드 {}을 만드는 화면 입력 또는 계산이 없습니다.",
            argument(diagnostic, "field_id")
        ),
        "semantic.lifecycle.model_creator_missing" => format!(
            "데이터 모델 {}을 생성하는 화면이 없습니다.",
            argument(diagnostic, "model_id")
        ),
        "semantic.lifecycle.produced_field_unread" => format!(
            "필드 {}은 만들어지지만 어떤 화면에서도 조회되지 않습니다.",
            argument(diagnostic, "field_id")
        ),
        "semantic.literal.type_undetermined" => "literal 타입을 결정할 수 없습니다.".into(),
        "semantic.literal.type_mismatch" => format!(
            "literal이 필드 타입 {}과 맞지 않습니다.",
            argument(diagnostic, "expected_type")
        ),
        "semantic.model.not_found" => format!(
            "데이터 모델 {}을 찾을 수 없습니다.",
            argument(diagnostic, "reference")
        ),
        "semantic.field.not_found" => format!(
            "데이터 모델 {}에서 필드 {}을 찾을 수 없습니다.",
            argument(diagnostic, "model_id"),
            argument(diagnostic, "reference")
        ),
        "semantic.symbol.not_found" => format!(
            "{} {}을 찾을 수 없습니다.",
            symbol_kind(argument(diagnostic, "kind")),
            argument(diagnostic, "reference")
        ),
        "semantic.declaration.stable_id_required" => "선언에 stable ID가 필요합니다.".into(),
        "semantic.declaration.duplicate_id" => format!(
            "stable ID {}가 중복 선언되었습니다.",
            argument(diagnostic, "id")
        ),
        "semantic.declaration.duplicate_name" => format!(
            "{} 표시 이름 {}이 중복 선언되었습니다.",
            symbol_kind(argument(diagnostic, "kind")),
            argument(diagnostic, "name")
        ),
        "model.invalid_canonical_id" => format!(
            "{}은 canonical machine ID가 아닙니다.",
            argument(diagnostic, "value")
        ),
        "model.empty_enum" => format!(
            "열거형 {}에는 값이 하나 이상 필요합니다.",
            argument(diagnostic, "type_id")
        ),
        "model.duplicate_enum_variant" => format!(
            "열거형 {}에 값 {}이 중복되었습니다.",
            argument(diagnostic, "type_id"),
            argument(diagnostic, "variant")
        ),
        "model.unknown_enum_variant" => format!(
            "{}은 열거형 {}의 값이 아닙니다.",
            argument(diagnostic, "variant"),
            argument(diagnostic, "type_id")
        ),
        "model.invalid_integer" => format!(
            "{}은 canonical base-10 정수가 아닙니다.",
            argument(diagnostic, "value")
        ),
        "model.invalid_refinement_base"
        | "model.invalid_refined_value"
        | "model.refinement_magnitude_exceeded"
        | "model.type_mismatch"
        | "model.empty_operands"
        | "model.arity_mismatch"
        | "model.unknown_predicate"
        | "model.conflicting_predicate_signature"
        | "model.non_ground_fact" => fallback(diagnostic),
        _ => fallback(diagnostic),
    }
}

fn argument<'a>(diagnostic: &'a Diagnostic, key: &str) -> &'a str {
    diagnostic.argument(key).unwrap_or("<?>")
}

fn syntax_kind(kind: &str) -> &str {
    match kind {
        "sum_derivation" => "계산",
        "recalculation" => "재계산",
        "field_intent" => "필드 사용 의도",
        _ => kind,
    }
}

fn symbol_kind(kind: &str) -> &str {
    match kind {
        "enum" => "열거형",
        "enum_variant" => "열거형 값",
        "role" => "역할",
        "action" => "행동",
        "data_model" => "데이터 모델",
        "field_id" => "필드 ID",
        "field" => "필드",
        _ => kind,
    }
}

fn fallback(diagnostic: &Diagnostic) -> String {
    if diagnostic.arguments.is_empty() {
        return diagnostic.message_key.clone();
    }
    let arguments = diagnostic
        .arguments
        .iter()
        .map(|(key, value)| format!("{key}={value}"))
        .collect::<Vec<_>>()
        .join(", ");
    format!("{} ({arguments})", diagnostic.message_key)
}

#[cfg(test)]
mod tests {
    use rspdl_domain::{Diagnostic, TextRange};

    use super::*;

    #[test]
    fn renders_structured_semantic_diagnostics_in_korean() {
        let diagnostic = Diagnostic::error(
            "RSPDL-LINK-003",
            "semantic.symbol.not_found",
            TextRange::default(),
        )
        .with_argument("kind", "role")
        .with_argument("reference", "manager");

        assert_eq!(
            render_diagnostic(&diagnostic),
            "역할 manager을 찾을 수 없습니다."
        );
        assert!(
            !serde_json::to_string(&diagnostic)
                .unwrap()
                .contains("찾을 수")
        );
    }
}
