use rspdl_domain::{
    CreationDecision, DataMutationKind, FieldIntentKind, PolicyEffect, RelationOperator,
    ScreenOperationKind, SurfaceRef, TextRange, UnlinkedAction, UnlinkedActionDataMutation,
    UnlinkedActionInput, UnlinkedActionInputKind, UnlinkedConstraint, UnlinkedCreationBranch,
    UnlinkedDataModel, UnlinkedDeclaration, UnlinkedEnum, UnlinkedEnumVariant, UnlinkedField,
    UnlinkedFieldIntent, UnlinkedFieldProducer, UnlinkedFieldProducerCondition,
    UnlinkedFieldProducerSource, UnlinkedLiteral, UnlinkedModule, UnlinkedOperand, UnlinkedPolicy,
    UnlinkedRelation, UnlinkedRelationalConstraint, UnlinkedRelationalConstraintKind, UnlinkedRole,
    UnlinkedScreen, UnlinkedTypeReference, analyze,
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
        creation_branches: Vec::new(),
        field_producers: Vec::new(),
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

fn enum_decision_input(id: &str) -> UnlinkedActionInput {
    UnlinkedActionInput {
        declaration: declaration("Status", Some(id)),
        kind: UnlinkedActionInputKind::Value {
            value_type: UnlinkedTypeReference::Named(reference("status")),
        },
        span: span(),
    }
}

fn creation_branch(
    id: &str,
    input: &str,
    variant: &str,
    decision: CreationDecision,
) -> UnlinkedCreationBranch {
    creation_branch_at(id, input, variant, decision, span())
}

fn creation_branch_at(
    id: &str,
    input: &str,
    variant: &str,
    decision: CreationDecision,
    branch_span: TextRange,
) -> UnlinkedCreationBranch {
    UnlinkedCreationBranch {
        declaration: UnlinkedDeclaration {
            name: "Creation branch".into(),
            id: Some(id.into()),
            span: branch_span,
        },
        action: SurfaceRef::stable_id("publish", branch_span),
        input: SurfaceRef::stable_id(input, branch_span),
        variant: SurfaceRef::stable_id(variant, branch_span),
        output_model: SurfaceRef::stable_id("notice", branch_span),
        decision,
        span: branch_span,
    }
}

fn conditional_creation_module(
    output_required: bool,
    inputs: Vec<UnlinkedActionInput>,
    branches: Vec<UnlinkedCreationBranch>,
) -> UnlinkedModule {
    let mut module = empty_module("Conditional notice");
    module.enums.push(UnlinkedEnum {
        declaration: declaration("Status", Some("status")),
        variants: vec![
            UnlinkedEnumVariant {
                declaration: declaration("Draft", Some("draft")),
                span: span(),
            },
            UnlinkedEnumVariant {
                declaration: declaration("Published", Some("published")),
                span: span(),
            },
        ],
        span: span(),
    });
    module.models.push(UnlinkedDataModel {
        declaration: declaration("Notice", Some("notice")),
        fields: vec![UnlinkedField {
            declaration: declaration("Body", Some("body")),
            required: output_required,
            value_type: UnlinkedTypeReference::String,
            span: span(),
        }],
        span: span(),
    });
    module.actions.push(UnlinkedAction {
        declaration: declaration("Publish", Some("publish")),
        inputs,
        span: span(),
    });
    module.creation_branches = branches;
    module
}

fn field_producer(id: &str, source: UnlinkedFieldProducerSource) -> UnlinkedFieldProducer {
    field_producer_at(id, source, span())
}

fn field_producer_at(
    id: &str,
    source: UnlinkedFieldProducerSource,
    producer_span: TextRange,
) -> UnlinkedFieldProducer {
    UnlinkedFieldProducer {
        declaration: UnlinkedDeclaration {
            name: "Field producer".into(),
            id: Some(id.into()),
            span: producer_span,
        },
        action: SurfaceRef::stable_id("publish", producer_span),
        output_model: SurfaceRef::stable_id("notice", producer_span),
        output_field: SurfaceRef::stable_id("body", producer_span),
        source,
        condition: None,
        span: producer_span,
    }
}

fn conditional_field_producer(
    id: &str,
    source: UnlinkedFieldProducerSource,
    input: &str,
    variant: &str,
) -> UnlinkedFieldProducer {
    let mut producer = field_producer(id, source);
    producer.condition = Some(UnlinkedFieldProducerCondition::EnumVariant {
        input: reference(input),
        variant: reference(variant),
    });
    producer
}

fn exhaustive_creation_branches(decision: CreationDecision) -> Vec<UnlinkedCreationBranch> {
    vec![
        creation_branch("draft_branch", "status", "draft", decision),
        creation_branch("published_branch", "status", "published", decision),
    ]
}

type DiagnosticProjection = Vec<(String, String, Vec<(String, String)>)>;

fn diagnostic_projection(output: &rspdl_domain::AnalysisOutput) -> DiagnosticProjection {
    output
        .diagnostics
        .iter()
        .map(|diagnostic| {
            (
                diagnostic.rule_id.clone(),
                diagnostic.message_key.clone(),
                diagnostic
                    .arguments
                    .iter()
                    .map(|(key, value)| (key.clone(), value.clone()))
                    .collect(),
            )
        })
        .collect()
}

#[test]
fn conditional_creation_branches_lower_to_an_exhaustive_optional_production() {
    let output = analyze(conditional_creation_module(
        false,
        vec![enum_decision_input("status")],
        vec![
            creation_branch("draft_skip", "status", "draft", CreationDecision::Skip),
            creation_branch(
                "published_create",
                "expense.publish.status",
                "published",
                CreationDecision::Create,
            ),
        ],
    ));

    assert!(output.diagnostics.is_empty(), "{:?}", output.diagnostics);
    let module = output.module.expect("valid production should lower");
    let production = &module.conditional_productions[0];
    assert_eq!(production.action_id.as_str(), "expense.publish");
    assert_eq!(production.output_model_id.as_str(), "expense.notice");
    assert_eq!(
        production.instance_cardinality,
        rspdl_domain::ProductionCardinality::ExactlyOne
    );
    assert_eq!(
        production.decision_input_id.as_str(),
        "expense.publish.status"
    );
    assert_eq!(
        production
            .branches
            .iter()
            .map(|branch| (
                branch.id.as_str(),
                branch.variant_id.as_str(),
                branch.decision,
            ))
            .collect::<Vec<_>>(),
        vec![
            (
                "expense.draft_skip",
                "expense.status.draft",
                CreationDecision::Skip
            ),
            (
                "expense.published_create",
                "expense.status.published",
                CreationDecision::Create,
            ),
        ]
    );
}

#[test]
fn conditional_creation_reports_sorted_missing_enum_coverage() {
    let output = analyze(conditional_creation_module(
        false,
        vec![enum_decision_input("status")],
        vec![creation_branch(
            "draft_skip",
            "status",
            "draft",
            CreationDecision::Skip,
        )],
    ));

    assert!(output.module.is_none());
    let diagnostic = output
        .diagnostics
        .iter()
        .find(|diagnostic| diagnostic.rule_id == "RSPDL-POLICY-008")
        .expect("missing enum variant should be diagnosed");
    assert_eq!(
        diagnostic.message_key,
        "semantic.creation_production.variant_coverage_missing"
    );
    assert_eq!(
        diagnostic.argument("production_id"),
        Some("expense.production_4f5995e84eac40dc")
    );
    assert_eq!(diagnostic.argument("action_id"), Some("expense.publish"));
    assert_eq!(
        diagnostic.argument("output_model_id"),
        Some("expense.notice")
    );
    assert_eq!(
        diagnostic.argument("input_id"),
        Some("expense.publish.status")
    );
    assert_eq!(
        diagnostic.argument("missing_variant_ids"),
        Some("expense.status.published")
    );
}

#[test]
fn conditional_creation_reports_conflicting_branches_for_the_same_variant() {
    let output = analyze(conditional_creation_module(
        false,
        vec![enum_decision_input("status")],
        vec![
            creation_branch("draft_first", "status", "draft", CreationDecision::Skip),
            creation_branch("draft_second", "status", "draft", CreationDecision::Skip),
            creation_branch(
                "published_skip",
                "status",
                "published",
                CreationDecision::Skip,
            ),
        ],
    ));

    assert!(output.module.is_none());
    let diagnostic = output
        .diagnostics
        .iter()
        .find(|diagnostic| diagnostic.rule_id == "RSPDL-POLICY-007")
        .expect("duplicate effective branch should be diagnosed");
    assert_eq!(
        diagnostic.argument("branch_ids"),
        Some("expense.draft_first,expense.draft_second")
    );
    assert_eq!(
        diagnostic.argument("variant_id"),
        Some("expense.status.draft")
    );
}

#[test]
fn conditional_creation_reports_required_output_field_gap_only_for_create_paths() {
    let output = analyze(conditional_creation_module(
        true,
        vec![enum_decision_input("status")],
        vec![
            creation_branch("draft_create", "status", "draft", CreationDecision::Create),
            creation_branch(
                "published_skip",
                "status",
                "published",
                CreationDecision::Skip,
            ),
        ],
    ));

    assert!(output.module.is_none());
    let diagnostic = output
        .diagnostics
        .iter()
        .find(|diagnostic| diagnostic.rule_id == "RSPDL-PROD-003")
        .expect("required output field must have a create-path producer");
    assert_eq!(diagnostic.argument("field_id"), Some("expense.notice.body"));
    assert_eq!(
        diagnostic.argument("create_branch_ids"),
        Some("expense.draft_create")
    );
}

#[test]
fn conditional_creation_with_only_skip_paths_has_no_payload_gap() {
    let output = analyze(conditional_creation_module(
        true,
        vec![enum_decision_input("status")],
        vec![
            creation_branch("draft_skip", "status", "draft", CreationDecision::Skip),
            creation_branch(
                "published_skip",
                "status",
                "published",
                CreationDecision::Skip,
            ),
        ],
    ));

    assert!(output.diagnostics.is_empty(), "{:?}", output.diagnostics);
    assert!(output.module.is_some());
}

#[test]
fn conditional_creation_rejects_scalar_and_existing_model_decision_inputs() {
    let scalar = analyze(conditional_creation_module(
        false,
        vec![UnlinkedActionInput {
            declaration: declaration("Status", Some("status")),
            kind: UnlinkedActionInputKind::Value {
                value_type: UnlinkedTypeReference::String,
            },
            span: span(),
        }],
        vec![creation_branch(
            "draft_skip",
            "status",
            "draft",
            CreationDecision::Skip,
        )],
    ));
    assert!(scalar.module.is_none());
    assert!(scalar.diagnostics.iter().any(|diagnostic| {
        diagnostic.rule_id == "RSPDL-PROD-002"
            && diagnostic.message_key == "semantic.creation_branch.decision_input_requires_enum"
            && diagnostic.argument("input_id") == Some("expense.publish.status")
    }));

    let existing = analyze(conditional_creation_module(
        false,
        vec![UnlinkedActionInput {
            declaration: declaration("Status", Some("status")),
            kind: UnlinkedActionInputKind::ExistingModel {
                model: reference("notice"),
            },
            span: span(),
        }],
        vec![creation_branch(
            "draft_skip",
            "status",
            "draft",
            CreationDecision::Skip,
        )],
    ));
    assert!(existing.module.is_none());
    assert!(existing.diagnostics.iter().any(|diagnostic| {
        diagnostic.rule_id == "RSPDL-PROD-002"
            && diagnostic.message_key == "semantic.creation_branch.decision_input_requires_enum"
            && diagnostic.argument("input_id") == Some("expense.publish.status")
    }));
}

#[test]
fn conditional_creation_rejects_variants_outside_the_input_enum_and_duplicate_branch_ids() {
    let foreign_variant = analyze(conditional_creation_module(
        false,
        vec![enum_decision_input("status")],
        vec![creation_branch(
            "invalid_variant",
            "status",
            "missing",
            CreationDecision::Skip,
        )],
    ));
    assert!(foreign_variant.module.is_none());
    assert!(foreign_variant.diagnostics.iter().any(|diagnostic| {
        diagnostic.rule_id == "RSPDL-PROD-002"
            && diagnostic.message_key == "semantic.creation_branch.variant_not_in_decision_enum"
            && diagnostic.argument("enum_id") == Some("expense.status")
            && diagnostic.argument("reference") == Some("missing")
    }));

    let duplicate_id = analyze(conditional_creation_module(
        false,
        vec![enum_decision_input("status")],
        vec![
            creation_branch("same_branch", "status", "draft", CreationDecision::Skip),
            creation_branch("same_branch", "status", "published", CreationDecision::Skip),
        ],
    ));
    assert!(duplicate_id.module.is_none());
    assert!(duplicate_id.diagnostics.iter().any(|diagnostic| {
        diagnostic.rule_id == "RSPDL-LINK-003"
            && diagnostic.message_key == "semantic.declaration.duplicate_id"
            && diagnostic.argument("id") == Some("expense.same_branch")
    }));
}

#[test]
fn conditional_creation_rejects_mixed_decision_inputs_in_one_production() {
    let output = analyze(conditional_creation_module(
        false,
        vec![
            enum_decision_input("status"),
            enum_decision_input("status_alt"),
        ],
        vec![
            creation_branch("draft_skip", "status", "draft", CreationDecision::Skip),
            creation_branch(
                "published_skip",
                "status_alt",
                "published",
                CreationDecision::Skip,
            ),
        ],
    ));

    assert!(output.module.is_none());
    let diagnostic = output
        .diagnostics
        .iter()
        .find(|diagnostic| diagnostic.rule_id == "RSPDL-PROD-007")
        .expect("one production must use one direct decision input");
    assert_eq!(
        diagnostic.argument("input_ids"),
        Some("expense.publish.status,expense.publish.status_alt")
    );
}

#[test]
fn conditional_creation_is_canonical_when_branches_are_reordered() {
    let draft = creation_branch_at(
        "draft_skip",
        "status",
        "draft",
        CreationDecision::Skip,
        TextRange {
            start: 100,
            end: 120,
        },
    );
    let published = creation_branch_at(
        "published_create",
        "status",
        "published",
        CreationDecision::Create,
        TextRange { start: 30, end: 50 },
    );
    let first = analyze(conditional_creation_module(
        false,
        vec![enum_decision_input("status")],
        vec![published.clone(), draft.clone()],
    ));
    let second = analyze(conditional_creation_module(
        false,
        vec![enum_decision_input("status")],
        vec![draft, published],
    ));

    assert_eq!(
        diagnostic_projection(&first),
        diagnostic_projection(&second)
    );
    let first = first.module.expect("reordered branches remain valid");
    let second = second.module.expect("reordered branches remain valid");
    let projection = |module: &rspdl_domain::SemanticModule| {
        module
            .conditional_productions
            .iter()
            .map(|production| {
                (
                    production.id.as_str().to_owned(),
                    production.action_id.as_str().to_owned(),
                    production.output_model_id.as_str().to_owned(),
                    production.decision_input_id.as_str().to_owned(),
                    production
                        .branches
                        .iter()
                        .map(|branch| {
                            (
                                branch.id.as_str().to_owned(),
                                branch.variant_id.as_str().to_owned(),
                                branch.decision,
                            )
                        })
                        .collect::<Vec<_>>(),
                )
            })
            .collect::<Vec<_>>()
    };
    assert_eq!(projection(&first), projection(&second));
    assert_eq!(
        first.conditional_productions[0].span,
        TextRange {
            start: 100,
            end: 120
        }
    );
}

#[test]
fn conditional_creation_field_producer_copies_a_direct_value_input() {
    let mut module = conditional_creation_module(
        true,
        vec![
            enum_decision_input("status"),
            UnlinkedActionInput {
                declaration: declaration("Body", Some("body_input")),
                kind: UnlinkedActionInputKind::Value {
                    value_type: UnlinkedTypeReference::String,
                },
                span: span(),
            },
        ],
        exhaustive_creation_branches(CreationDecision::Create),
    );
    module.field_producers = vec![field_producer(
        "copy_body",
        UnlinkedFieldProducerSource::ActionInput {
            input: reference("body_input"),
        },
    )];

    let output = analyze(module);
    assert!(output.diagnostics.is_empty(), "{:?}", output.diagnostics);
    let producer = &output.module.unwrap().conditional_productions[0].field_producers[0];
    assert_eq!(producer.id.as_str(), "expense.copy_body");
    assert_eq!(producer.output_field_id.as_str(), "expense.notice.body");
    assert_eq!(producer.phase, rspdl_domain::ProducerPhase::PreMutation);
    assert!(matches!(
        &producer.source,
        rspdl_domain::FieldProducerSource::ActionInput { input_id }
            if input_id.as_str() == "expense.publish.body_input"
    ));
}

#[test]
fn conditional_creation_field_producer_reads_an_existing_model_input_field() {
    let mut module = conditional_creation_module(
        true,
        vec![
            enum_decision_input("status"),
            UnlinkedActionInput {
                declaration: declaration("Request", Some("request_input")),
                kind: UnlinkedActionInputKind::ExistingModel {
                    model: reference("request"),
                },
                span: span(),
            },
        ],
        exhaustive_creation_branches(CreationDecision::Create),
    );
    module.models.push(UnlinkedDataModel {
        declaration: declaration("Request", Some("request")),
        fields: vec![UnlinkedField {
            declaration: declaration("Title", Some("title")),
            required: true,
            value_type: UnlinkedTypeReference::String,
            span: span(),
        }],
        span: span(),
    });
    module.field_producers = vec![field_producer(
        "copy_request_title",
        UnlinkedFieldProducerSource::InputField {
            input: reference("request_input"),
            field: reference("title"),
        },
    )];

    let output = analyze(module);
    assert!(output.diagnostics.is_empty(), "{:?}", output.diagnostics);
    assert!(matches!(
        &output.module.unwrap().conditional_productions[0].field_producers[0].source,
        rspdl_domain::FieldProducerSource::InputField { input_id, field_id }
            if input_id.as_str() == "expense.publish.request_input"
                && field_id.as_str() == "expense.request.title"
    ));
}

#[test]
fn conditional_creation_field_producer_rejects_an_existing_record_as_a_direct_value() {
    let mut module = conditional_creation_module(
        true,
        vec![
            enum_decision_input("status"),
            UnlinkedActionInput {
                declaration: declaration("Request", Some("request_input")),
                kind: UnlinkedActionInputKind::ExistingModel {
                    model: reference("request"),
                },
                span: span(),
            },
        ],
        exhaustive_creation_branches(CreationDecision::Create),
    );
    module.models.push(UnlinkedDataModel {
        declaration: declaration("Request", Some("request")),
        fields: vec![UnlinkedField {
            declaration: declaration("Title", Some("title")),
            required: true,
            value_type: UnlinkedTypeReference::String,
            span: span(),
        }],
        span: span(),
    });
    module.field_producers = vec![field_producer(
        "copy_request",
        UnlinkedFieldProducerSource::ActionInput {
            input: reference("request_input"),
        },
    )];

    let output = analyze(module);
    assert!(output.module.is_none());
    assert!(output.diagnostics.iter().any(|diagnostic| {
        diagnostic.rule_id == "RSPDL-PROD-002"
            && diagnostic.message_key == "semantic.field_producer.type_mismatch"
            && diagnostic.argument("producer_id") == Some("expense.copy_request")
            && diagnostic.argument("source") == Some("expense.publish.request_input")
    }));
}

#[test]
fn conditional_creation_field_producer_rejects_a_field_path_from_a_scalar_input() {
    let mut module = conditional_creation_module(
        true,
        vec![
            enum_decision_input("status"),
            UnlinkedActionInput {
                declaration: declaration("Body", Some("body_input")),
                kind: UnlinkedActionInputKind::Value {
                    value_type: UnlinkedTypeReference::String,
                },
                span: span(),
            },
        ],
        exhaustive_creation_branches(CreationDecision::Create),
    );
    module.field_producers = vec![field_producer(
        "read_scalar_field",
        UnlinkedFieldProducerSource::InputField {
            input: reference("body_input"),
            field: reference("title"),
        },
    )];

    let output = analyze(module);
    assert!(output.module.is_none());
    assert!(output.diagnostics.iter().any(|diagnostic| {
        diagnostic.rule_id == "RSPDL-PROD-002"
            && diagnostic.message_key == "semantic.field_producer.type_mismatch"
            && diagnostic.argument("producer_id") == Some("expense.read_scalar_field")
            && diagnostic.argument("source") == Some("expense.publish.body_input")
    }));
}

#[test]
fn conditional_creation_field_producer_cannot_read_a_field_from_another_input_model() {
    let mut module = conditional_creation_module(
        true,
        vec![
            enum_decision_input("status"),
            UnlinkedActionInput {
                declaration: declaration("Request", Some("request_input")),
                kind: UnlinkedActionInputKind::ExistingModel {
                    model: reference("request"),
                },
                span: span(),
            },
        ],
        exhaustive_creation_branches(CreationDecision::Create),
    );
    for model_id in ["request", "other"] {
        module.models.push(UnlinkedDataModel {
            declaration: declaration(model_id, Some(model_id)),
            fields: vec![UnlinkedField {
                declaration: declaration("Title", Some("title")),
                required: true,
                value_type: UnlinkedTypeReference::String,
                span: span(),
            }],
            span: span(),
        });
    }
    module.field_producers = vec![field_producer(
        "cross_model_title",
        UnlinkedFieldProducerSource::InputField {
            input: reference("request_input"),
            field: SurfaceRef::stable_id("expense.other.title", span()),
        },
    )];

    let output = analyze(module);
    assert!(output.module.is_none());
    assert!(
        output.diagnostics.iter().any(|diagnostic| {
            diagnostic.rule_id == "RSPDL-LINK-003"
                && diagnostic.message_key == "semantic.field.not_found"
                && diagnostic.argument("model_id") == Some("expense.request")
                && diagnostic.argument("reference") == Some("expense.other.title")
        }),
        "{:?}",
        output.diagnostics
    );
}

#[test]
fn conditional_creation_field_producer_accepts_zero_as_a_constant_value() {
    let mut module = conditional_creation_module(
        true,
        vec![enum_decision_input("status")],
        exhaustive_creation_branches(CreationDecision::Create),
    );
    module.models[0].fields[0].value_type = UnlinkedTypeReference::Integer;
    module.field_producers = vec![field_producer(
        "zero_body",
        UnlinkedFieldProducerSource::Constant {
            literal: UnlinkedLiteral::Integer {
                value: "0".into(),
                span: span(),
            },
        },
    )];

    let output = analyze(module);
    assert!(output.diagnostics.is_empty(), "{:?}", output.diagnostics);
    assert!(matches!(
        &output.module.unwrap().conditional_productions[0].field_producers[0].source,
        rspdl_domain::FieldProducerSource::Constant { value }
            if value.as_integer().is_some_and(|integer| integer.to_string() == "0")
    ));
}

#[test]
fn conditional_creation_field_producer_reports_type_mismatch_with_canonical_evidence() {
    let mut module = conditional_creation_module(
        true,
        vec![
            enum_decision_input("status"),
            UnlinkedActionInput {
                declaration: declaration("Count", Some("count")),
                kind: UnlinkedActionInputKind::Value {
                    value_type: UnlinkedTypeReference::Integer,
                },
                span: span(),
            },
        ],
        exhaustive_creation_branches(CreationDecision::Create),
    );
    module.field_producers = vec![field_producer(
        "copy_count",
        UnlinkedFieldProducerSource::ActionInput {
            input: reference("count"),
        },
    )];

    let output = analyze(module);
    let diagnostic = output
        .diagnostics
        .iter()
        .find(|diagnostic| diagnostic.rule_id == "RSPDL-PROD-002")
        .expect("incompatible producer must be rejected");
    assert_eq!(
        diagnostic.argument("producer_id"),
        Some("expense.copy_count")
    );
    assert_eq!(diagnostic.argument("action_id"), Some("expense.publish"));
    assert_eq!(
        diagnostic.argument("output_field_id"),
        Some("expense.notice.body")
    );
    assert_eq!(diagnostic.argument("source"), Some("expense.publish.count"));
    assert_eq!(diagnostic.argument("output_type"), Some("string"));
}

#[test]
fn conditional_creation_field_producers_detect_duplicate_target_evidence_canonically() {
    let mut module = conditional_creation_module(
        true,
        vec![
            enum_decision_input("status"),
            UnlinkedActionInput {
                declaration: declaration("First", Some("first")),
                kind: UnlinkedActionInputKind::Value {
                    value_type: UnlinkedTypeReference::String,
                },
                span: span(),
            },
            UnlinkedActionInput {
                declaration: declaration("Second", Some("second")),
                kind: UnlinkedActionInputKind::Value {
                    value_type: UnlinkedTypeReference::String,
                },
                span: span(),
            },
        ],
        exhaustive_creation_branches(CreationDecision::Create),
    );
    module.field_producers = vec![
        field_producer(
            "z_second",
            UnlinkedFieldProducerSource::ActionInput {
                input: reference("second"),
            },
        ),
        field_producer(
            "a_first",
            UnlinkedFieldProducerSource::ActionInput {
                input: reference("first"),
            },
        ),
    ];

    let output = analyze(module);
    let diagnostics = output
        .diagnostics
        .iter()
        .filter(|diagnostic| diagnostic.rule_id == "RSPDL-PROD-004")
        .collect::<Vec<_>>();
    assert_eq!(diagnostics.len(), 2, "one witness per Create variant");
    assert!(diagnostics.iter().all(|diagnostic| {
        diagnostic.argument("producer_ids") == Some("expense.a_first,expense.z_second")
    }));
    assert_eq!(
        diagnostics
            .iter()
            .map(|diagnostic| diagnostic.argument("create_branch_ids"))
            .collect::<Vec<_>>(),
        vec![
            Some("expense.draft_branch"),
            Some("expense.published_branch")
        ]
    );
}

#[test]
fn conditional_field_producers_cover_distinct_create_variants_with_direct_and_existing_sources() {
    let mut module = conditional_creation_module(
        true,
        vec![
            enum_decision_input("status"),
            UnlinkedActionInput {
                declaration: declaration("Title", Some("title")),
                kind: UnlinkedActionInputKind::Value {
                    value_type: UnlinkedTypeReference::String,
                },
                span: span(),
            },
            UnlinkedActionInput {
                declaration: declaration("Request", Some("request")),
                kind: UnlinkedActionInputKind::ExistingModel {
                    model: reference("request"),
                },
                span: span(),
            },
        ],
        exhaustive_creation_branches(CreationDecision::Create),
    );
    module.models.push(UnlinkedDataModel {
        declaration: declaration("Request", Some("request")),
        fields: vec![UnlinkedField {
            declaration: declaration("Title", Some("title")),
            required: true,
            value_type: UnlinkedTypeReference::String,
            span: span(),
        }],
        span: span(),
    });
    module.field_producers = vec![
        conditional_field_producer(
            "received_title",
            UnlinkedFieldProducerSource::ActionInput {
                input: reference("title"),
            },
            "status",
            "draft",
        ),
        conditional_field_producer(
            "published_title",
            UnlinkedFieldProducerSource::InputField {
                input: reference("request"),
                field: reference("title"),
            },
            "status",
            "published",
        ),
    ];

    let output = analyze(module);
    assert!(output.diagnostics.is_empty(), "{:?}", output.diagnostics);
    let producers = &output.module.unwrap().conditional_productions[0].field_producers;
    assert_eq!(producers.len(), 2);
    assert!(matches!(
        producers[0].condition,
        Some(rspdl_domain::FieldProducerCondition::EnumVariant { .. })
    ));
}

#[test]
fn conditional_field_producer_reports_gaps_and_conflicts_per_create_variant() {
    let mut gap = conditional_creation_module(
        true,
        vec![enum_decision_input("status")],
        exhaustive_creation_branches(CreationDecision::Create),
    );
    gap.field_producers = vec![conditional_field_producer(
        "draft_title",
        UnlinkedFieldProducerSource::Constant {
            literal: UnlinkedLiteral::String {
                value: "draft".into(),
                span: span(),
            },
        },
        "status",
        "draft",
    )];
    let gap = analyze(gap);
    let missing = gap
        .diagnostics
        .iter()
        .find(|diagnostic| diagnostic.rule_id == "RSPDL-PROD-003")
        .expect("published branch must have a payload gap");
    assert_eq!(
        missing.argument("variant_id"),
        Some("expense.status.published")
    );
    assert_eq!(
        missing.argument("create_branch_ids"),
        Some("expense.published_branch")
    );

    let mut duplicate = conditional_creation_module(
        true,
        vec![enum_decision_input("status")],
        exhaustive_creation_branches(CreationDecision::Create),
    );
    duplicate.field_producers = ["z_draft", "a_draft"]
        .into_iter()
        .map(|id| {
            conditional_field_producer(
                id,
                UnlinkedFieldProducerSource::Constant {
                    literal: UnlinkedLiteral::String {
                        value: id.into(),
                        span: span(),
                    },
                },
                "status",
                "draft",
            )
        })
        .collect();
    let duplicate = analyze(duplicate);
    let conflicts = duplicate
        .diagnostics
        .iter()
        .filter(|diagnostic| diagnostic.rule_id == "RSPDL-PROD-004")
        .collect::<Vec<_>>();
    assert_eq!(conflicts.len(), 1);
    assert_eq!(
        conflicts[0].argument("variant_id"),
        Some("expense.status.draft")
    );
    assert_eq!(
        conflicts[0].argument("producer_ids"),
        Some("expense.a_draft,expense.z_draft")
    );
}

#[test]
fn conditional_producer_only_conflicts_with_an_unconditional_producer_in_its_variant() {
    let mut module = conditional_creation_module(
        true,
        vec![
            enum_decision_input("status"),
            UnlinkedActionInput {
                declaration: declaration("Title", Some("title")),
                kind: UnlinkedActionInputKind::Value {
                    value_type: UnlinkedTypeReference::String,
                },
                span: span(),
            },
        ],
        exhaustive_creation_branches(CreationDecision::Create),
    );
    module.field_producers = vec![
        field_producer(
            "always",
            UnlinkedFieldProducerSource::ActionInput {
                input: reference("title"),
            },
        ),
        conditional_field_producer(
            "draft_override",
            UnlinkedFieldProducerSource::ActionInput {
                input: reference("title"),
            },
            "status",
            "draft",
        ),
    ];
    let output = analyze(module);
    let conflicts = output
        .diagnostics
        .iter()
        .filter(|diagnostic| diagnostic.rule_id == "RSPDL-PROD-004")
        .collect::<Vec<_>>();
    assert_eq!(conflicts.len(), 1);
    assert_eq!(
        conflicts[0].argument("variant_id"),
        Some("expense.status.draft")
    );
}

#[test]
fn conditional_field_producer_rejects_every_non_decision_condition_shape() {
    let make_module =
        |condition_input: &str, condition_variant: &str, extra_inputs: Vec<UnlinkedActionInput>| {
            let mut inputs = vec![enum_decision_input("status")];
            inputs.extend(extra_inputs);
            let mut module = conditional_creation_module(
                false,
                inputs,
                exhaustive_creation_branches(CreationDecision::Skip),
            );
            module.field_producers = vec![conditional_field_producer(
                "invalid_condition",
                UnlinkedFieldProducerSource::Constant {
                    literal: UnlinkedLiteral::String {
                        value: "x".into(),
                        span: span(),
                    },
                },
                condition_input,
                condition_variant,
            )];
            analyze(module)
        };
    let non_enum = make_module(
        "text",
        "draft",
        vec![UnlinkedActionInput {
            declaration: declaration("Text", Some("text")),
            kind: UnlinkedActionInputKind::Value {
                value_type: UnlinkedTypeReference::String,
            },
            span: span(),
        }],
    );
    let wrong_axis = make_module(
        "other_status",
        "draft",
        vec![enum_decision_input("other_status")],
    );
    let unknown_variant = make_module("status", "missing", Vec::new());
    for output in [non_enum, wrong_axis, unknown_variant] {
        assert_eq!(
            output
                .diagnostics
                .iter()
                .filter(|diagnostic| diagnostic.rule_id == "RSPDL-PROD-007")
                .count(),
            1,
            "{:?}",
            output.diagnostics
        );
    }
}

#[test]
fn conditional_creation_field_producer_rejects_a_missing_action_input() {
    let mut module = conditional_creation_module(
        false,
        vec![enum_decision_input("status")],
        exhaustive_creation_branches(CreationDecision::Create),
    );
    module.field_producers = vec![field_producer(
        "missing_input",
        UnlinkedFieldProducerSource::ActionInput {
            input: reference("missing"),
        },
    )];

    let output = analyze(module);
    let diagnostic = output
        .diagnostics
        .iter()
        .find(|diagnostic| diagnostic.rule_id == "RSPDL-PROD-001")
        .expect("producer sources must start from an action input");
    assert_eq!(
        diagnostic.argument("producer_id"),
        Some("expense.missing_input")
    );
    assert_eq!(diagnostic.argument("source"), Some("missing"));
}

#[test]
fn conditional_creation_field_producer_requires_an_existing_creation_production() {
    let mut module = conditional_creation_module(false, Vec::new(), Vec::new());
    module.field_producers = vec![field_producer(
        "orphan",
        UnlinkedFieldProducerSource::Constant {
            literal: UnlinkedLiteral::String {
                value: "body".into(),
                span: span(),
            },
        },
    )];

    let output = analyze(module);
    assert!(output.module.is_none());
    assert!(output.diagnostics.iter().any(|diagnostic| {
        diagnostic.rule_id == "RSPDL-PROD-007"
            && diagnostic.message_key
                == "semantic.creation_production.field_producer_without_creation_decision"
            && diagnostic.argument("producer_id") == Some("expense.orphan")
    }));
}

#[test]
fn conditional_creation_optional_field_needs_no_producer_and_skip_paths_hide_payload_conflicts() {
    let optional = analyze(conditional_creation_module(
        false,
        vec![enum_decision_input("status")],
        exhaustive_creation_branches(CreationDecision::Create),
    ));
    assert!(
        optional.diagnostics.is_empty(),
        "{:?}",
        optional.diagnostics
    );

    let mut all_skip = conditional_creation_module(
        true,
        vec![
            enum_decision_input("status"),
            UnlinkedActionInput {
                declaration: declaration("First", Some("first")),
                kind: UnlinkedActionInputKind::Value {
                    value_type: UnlinkedTypeReference::String,
                },
                span: span(),
            },
            UnlinkedActionInput {
                declaration: declaration("Second", Some("second")),
                kind: UnlinkedActionInputKind::Value {
                    value_type: UnlinkedTypeReference::String,
                },
                span: span(),
            },
        ],
        exhaustive_creation_branches(CreationDecision::Skip),
    );
    all_skip.field_producers = vec![
        field_producer(
            "first_copy",
            UnlinkedFieldProducerSource::ActionInput {
                input: reference("first"),
            },
        ),
        field_producer(
            "second_copy",
            UnlinkedFieldProducerSource::ActionInput {
                input: reference("second"),
            },
        ),
    ];
    let all_skip = analyze(all_skip);
    assert!(
        all_skip.diagnostics.is_empty(),
        "{:?}",
        all_skip.diagnostics
    );
}

#[test]
fn conditional_creation_field_producers_for_different_fields_do_not_conflict() {
    let mut module = conditional_creation_module(
        false,
        vec![
            enum_decision_input("status"),
            UnlinkedActionInput {
                declaration: declaration("Body", Some("body_input")),
                kind: UnlinkedActionInputKind::Value {
                    value_type: UnlinkedTypeReference::String,
                },
                span: span(),
            },
            UnlinkedActionInput {
                declaration: declaration("Title", Some("title_input")),
                kind: UnlinkedActionInputKind::Value {
                    value_type: UnlinkedTypeReference::String,
                },
                span: span(),
            },
        ],
        exhaustive_creation_branches(CreationDecision::Create),
    );
    module.models[0].fields.push(UnlinkedField {
        declaration: declaration("Title", Some("title")),
        required: false,
        value_type: UnlinkedTypeReference::String,
        span: span(),
    });
    let mut title_producer = field_producer(
        "copy_title",
        UnlinkedFieldProducerSource::ActionInput {
            input: reference("title_input"),
        },
    );
    title_producer.output_field = reference("title");
    module.field_producers = vec![
        field_producer(
            "copy_body",
            UnlinkedFieldProducerSource::ActionInput {
                input: reference("body_input"),
            },
        ),
        title_producer,
    ];

    let output = analyze(module);
    assert!(output.diagnostics.is_empty(), "{:?}", output.diagnostics);
}

#[test]
fn conditional_creation_field_producers_are_canonical_when_reordered() {
    let make_module = |producers: Vec<UnlinkedFieldProducer>| {
        let mut module = conditional_creation_module(
            false,
            vec![
                enum_decision_input("status"),
                UnlinkedActionInput {
                    declaration: declaration("Body", Some("body_input")),
                    kind: UnlinkedActionInputKind::Value {
                        value_type: UnlinkedTypeReference::String,
                    },
                    span: span(),
                },
                UnlinkedActionInput {
                    declaration: declaration("Title", Some("title_input")),
                    kind: UnlinkedActionInputKind::Value {
                        value_type: UnlinkedTypeReference::String,
                    },
                    span: span(),
                },
            ],
            exhaustive_creation_branches(CreationDecision::Create),
        );
        module.models[0].fields.push(UnlinkedField {
            declaration: declaration("Title", Some("title")),
            required: false,
            value_type: UnlinkedTypeReference::String,
            span: span(),
        });
        module.field_producers = producers;
        module
    };
    let body = field_producer_at(
        "copy_body",
        UnlinkedFieldProducerSource::ActionInput {
            input: reference("body_input"),
        },
        TextRange {
            start: 90,
            end: 100,
        },
    );
    let mut title = field_producer_at(
        "copy_title",
        UnlinkedFieldProducerSource::ActionInput {
            input: reference("title_input"),
        },
        TextRange { start: 30, end: 40 },
    );
    title.output_field = reference("title");
    let first = analyze(make_module(vec![title.clone(), body.clone()]));
    let second = analyze(make_module(vec![body, title]));

    assert_eq!(
        diagnostic_projection(&first),
        diagnostic_projection(&second)
    );
    let projection = |output: &rspdl_domain::AnalysisOutput| {
        output.module.as_ref().map(|module| {
            module.conditional_productions[0]
                .field_producers
                .iter()
                .map(|producer| {
                    (
                        producer.id.as_str().to_owned(),
                        producer.output_field_id.as_str().to_owned(),
                    )
                })
                .collect::<Vec<_>>()
        })
    };
    assert_eq!(projection(&first), projection(&second));
    assert_eq!(
        projection(&first),
        Some(vec![
            ("expense.copy_body".into(), "expense.notice.body".into()),
            ("expense.copy_title".into(), "expense.notice.title".into()),
        ])
    );
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
