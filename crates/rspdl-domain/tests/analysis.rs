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

fn reference(id: &str) -> SurfaceRef {
    SurfaceRef::stable_id(id, span())
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
        model: reference("request"),
        left: UnlinkedOperand::Field(reference("amount")),
        operator: RelationOperator::GreaterThan,
        right: UnlinkedOperand::Literal(UnlinkedLiteral::Integer {
            value: "0".into(),
            span: span(),
        }),
        span: span(),
    });
    module.policies.push(UnlinkedPolicy {
        declaration: declaration("", None),
        role: reference("manager"),
        model: reference("request"),
        field: reference("amount"),
        action: reference("change"),
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
    let mut qualified = policy_module(["Approval", "Request", "Amount", "Manager", "Change"]);
    qualified.constraints[0].model = reference("expense.request");
    qualified.constraints[0].left = UnlinkedOperand::Field(reference("expense.request.amount"));
    qualified.policies[0].role = reference("expense.manager");
    qualified.policies[0].model = reference("expense.request");
    qualified.policies[0].field = reference("expense.request.amount");
    qualified.policies[0].action = reference("expense.change");
    let qualified = analyze(qualified);
    assert!(korean.diagnostics.is_empty(), "{:?}", korean.diagnostics);
    assert!(english.diagnostics.is_empty(), "{:?}", english.diagnostics);
    assert!(
        qualified.diagnostics.is_empty(),
        "{:?}",
        qualified.diagnostics
    );

    let korean = korean.module.unwrap();
    let english = english.module.unwrap();
    let qualified = qualified.module.unwrap();
    assert_eq!(korean.constraints[0].id, english.constraints[0].id);
    assert_eq!(english.constraints[0].id, qualified.constraints[0].id);
    assert_eq!(korean.policies[0].id, english.policies[0].id);
    assert_eq!(english.policies[0].id, qualified.policies[0].id);
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
fn invalid_stable_references_are_rejected_before_name_lookup() {
    let mut module = policy_module(["Approval", "Request", "Amount", "Manager", "Change"]);
    module.policies[0].role = reference("Not-Canonical");

    let output = analyze(module);

    assert!(output.module.is_none());
    assert!(output.diagnostics.iter().any(|diagnostic| {
        diagnostic.rule_id == "RSPDL-LINK-003"
            && diagnostic.message_key == "model.invalid_canonical_id"
            && diagnostic.argument("value") == Some("Not-Canonical")
    }));
    assert!(
        !output
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.message_key == "semantic.symbol.not_found")
    );
}

#[test]
fn bare_stable_references_reject_ambiguous_qualified_suffixes() {
    let mut module = policy_module(["Approval", "Request", "Amount", "Manager", "Change"]);
    module.models.push(UnlinkedDataModel {
        declaration: declaration("Shared request", Some("shared.request")),
        fields: vec![UnlinkedField {
            declaration: declaration("Shared amount", Some("amount")),
            required: true,
            value_type: UnlinkedTypeReference::Integer,
        }],
    });

    let output = analyze(module);

    assert!(output.module.is_none());
    assert!(output.diagnostics.iter().any(|diagnostic| {
        diagnostic.message_key == "semantic.reference.ambiguous"
            && diagnostic.argument("kind") == Some("model")
            && diagnostic.argument("reference") == Some("request")
            && diagnostic.argument("candidates") == Some("expense.request,shared.request")
    }));
}

#[test]
fn duplicate_field_ids_have_stable_id_evidence() {
    let mut module = policy_module(["Approval", "Request", "Amount", "Manager", "Change"]);
    module.models[0].fields.push(UnlinkedField {
        declaration: declaration("Total", Some("amount")),
        required: false,
        value_type: UnlinkedTypeReference::Integer,
    });

    let output = analyze(module);

    assert!(output.module.is_none());
    assert!(output.diagnostics.iter().any(|diagnostic| {
        diagnostic.message_key == "semantic.field.duplicate_local_id"
            && diagnostic.argument("id") == Some("amount")
    }));
}

#[test]
fn analyzer_diagnostics_are_stable_across_repeated_execution() {
    let mut module = policy_module(["Approval", "Request", "Amount", "Manager", "Change"]);
    module.policies[0].role = reference("missing_role");
    module.policies[0].action = reference("missing_action");
    module.constraints[0].model = reference("missing_model");

    assert_eq!(analyze(module.clone()), analyze(module));
}

#[test]
fn unresolved_symbols_are_rejected_by_the_shared_analyzer() {
    let mut module = policy_module(["승인", "신청", "금액", "관리자", "변경"]);
    module.policies[0].role = reference("missing_role");

    let output = analyze(module);

    assert!(output.module.is_none());
    assert!(output.diagnostics.iter().any(|diagnostic| {
        diagnostic.rule_id == "RSPDL-LINK-003"
            && diagnostic.message_key == "semantic.symbol.not_found"
            && diagnostic.argument("reference") == Some("missing_role")
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
        model: reference("item"),
        fields: Vec::new(),
        operation: ScreenOperationKind::Create,
        span: span(),
    });
    if include_input {
        module.screens.push(UnlinkedScreen {
            declaration: declaration("작성 화면", Some("create_item")),
            model: reference("item"),
            fields: vec![reference("amount")],
            operation: ScreenOperationKind::Input,
            span: span(),
        });
    }
    if include_read {
        module.screens.push(UnlinkedScreen {
            declaration: declaration("상세 화면", Some("item_detail")),
            model: reference("item"),
            fields: vec![reference("amount")],
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
            model: reference("item"),
            field: reference("amount"),
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
