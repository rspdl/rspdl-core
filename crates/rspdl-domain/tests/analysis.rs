use rspdl_domain::{
    DataMutationKind, FieldIntentKind, PolicyEffect, RelationOperator, ScreenOperationKind,
    SurfaceRef, TextRange, UnlinkedAction, UnlinkedActionDataMutation, UnlinkedActionInput,
    UnlinkedActionInputKind, UnlinkedConstraint, UnlinkedDataModel, UnlinkedDeclaration,
    UnlinkedField, UnlinkedFieldIntent, UnlinkedLiteral, UnlinkedModule, UnlinkedOperand,
    UnlinkedPolicy, UnlinkedRelation, UnlinkedRelationalConstraint,
    UnlinkedRelationalConstraintKind, UnlinkedRole, UnlinkedScreen, UnlinkedTypeReference, analyze,
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
        span: span(),
        enums: Vec::new(),
        models: Vec::new(),
        relations: Vec::new(),
        relational_constraints: Vec::new(),
        screens: Vec::new(),
        action_data_mutations: Vec::new(),
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
            span: span(),
        }],
        span: span(),
    });
    module.roles.push(UnlinkedRole {
        declaration: declaration(role_name, Some("manager")),
        span: span(),
    });
    module.actions.push(UnlinkedAction {
        declaration: declaration(action_name, Some("change")),
        inputs: Vec::new(),
        span: span(),
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
fn action_input_ids_are_scoped_by_action_in_the_shared_analyzer() {
    let mut module = empty_module("주문");
    module.models.push(UnlinkedDataModel {
        declaration: declaration("주문", Some("order")),
        fields: vec![UnlinkedField {
            declaration: declaration("상태", Some("status")),
            required: true,
            value_type: UnlinkedTypeReference::String,
            span: span(),
        }],
        span: span(),
    });
    for (name, id) in [("접수", "receive"), ("취소", "cancel")] {
        module.actions.push(UnlinkedAction {
            declaration: declaration(name, Some(id)),
            inputs: vec![UnlinkedActionInput {
                declaration: declaration("대상", Some("target")),
                kind: UnlinkedActionInputKind::ExistingModel {
                    model: reference("order"),
                },
                span: span(),
            }],
            span: span(),
        });
    }

    let analyzed = analyze(module);
    assert!(
        analyzed.diagnostics.is_empty(),
        "{:?}",
        analyzed.diagnostics
    );
    let module = analyzed.module.unwrap();
    assert_eq!(
        module.actions[0].inputs[0].id.as_str(),
        "expense.receive.target"
    );
    assert_eq!(
        module.actions[1].inputs[0].id.as_str(),
        "expense.cancel.target"
    );
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
fn analyzer_preserves_source_backed_spans_without_using_them_for_policy_ids() {
    let mut first = policy_module(["Approval", "Request", "Amount", "Manager", "Change"]);
    first.span = TextRange { start: 0, end: 8 };
    first.models[0].span = TextRange { start: 10, end: 40 };
    first.models[0].fields[0].span = TextRange { start: 20, end: 30 };
    first.roles[0].span = TextRange { start: 41, end: 50 };
    first.actions[0].span = TextRange { start: 51, end: 60 };
    first.constraints[0].span = TextRange { start: 61, end: 70 };
    first.policies[0].span = TextRange { start: 71, end: 80 };

    let mut shifted = first.clone();
    shifted.policies[0].span = TextRange {
        start: 171,
        end: 180,
    };

    let first = analyze(first).module.unwrap();
    let shifted = analyze(shifted).module.unwrap();
    assert_eq!(first.span, TextRange { start: 0, end: 8 });
    assert_eq!(first.models[0].span, TextRange { start: 10, end: 40 });
    assert_eq!(
        first.models[0].fields[0].span,
        TextRange { start: 20, end: 30 }
    );
    assert_eq!(first.roles[0].span, TextRange { start: 41, end: 50 });
    assert_eq!(first.actions[0].span, TextRange { start: 51, end: 60 });
    assert_eq!(first.constraints[0].span, TextRange { start: 61, end: 70 });
    assert_eq!(first.policies[0].span, TextRange { start: 71, end: 80 });
    assert_eq!(first.policies[0].id, shifted.policies[0].id);
    assert_ne!(first.policies[0].span, shifted.policies[0].span);
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
            span: span(),
        }],
        span: span(),
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
        span: span(),
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

#[test]
fn fieldless_data_models_are_rejected_by_the_shared_analyzer() {
    let mut module = empty_module("Fieldless model");
    module.models.push(UnlinkedDataModel {
        declaration: declaration("Project", Some("project")),
        fields: Vec::new(),
        span: span(),
    });

    let output = analyze(module);

    assert!(output.module.is_none());
    assert!(output.diagnostics.iter().any(|diagnostic| {
        diagnostic.rule_id == "RSPDL-DATA-007"
            && diagnostic.message_key == "semantic.model.field_required"
    }));
}

#[test]
fn relation_cardinality_sentence_must_name_the_anchor_model() {
    let mut module = empty_module("Relations");
    for (name, id) in [("Project", "project"), ("User", "user")] {
        module.models.push(UnlinkedDataModel {
            declaration: declaration(name, Some(id)),
            fields: vec![UnlinkedField {
                declaration: declaration("Name", Some("name")),
                required: true,
                value_type: UnlinkedTypeReference::String,
                span: span(),
            }],
            span: span(),
        });
    }
    module.relations.push(UnlinkedRelation {
        declaration: declaration("Owner", Some("owner")),
        parameter_models: vec![reference("project"), reference("user")],
        span: span(),
    });
    module
        .relational_constraints
        .push(UnlinkedRelationalConstraint {
            declaration: declaration("", None),
            constraint: UnlinkedRelationalConstraintKind::Required {
                model: reference("user"),
                relation: reference("owner"),
            },
            span: span(),
        });

    let output = analyze(module);

    assert!(output.module.is_none());
    assert!(output.diagnostics.iter().any(|diagnostic| {
        diagnostic.rule_id == "RSPDL-REL-002"
            && diagnostic.message_key == "semantic.relation.cardinality_anchor_mismatch"
            && diagnostic.argument("expected_model_id") == Some("expense.project")
            && diagnostic.argument("actual_model_id") == Some("expense.user")
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
            span: span(),
        }],
        span: span(),
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

#[test]
fn action_data_mutations_require_a_structural_model_producer() {
    for mutation in [DataMutationKind::Update, DataMutationKind::Delete] {
        let mut module = data_usage_module(true, true);
        module.screens.retain(|screen| {
            screen.operation != ScreenOperationKind::Create
                && screen.operation != ScreenOperationKind::Input
                && screen.operation != ScreenOperationKind::Read
        });
        module.actions.push(UnlinkedAction {
            declaration: declaration("처리", Some("process")),
            inputs: Vec::new(),
            span: span(),
        });
        module
            .action_data_mutations
            .push(UnlinkedActionDataMutation {
                action: reference("process"),
                model: reference("item"),
                mutation,
                span: span(),
            });

        let output = analyze(module);
        assert!(output.module.is_none(), "{mutation:?}");
        assert!(output.diagnostics.iter().any(|diagnostic| {
            diagnostic.rule_id == "RSPDL-DATA-002"
                && diagnostic.message_key == "semantic.lifecycle.model_creator_missing"
                && diagnostic.argument("model_id") == Some("expense.item")
        }));
    }
}

#[test]
fn same_action_cannot_update_and_delete_the_same_model() {
    let mut module = data_usage_module(true, true);
    module.actions.push(UnlinkedAction {
        declaration: declaration("취소", Some("cancel")),
        inputs: Vec::new(),
        span: span(),
    });
    for mutation in [DataMutationKind::Update, DataMutationKind::Delete] {
        module
            .action_data_mutations
            .push(UnlinkedActionDataMutation {
                action: reference("cancel"),
                model: reference("item"),
                mutation,
                span: span(),
            });
    }

    let output = analyze(module);
    assert!(output.module.is_none());
    assert!(output.diagnostics.iter().any(|diagnostic| {
        diagnostic.rule_id == "RSPDL-DATA-004"
            && diagnostic.message_key == "semantic.action_data_mutation.conflict"
            && diagnostic.argument("action_id") == Some("expense.cancel")
            && diagnostic.argument("model_id") == Some("expense.item")
            && diagnostic.argument("mutations") == Some("update,delete")
    }));
}

#[test]
fn duplicate_action_data_mutation_is_rejected() {
    let mut module = data_usage_module(true, true);
    module.actions.push(UnlinkedAction {
        declaration: declaration("변경", Some("change")),
        inputs: Vec::new(),
        span: span(),
    });
    for _ in 0..2 {
        module
            .action_data_mutations
            .push(UnlinkedActionDataMutation {
                action: reference("change"),
                model: reference("item"),
                mutation: DataMutationKind::Update,
                span: span(),
            });
    }

    let output = analyze(module);
    assert!(output.module.is_none());
    assert!(output.diagnostics.iter().any(|diagnostic| {
        diagnostic.rule_id == "RSPDL-DATA-004"
            && diagnostic.message_key == "semantic.action_data_mutation.duplicate"
            && diagnostic.argument("mutation") == Some("update")
    }));
}

#[test]
fn different_actions_may_update_and_delete_the_same_model() {
    let mut module = data_usage_module(true, true);
    for (name, id, mutation) in [
        ("변경", "change", DataMutationKind::Update),
        ("삭제", "remove", DataMutationKind::Delete),
    ] {
        module.actions.push(UnlinkedAction {
            declaration: declaration(name, Some(id)),
            inputs: Vec::new(),
            span: span(),
        });
        module
            .action_data_mutations
            .push(UnlinkedActionDataMutation {
                action: reference(id),
                model: reference("item"),
                mutation,
                span: span(),
            });
    }

    let output = analyze(module);
    assert!(output.module.is_some(), "{:?}", output.diagnostics);
    assert!(output.diagnostics.is_empty(), "{:?}", output.diagnostics);
}

#[test]
fn one_screen_may_offer_update_and_delete_capabilities() {
    let mut module = data_usage_module(true, true);
    for operation in [ScreenOperationKind::Update, ScreenOperationKind::Delete] {
        module.screens.push(UnlinkedScreen {
            declaration: declaration("관리 화면", Some("manage_item")),
            model: reference("item"),
            fields: Vec::new(),
            operation,
            span: span(),
        });
    }

    let output = analyze(module);
    assert!(output.module.is_some(), "{:?}", output.diagnostics);
    assert!(output.diagnostics.is_empty(), "{:?}", output.diagnostics);
}
