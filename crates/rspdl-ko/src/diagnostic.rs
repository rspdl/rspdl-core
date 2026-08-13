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
        "ko.syntax.domain_annotation_forbidden" => format!(
            "{} annotation은 사용할 수 없습니다. @는 문서 metadata인 @모듈에만 허용됩니다.",
            argument(diagnostic, "annotation")
        ),
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
        "ko.syntax.action_data_mutation_invalid" => {
            "행동 결과는 생성한다, 수정한다, 삭제한다 중 하나여야 합니다.".into()
        }
        "ko.syntax.action_input_stable_id_required" => {
            "행동 입력의 표시 이름 뒤에 stable ID가 필요합니다.".into()
        }
        "ko.syntax.action_input_name_marker_required" => {
            "행동 입력 이름과 stable ID 뒤에는 로 또는 으로가 필요합니다.".into()
        }
        "ko.syntax.creation_branch_stable_id_required" => {
            "조건부 생성 branch의 표시 이름 뒤에 stable ID가 필요합니다.".into()
        }
        "ko.syntax.creation_branch_topic_marker_required" => {
            "조건부 생성 branch 이름 뒤에는 은 또는 는이 필요합니다.".into()
        }
        "ko.syntax.creation_branch_result_invalid" => {
            "조건부 생성 결과는 하나 생성한다 또는 생성하지 않는다여야 합니다.".into()
        }
        "ko.syntax.creation_branch_condition_marker_invalid" => {
            "조건부 생성 조건은 행동의 입력이 enum 값이면 형식이어야 합니다.".into()
        }
        "ko.syntax.creation_branch_result_marker_invalid" => {
            "조건부 생성 결과의 output model 뒤에는 을 또는 를이 필요합니다.".into()
        }
        "ko.syntax.field_producer_stable_id_required" => {
            "필드 생산자의 stable ID가 필요합니다.".into()
        }
        "ko.syntax.field_producer_topic_marker_required" => {
            "필드 생산자 이름 뒤에는 은 또는 는이 필요합니다.".into()
        }
        "ko.syntax.field_producer_literal_required" => {
            "상수 생산자에는 literal이 필요합니다.".into()
        }
        "ko.syntax.field_producer_literal_marker_required" => {
            "상수 literal 뒤에는 을 또는 를이 필요합니다.".into()
        }
        "ko.syntax.template_string_required" => {
            "알림 내용 조합에는 문자열 template이 필요합니다.".into()
        }
        "ko.syntax.template_unmatched_brace" => {
            "template의 { 또는 } 짝이 맞지 않습니다. literal brace는 {{ 또는 }}로 쓰세요.".into()
        }
        "ko.syntax.template_empty_placeholder" => {
            "template placeholder는 비워둘 수 없습니다.".into()
        }
        "ko.syntax.template_nested_placeholder" => {
            "template placeholder 안에 {를 중첩할 수 없습니다.".into()
        }
        "ko.syntax.template_path_placeholder_forbidden" => format!(
            "template placeholder {}은 output field 이름 하나여야 하며 경로를 쓸 수 없습니다.",
            argument(diagnostic, "placeholder")
        ),
        "ko.syntax.relation_producer_stable_id_required" => {
            "관계 생산자의 stable ID가 필요합니다.".into()
        }
        "ko.syntax.relation_producer_topic_marker_required" => {
            "관계 생산자 이름 뒤에는 은 또는 는이 필요합니다.".into()
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
        "ko.syntax.relation_stable_id_required" => "관계 선언에 stable ID가 필요합니다.".into(),
        "ko.syntax.relation_direction_marker_required" => {
            "관계 이름 뒤에는 로 또는 으로가 필요합니다.".into()
        }
        "ko.syntax.relational_constraint_group_references" => {
            "그룹 관계 규칙에는 서로 다른 관계 참조가 둘 이상 필요합니다.".into()
        }
        "ko.syntax.reference_list_required" => "하나 이상의 참조가 필요합니다.".into(),
        "ko.syntax.reference_list_empty_name" => "참조 목록에 빈 이름이 있습니다.".into(),
        "ko.syntax.reference_list_invalid" => "참조 목록 형식이 올바르지 않습니다.".into(),
        "ko.syntax.reference_list_final_marker_required" => {
            "관계 목록의 마지막 이름 뒤에는 은 또는 는이 필요합니다.".into()
        }
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
        "semantic.field.duplicate_local_id" => format!(
            "필드 stable ID {}가 중복 선언되었습니다.",
            argument(diagnostic, "id")
        ),
        "semantic.reference.ambiguous" => format!(
            "{} 참조 {}가 여러 stable ID({})와 일치합니다.",
            symbol_kind(argument(diagnostic, "kind")),
            argument(diagnostic, "reference"),
            argument(diagnostic, "candidates")
        ),
        "semantic.enum.not_found" => {
            let reference = argument(diagnostic, "reference");
            format!(
                "열거형 {reference}{} 찾을 수 없습니다.",
                object_marker(reference)
            )
        }
        "semantic.enum.variant_not_found" => {
            let reference = argument(diagnostic, "reference");
            format!(
                "열거형 값 {reference}{} 찾을 수 없습니다.",
                object_marker(reference)
            )
        }
        "semantic.creation_branch.decision_input_not_found" => {
            let trigger_id = diagnostic
                .argument("trigger_id")
                .or_else(|| diagnostic.argument("action_id"))
                .unwrap_or("<?>");
            if diagnostic.argument("trigger_kind") == Some("event") {
                format!(
                    "사건 {trigger_id}에서 조건 입력 {}을 찾을 수 없습니다.",
                    argument(diagnostic, "reference")
                )
            } else {
                format!(
                    "행동 {trigger_id}에서 조건 입력 {}을 찾을 수 없습니다.",
                    argument(diagnostic, "reference")
                )
            }
        }
        "semantic.creation_branch.decision_input_requires_enum" => format!(
            "조건부 생성 입력 {}은 닫힌 열거형 값이어야 합니다.",
            argument(diagnostic, "input_id")
        ),
        "semantic.creation_branch.variant_not_in_decision_enum" => format!(
            "조건 값 {}은 입력 {}의 열거형 {}에 속하지 않습니다.",
            argument(diagnostic, "reference"),
            argument(diagnostic, "input_id"),
            argument(diagnostic, "enum_id")
        ),
        "semantic.creation_branch.legacy_action_incompatible" => {
            if diagnostic.argument("trigger_kind") == Some("event") {
                format!(
                    "사건 {}을 조건부 생성 branch의 행동 호환 참조로 함께 지정할 수 없습니다.",
                    argument(diagnostic, "trigger_id")
                )
            } else {
                format!(
                    "조건부 생성 branch의 행동 호환 참조 {}은 트리거 {}와 같아야 합니다.",
                    diagnostic
                        .argument("legacy_action_id")
                        .or_else(|| diagnostic.argument("legacy_action_reference"))
                        .unwrap_or("<?>"),
                    argument(diagnostic, "trigger_id")
                )
            }
        }
        "semantic.creation_production.mixed_decision_inputs" => format!(
            "생산 {}은 하나의 조건 입력만 사용해야 하지만 {}을 함께 사용합니다.",
            argument(diagnostic, "production_id"),
            argument(diagnostic, "input_ids")
        ),
        "semantic.creation_production.variant_conflict" => format!(
            "생산 {}의 값 {}에 branch {}가 함께 선언되었습니다.",
            argument(diagnostic, "production_id"),
            argument(diagnostic, "variant_id"),
            argument(diagnostic, "branch_ids")
        ),
        "semantic.creation_production.variant_coverage_missing" => format!(
            "생산 {}의 조건 입력 {}에서 값 {}가 빠졌습니다.",
            argument(diagnostic, "production_id"),
            argument(diagnostic, "input_id"),
            argument(diagnostic, "missing_variant_ids")
        ),
        "semantic.creation_production.required_field_producer_missing" => format!(
            "생산 {}의 필수 field {}는 생성 branch {}에서 값을 생산하지 않습니다.",
            argument(diagnostic, "production_id"),
            argument(diagnostic, "field_id"),
            argument(diagnostic, "create_branch_ids")
        ),
        "semantic.field_producer.source_input_not_found" => format!(
            "생산자 {}의 trigger 입력 {}을 찾을 수 없습니다.",
            argument(diagnostic, "producer_id"),
            argument(diagnostic, "source")
        ),
        "semantic.field_producer.source_trigger_owner_mismatch" => format!(
            "생산자 {}의 source는 해당 trigger의 선언 입력이어야 합니다.",
            argument(diagnostic, "producer_id")
        ),
        "semantic.field_producer.type_mismatch" => match diagnostic.argument("source_type") {
            Some(source_type) => format!(
                "생산자 {}의 source {} 타입 {}은 output field {}의 {} 타입과 맞지 않습니다.",
                argument(diagnostic, "producer_id"),
                argument(diagnostic, "source"),
                source_type,
                argument(diagnostic, "output_field_id"),
                argument(diagnostic, "output_type")
            ),
            None => format!(
                "생산자 {}의 source {}은 output field {}의 {} 타입 값으로 사용할 수 없습니다.",
                argument(diagnostic, "producer_id"),
                argument(diagnostic, "source"),
                argument(diagnostic, "output_field_id"),
                argument(diagnostic, "output_type")
            ),
        },
        "semantic.creation_production.field_producer_without_creation_decision" => format!(
            "생산자 {}은 생산 결정이 없는 trigger/output에 붙을 수 없습니다.",
            argument(diagnostic, "producer_id")
        ),
        "semantic.event_field_producer.conditional_unsupported" => format!(
            "Event 생산자 {}은 현재 조건을 가질 수 없습니다.",
            argument(diagnostic, "producer_id")
        ),
        "semantic.event_field_producer.constant_unsupported" => format!(
            "Event 생산자 {}은 현재 상수를 source로 사용할 수 없습니다.",
            argument(diagnostic, "producer_id")
        ),
        "semantic.producer.legacy_action_incompatible" => format!(
            "생산자 {}의 legacy Action reference는 tagged trigger와 함께 사용할 수 없습니다.",
            argument(diagnostic, "producer_id")
        ),
        "semantic.creation_production.field_producer_conflict" => format!(
            "생산 {}의 field {}에 producer {}가 함께 선언되었습니다.",
            argument(diagnostic, "production_id"),
            argument(diagnostic, "field_id"),
            argument(diagnostic, "producer_ids")
        ),
        "semantic.template.dependency_producer_missing" => format!(
            "template target {}가 참조한 output field {}는 생성 branch {}에서 값을 생산하지 않습니다.",
            argument(diagnostic, "target_field_id"),
            argument(diagnostic, "dependency_field_id"),
            argument(diagnostic, "create_branch_ids")
        ),
        "semantic.template.dependency_producer_conflict" => format!(
            "template target {}가 참조한 output field {}에 producer {}가 함께 선언되었습니다.",
            argument(diagnostic, "target_field_id"),
            argument(diagnostic, "dependency_field_id"),
            argument(diagnostic, "producer_ids")
        ),
        "semantic.template.dependency_cycle" => format!(
            "생산 {}의 template output field 의존성이 순환합니다: {}.",
            argument(diagnostic, "production_id"),
            argument(diagnostic, "cycle_field_ids")
        ),
        "semantic.template.placeholder_not_string" => format!(
            "template 생산자 {}가 참조한 output field {}의 타입 {}은 문자열이 아닙니다.",
            argument(diagnostic, "producer_id"),
            argument(diagnostic, "dependency_field_id"),
            argument(diagnostic, "dependency_type")
        ),
        "semantic.creation_production.relation_producer_without_creation_decision" => format!(
            "관계 생산자 {}은 생산 결정이 없는 trigger/output에 붙을 수 없습니다.",
            argument(diagnostic, "producer_id")
        ),
        "semantic.creation_production.relation_producer_not_exactly_one_slot" => format!(
            "관계 생산자 {}의 관계 {}은 ExactlyOne output slot이 아닙니다.",
            argument(diagnostic, "producer_id"),
            argument(diagnostic, "relation_id")
        ),
        "semantic.relation_producer.source_input_invalid" => format!(
            "관계 생산자 {}의 trigger 입력 {}을 찾을 수 없습니다.",
            argument(diagnostic, "producer_id"),
            argument(diagnostic, "source")
        ),
        "semantic.relation_producer.source_endpoint_mismatch" => format!(
            "관계 생산자 {}의 입력 {}은 관계 {}의 endpoint {}와 맞지 않습니다.",
            argument(diagnostic, "producer_id"),
            argument(diagnostic, "input_id"),
            argument(diagnostic, "relation_id"),
            argument(diagnostic, "endpoint_model_id")
        ),
        "semantic.creation_production.required_relation_producer_missing" => format!(
            "생산 {}의 관계 slot {}은 생성 branch {}에서 연결되지 않습니다.",
            argument(diagnostic, "production_id"),
            argument(diagnostic, "relation_id"),
            argument(diagnostic, "create_branch_ids")
        ),
        "semantic.creation_production.relation_producer_conflict" => format!(
            "생산 {}의 관계 slot {}에 producer {}가 함께 선언되었습니다.",
            argument(diagnostic, "production_id"),
            argument(diagnostic, "relation_id"),
            argument(diagnostic, "producer_ids")
        ),
        "semantic.field_producer.condition_not_creation_decision_variant" => format!(
            "생산자 {}의 조건 {}={}은 생산 {}의 결정 입력 {}의 값이어야 합니다.",
            argument(diagnostic, "producer_id"),
            argument(diagnostic, "input_id"),
            argument(diagnostic, "variant_id"),
            argument(diagnostic, "production_id"),
            argument(diagnostic, "decision_input_id")
        ),
        "semantic.constraint.operand_type_mismatch" => {
            "제약의 양쪽 operand 타입이 다릅니다.".into()
        }
        "semantic.constraint.order_requires_ordered_type" => {
            "대소 비교는 정수, 소수, 날짜, 시간, 날짜시간, 기간, 위도 또는 경도에만 사용할 수 있습니다.".into()
        }
        "semantic.relation.arity_unsupported" => format!(
            "관계 parameter는 {}개만 지원하지만 {}개가 선언되었습니다.",
            argument(diagnostic, "supported"),
            argument(diagnostic, "actual")
        ),
        "semantic.relation.not_found" => {
            let reference = argument(diagnostic, "reference");
            format!(
                "관계 {reference}{} 찾을 수 없습니다.",
                object_marker(reference)
            )
        }
        "semantic.relation.cardinality_requires_binary" => format!(
            "필수/유일 cardinality는 이항 관계에만 사용할 수 있습니다: {}.",
            argument(diagnostic, "relation_id")
        ),
        "semantic.relation.cardinality_anchor_mismatch" => {
            let relation_id = argument(diagnostic, "relation_id");
            let expected_model_id = argument(diagnostic, "expected_model_id");
            let actual_model_id = argument(diagnostic, "actual_model_id");
            format!(
                "관계 {relation_id}의 기준 개체는 {expected_model_id}인데 규칙에는 {actual_model_id}{} 사용되었습니다.",
                subject_marker(actual_model_id)
            )
        }
        "semantic.model.field_required" => {
            "데이터 모델은 하나 이상의 필드를 선언해야 합니다.".into()
        }
        "semantic.relation.group_requires_distinct_members" => {
            "관계 그룹에는 서로 다른 관계가 둘 이상 필요합니다.".into()
        }
        "semantic.relation.group_signature_mismatch" => {
            "같은 그룹의 관계는 parameter 모델과 순서가 같아야 합니다.".into()
        }
        "semantic.relation.compatibility_conflict" => {
            let relation_ids = argument(diagnostic, "relation_ids");
            format!(
                "같은 관계 그룹 {relation_ids}{} 배타적이면서 공존 가능하다고 선언할 수 없습니다.",
                object_marker(relation_ids)
            )
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
        "semantic.action_data_mutation.duplicate" => format!(
            "행동 {}의 데이터 모델 {}에 대한 {} 결과가 중복 선언되었습니다.",
            argument(diagnostic, "action_id"),
            argument(diagnostic, "model_id"),
            argument(diagnostic, "mutation")
        ),
        "semantic.action_data_mutation.conflict" => format!(
            "행동 {}은 데이터 모델 {}에 서로 양립할 수 없는 결과 {}를 동시에 선언할 수 없습니다.",
            argument(diagnostic, "action_id"),
            argument(diagnostic, "model_id"),
            argument(diagnostic, "mutations")
        ),
        "semantic.action_input.duplicate_id" => format!(
            "같은 행동에서 입력 stable ID {}가 중복 선언되었습니다.",
            argument(diagnostic, "id")
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
        "semantic.lifecycle.field_producer_missing" => {
            let field_id = argument(diagnostic, "field_id");
            format!(
                "필드 {field_id}{} 만드는 화면 입력 또는 계산이 없습니다.",
                object_marker(field_id)
            )
        }
        "semantic.lifecycle.model_creator_missing" => {
            let model_id = argument(diagnostic, "model_id");
            format!(
                "데이터 모델 {model_id}{} 생성하는 화면 또는 행동 결과가 없습니다.",
                object_marker(model_id)
            )
        }
        "semantic.lifecycle.produced_field_unread" => format!(
            "필드 {}은 만들어지지만 어떤 화면에서도 조회되지 않습니다.",
            argument(diagnostic, "field_id")
        ),
        "semantic.literal.type_undetermined" => "literal 타입을 결정할 수 없습니다.".into(),
        "semantic.literal.type_mismatch" => format!(
            "literal이 필드 타입 {}과 맞지 않습니다.",
            argument(diagnostic, "expected_type")
        ),
        "semantic.model.not_found" => {
            let reference = argument(diagnostic, "reference");
            format!(
                "데이터 모델 {reference}{} 찾을 수 없습니다.",
                object_marker(reference)
            )
        }
        "semantic.field.not_found" => {
            let reference = argument(diagnostic, "reference");
            format!(
                "데이터 모델 {}에서 필드 {reference}{} 찾을 수 없습니다.",
                argument(diagnostic, "model_id"),
                object_marker(reference)
            )
        }
        "semantic.symbol.not_found" => {
            let reference = argument(diagnostic, "reference");
            format!(
                "{} {reference}{} 찾을 수 없습니다.",
                symbol_kind(argument(diagnostic, "kind")),
                object_marker(reference)
            )
        }
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
        "model.invalid_decimal" => format!(
            "{}은 올바른 소수가 아닙니다.",
            argument(diagnostic, "value")
        ),
        "model.invalid_date" => format!(
            "{}은 올바른 날짜가 아닙니다.",
            argument(diagnostic, "value")
        ),
        "model.invalid_time" => format!(
            "{}은 올바른 시간이 아닙니다.",
            argument(diagnostic, "value")
        ),
        "model.invalid_date_time" => format!(
            "{}은 올바른 RFC 3339 날짜시간이 아닙니다.",
            argument(diagnostic, "value")
        ),
        "model.invalid_duration" => format!(
            "{}은 올바른 고정 기간이 아닙니다.",
            argument(diagnostic, "value")
        ),
        "model.invalid_latitude" => format!(
            "위도 {}은 -90 이상 90 이하여야 합니다.",
            argument(diagnostic, "value")
        ),
        "model.invalid_longitude" => format!(
            "경도 {}은 -180 이상 180 이하여야 합니다.",
            argument(diagnostic, "value")
        ),
        "model.unsupported_operation" => format!(
            "타입 {}에는 {} 연산을 사용할 수 없습니다.",
            argument(diagnostic, "value_type"),
            argument(diagnostic, "operation")
        ),
        _ => fallback(diagnostic),
    }
}

fn argument<'a>(diagnostic: &'a Diagnostic, key: &str) -> &'a str {
    diagnostic.argument(key).unwrap_or("<?>")
}

