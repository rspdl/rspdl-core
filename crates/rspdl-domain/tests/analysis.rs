use rspdl_domain::{
    FieldIntentKind, PolicyEffect, RelationOperator, ScreenOperationKind, SurfaceRef, TextRange,
    UnlinkedAction, UnlinkedConstraint, UnlinkedDataModel, UnlinkedDeclaration, UnlinkedField,
    UnlinkedFieldIntent, UnlinkedLiteral, UnlinkedModule, UnlinkedOperand, UnlinkedPolicy,
    UnlinkedRole, UnlinkedScreen, UnlinkedTypeReference, analyze,
};

fn span() -> TextRange {
    TextRange { start: 10, end: 20 }
}

fn declaration(name: &str, id: Option<&str>) -> UnlinkedDeclaration {
    UnlinkedDeclaration {
        name: name.into(),
        id: id.map(str::to_owned),
        span: span(),
    }
}

fn reference(name: &str) -> SurfaceRef {
    SurfaceRef::new(name, span())
}

fn empty_module(name: &str) -> UnlinkedModule {
    UnlinkedModule {
        declaration: declaration(name, Some("expense")),
        enums: Vec::new(),
        models: Vec::new(),
        screens: Vec::new(),
        derivations: Vec::new(),
        recalculations: Vec::new(),
        field_intents: Vec::new(),
        constraints: Vec::new(),
        roles: Vec::new(),
        actions: Vec::new(),
        policies: Vec::new(),
    }
}

fn policy_module(labels: [&str; 5]) -> UnlinkedModule {
    let [module_name, model_name, field_name, role_name, action_name] = labels;
    let mut module = empty_module(module_name);
    module.models.push(UnlinkedDataModel {
        declaration: declaration(model_name, Some("request")),
        fields: vec![UnlinkedField {
            declaration: declaration(field_name, Some("amount")),
            required: true,
            value_type: UnlinkedTypeReference::Integer,
        }],
    });
    module.roles.push(UnlinkedRole {
        declaration: declaration(role_name, Some("manager")),
    });
    module.actions.push(UnlinkedAction {
        declaration: declaration(action_name, Some("change")),
    });
    module.constraints.push(UnlinkedConstraint {
        declaration: declaration("", None),
        model: reference(model_name),
        left: UnlinkedOperand::Field(reference(field_name)),
        operator: RelationOperator::GreaterThan,
        right: UnlinkedOperand::Literal(UnlinkedLiteral::Integer {
            value: "0".into(),
            span: span(),
        }),
        span: span(),
    });
    module.policies.push(UnlinkedPolicy {
        declaration: declaration("", None),
        role: reference(role_name),
        model: reference(model_name),
        field: reference(field_name),
        action: reference(action_name),
        effect: PolicyEffect::Allow,
        span: span(),
    });
    module
}

#[test]
fn canonical_generated_ids_do_not_depend_on_locale_labels() {
    let korean = analyze(policy_module(["승인", "신청", "금액", "관리자", "변경"]));
    let english = analyze(policy_module([
        "Approval", "Request", "Amount", "Manager", "Change",
    ]));
    assert!(korean.diagnostics.is_empty(), "{:?}", korean.diagnostics);
    assert!(english.diagnostics.is_empty(), "{:?}", english.diagnostics);

    let korean = korean.module.unwrap();
    let english = english.module.unwrap();
    assert_eq!(korean.constraints[0].id, english.constraints[0].id);
    assert_eq!(korean.policies[0].id, english.policies[0].id);
    assert_eq!(
        korean.constraints[0].id.as_str(),
        "expense.constraint_72fbbd5f8aa621cb"
    );
    assert_eq!(
        korean.policies[0].id.as_str(),
        "expense.policy_45439f1d15749ca3"
    );
    assert_eq!(korean.policies[0].role_id, english.policies[0].role_id);
    assert_eq!(korean.policies[0].field_id, english.policies[0].field_id);
}

#[test]
fn unresolved_symbols_are_rejected_by_the_shared_analyzer() {
    let mut module = policy_module(["승인", "신청", "금액", "관리자", "변경"]);
    module.policies[0].role = reference("없는 역할");

    let output = analyze(module);

    assert!(output.module.is_none());
    assert!(output.diagnostics.iter().any(|diagnostic| {
        diagnostic.rule_id == "RSPDL-LINK-003"
            && diagnostic.message_key == "semantic.symbol.not_found"
            && diagnostic.argument("reference") == Some("없는 역할")
    }));
}

fn data_usage_module(include_input: bool, include_read: bool) -> UnlinkedModule {
    let mut module = empty_module("데이터 사용");
    module.models.push(UnlinkedDataModel {
        declaration: declaration("항목", Some("item")),
        fields: vec![UnlinkedField {
            declaration: declaration("금액", Some("amount")),
            required: true,
            value_type: UnlinkedTypeReference::Integer,
        }],
    });
    module.screens.push(UnlinkedScreen {
        declaration: declaration("작성 화면", Some("create_item")),
        model: reference("항목"),
        fields: Vec::new(),
        operation: ScreenOperationKind::Create,
        span: span(),
    });
    if include_input {
        module.screens.push(UnlinkedScreen {
            declaration: declaration("작성 화면", Some("create_item")),
            model: reference("항목"),
            fields: vec![reference("금액")],
            operation: ScreenOperationKind::Input,
            span: span(),
        });
    }
    if include_read {
        module.screens.push(UnlinkedScreen {
            declaration: declaration("상세 화면", Some("item_detail")),
            model: reference("항목"),
            fields: vec![reference("금액")],
            operation: ScreenOperationKind::Read,
            span: span(),
        });
    }
    module
}

#[test]
fn lifecycle_rules_run_without_a_locale_frontend() {
    let normal = analyze(data_usage_module(true, true));
    assert!(normal.module.is_some(), "{:?}", normal.diagnostics);
    assert!(normal.diagnostics.is_empty());

    let failure = analyze(data_usage_module(false, true));
    assert!(failure.module.is_none());
    assert!(
        failure
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.rule_id == "RSPDL-DATA-001")
    );

    let mut intentional_non_read = data_usage_module(true, false);
    intentional_non_read
        .field_intents
        .push(UnlinkedFieldIntent {
            model: reference("항목"),
            field: reference("금액"),
            intent: FieldIntentKind::Hidden,
            span: span(),
        });
    let false_positive_prevention = analyze(intentional_non_read);
    assert!(
        false_positive_prevention.module.is_some(),
        "{:?}",
        false_positive_prevention.diagnostics
    );
    assert!(
        !false_positive_prevention
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.rule_id == "RSPDL-DATA-W001")
    );
}