fn syntax_kind(kind: &str) -> &str {
    match kind {
        "action_data_mutation" => "행동 데이터 결과",
        "sum_derivation" => "계산",
        "recalculation" => "재계산",
        "field_intent" => "필드 사용 의도",
        "creation_branch" => "조건부 생성 branch",
        "field_producer" => "필드 생산자",
        "relation" => "관계 선언",
        "entity" => "개체 선언",
        "relational_constraint" => "관계 메타 규칙",
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
        "relation" => "관계",
        _ => kind,
    }
}

fn object_marker(value: &str) -> &'static str {
    match value
        .chars()
        .last()
        .filter(|character| ('가'..='힣').contains(character))
    {
        Some(last) if (last as u32 - '가' as u32).is_multiple_of(28) => "를",
        Some(_) | None => "을",
    }
}

fn subject_marker(value: &str) -> &'static str {
    match value
        .chars()
        .last()
        .filter(|character| ('가'..='힣').contains(character))
    {
        Some(last) if (last as u32 - '가' as u32).is_multiple_of(28) => "가",
        Some(_) | None => "이",
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

        let vowel_ending = Diagnostic::error(
            "RSPDL-LINK-003",
            "semantic.model.not_found",
            TextRange::default(),
        )
        .with_argument("reference", "상태");
        let consonant_ending = Diagnostic::error(
            "RSPDL-LINK-003",
            "semantic.model.not_found",
            TextRange::default(),
        )
        .with_argument("reference", "금액");
        assert_eq!(
            render_diagnostic(&vowel_ending),
            "데이터 모델 상태를 찾을 수 없습니다."
        );
        assert_eq!(
            render_diagnostic(&consonant_ending),
            "데이터 모델 금액을 찾을 수 없습니다."
        );
    }

    #[test]
    fn renders_natural_particles_for_relation_diagnostics() {
        for (reference, expected) in [
            ("소유자", "관계 소유자를 찾을 수 없습니다."),
            ("검토팀", "관계 검토팀을 찾을 수 없습니다."),
        ] {
            let diagnostic = Diagnostic::error(
                "RSPDL-REL-001",
                "semantic.relation.not_found",
                TextRange::default(),
            )
            .with_argument("reference", reference);
            assert_eq!(render_diagnostic(&diagnostic), expected);
        }

        let anchor = Diagnostic::error(
            "RSPDL-REL-003",
            "semantic.relation.cardinality_anchor_mismatch",
            TextRange::default(),
        )
        .with_argument("relation_id", "소유자")
        .with_argument("expected_model_id", "프로젝트")
        .with_argument("actual_model_id", "사용자");
        assert_eq!(
            render_diagnostic(&anchor),
            "관계 소유자의 기준 개체는 프로젝트인데 규칙에는 사용자가 사용되었습니다."
        );

        for (relations, expected) in [
            (
                "소유자",
                "같은 관계 그룹 소유자를 배타적이면서 공존 가능하다고 선언할 수 없습니다.",
            ),
            (
                "검토팀",
                "같은 관계 그룹 검토팀을 배타적이면서 공존 가능하다고 선언할 수 없습니다.",
            ),
        ] {
            let diagnostic = Diagnostic::error(
                "RSPDL-REL-004",
                "semantic.relation.compatibility_conflict",
                TextRange::default(),
            )
            .with_argument("relation_ids", relations);
            assert_eq!(render_diagnostic(&diagnostic), expected);
        }
    }

    #[test]
    fn renders_conditional_creation_core_diagnostics() {
        let arguments = [
            ("action_id", "notice.assign"),
            ("reference", "status"),
            ("input_id", "notice.assign.status"),
            ("enum_id", "notice.status"),
            ("production_id", "notice.production_x"),
            ("input_ids", "notice.assign.status,notice.assign.kind"),
            ("variant_id", "notice.status.received"),
            ("branch_ids", "notice.first,notice.second"),
            ("missing_variant_ids", "notice.status.held"),
            ("field_id", "notice.output.body"),
            ("create_branch_ids", "notice.first"),
        ];
        for key in [
            "semantic.creation_branch.decision_input_not_found",
            "semantic.creation_branch.decision_input_requires_enum",
            "semantic.creation_branch.variant_not_in_decision_enum",
            "semantic.creation_production.mixed_decision_inputs",
            "semantic.creation_production.variant_conflict",
            "semantic.creation_production.variant_coverage_missing",
            "semantic.creation_production.required_field_producer_missing",
        ] {
            let diagnostic = arguments.into_iter().fold(
                Diagnostic::error("RSPDL-TEST", key, TextRange::default()),
                |diagnostic, (argument, value)| diagnostic.with_argument(argument, value),
            );
            let rendered = render_diagnostic(&diagnostic);
            assert_ne!(rendered, key, "{key}");
            assert!(!rendered.contains("<?>"), "{key}: {rendered}");
        }
    }

    #[test]
    fn renders_field_producer_diagnostics_without_raw_keys_or_missing_arguments() {
        for key in [
            "ko.syntax.field_producer_stable_id_required",
            "ko.syntax.field_producer_topic_marker_required",
            "ko.syntax.field_producer_literal_required",
            "ko.syntax.field_producer_literal_marker_required",
        ] {
            assert_ne!(
                render_diagnostic(&Diagnostic::error("RSPDL-TEST", key, TextRange::default())),
                key
            );
        }

        let arguments = [
            ("producer_id", "notice.title_binding"),
            ("source", "notice.assign.title"),
            ("source_type", "integer"),
            ("output_field_id", "notice.output.title"),
            ("output_type", "string"),
            ("production_id", "notice.production_x"),
            ("field_id", "notice.output.title"),
            ("producer_ids", "notice.first,notice.second"),
        ];
        for key in [
            "semantic.field_producer.source_input_not_found",
            "semantic.field_producer.source_trigger_owner_mismatch",
            "semantic.field_producer.type_mismatch",
            "semantic.creation_production.field_producer_without_creation_decision",
            "semantic.creation_production.field_producer_conflict",
            "semantic.event_field_producer.conditional_unsupported",
            "semantic.event_field_producer.constant_unsupported",
            "semantic.producer.legacy_action_incompatible",
        ] {
            let diagnostic = arguments.into_iter().fold(
                Diagnostic::error("RSPDL-TEST", key, TextRange::default()),
                |diagnostic, (argument, value)| diagnostic.with_argument(argument, value),
            );
            let rendered = render_diagnostic(&diagnostic);
            assert_ne!(rendered, key, "{key}");
            assert!(!rendered.contains("<?>"), "{key}: {rendered}");
        }
    }
}
