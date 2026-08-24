use rspdl_compiler::{Source, compile_source_with_frontend, compile_with_frontend};
use rspdl_domain::{
    DataMutationKind, Diagnostic, Frontend, FrontendOutput, RelationOperator, SurfaceRef,
    TextRange, UnlinkedAction, UnlinkedActionDataMutation, UnlinkedConstraint, UnlinkedDataModel,
    UnlinkedDeclaration, UnlinkedField, UnlinkedLiteral, UnlinkedModule, UnlinkedOperand,
    UnlinkedTypeReference,
};

struct TestFrontend;

impl Frontend for TestFrontend {
    fn language_id(&self) -> &'static str {
        "test"
    }

    fn lower_source(&self, source: &str) -> FrontendOutput {
        let model_reference = if source == "missing model" {
            "missing"
        } else {
            "item"
        };
        FrontendOutput {
            module: Some(UnlinkedModule {
                declaration: declaration("Test module", "test", span(0, 10)),
                span: span(0, 10),
                enums: Vec::new(),
                models: vec![UnlinkedDataModel {
                    declaration: declaration("Item", "item", span(10, 20)),
                    span: span(10, 30),
                    fields: vec![UnlinkedField {
                        declaration: declaration("Value", "value", span(20, 30)),
                        required: true,
                        value_type: UnlinkedTypeReference::Integer,
                        span: span(20, 30),
                    }],
                }],
                relations: Vec::new(),
                relational_constraints: Vec::new(),
                screens: Vec::new(),
                action_data_mutations: vec![UnlinkedActionDataMutation {
                    action: SurfaceRef::stable_id("create_item", span(70, 81)),
                    model: SurfaceRef::stable_id("item", span(82, 86)),
                    mutation: DataMutationKind::Create,
                    span: span(70, 90),
                }],
                derivations: Vec::new(),
                recalculations: Vec::new(),
                field_intents: Vec::new(),
                constraints: vec![UnlinkedConstraint {
                    declaration: UnlinkedDeclaration {
                        name: String::new(),
                        id: None,
                        span: span(30, 40),
                    },
                    model: SurfaceRef::stable_id(model_reference, span(40, 50)),
                    left: UnlinkedOperand::Field(SurfaceRef::stable_id("value", span(50, 60))),
                    operator: RelationOperator::GreaterThan,
                    right: UnlinkedOperand::Literal(UnlinkedLiteral::Integer {
                        value: "0".into(),
                        span: span(60, 61),
                    }),
                    span: span(30, 70),
                }],
                roles: Vec::new(),
                actions: vec![UnlinkedAction {
                    declaration: declaration("Create item", "create_item", span(70, 81)),
                    span: span(70, 81),
                }],
                policies: Vec::new(),
            }),
            diagnostics: Vec::<Diagnostic>::new(),
        }
    }
}

fn span(start: usize, end: usize) -> TextRange {
    TextRange { start, end }
}

fn declaration(name: &str, id: &str, span: TextRange) -> UnlinkedDeclaration {
    UnlinkedDeclaration {
        name: name.into(),
        id: Some(id.into()),
        span,
    }
}

#[test]
fn compiler_accepts_a_non_korean_frontend_through_the_shared_contract() {
    let compilation = compile_with_frontend(&TestFrontend, "custom surface text");

    assert!(compilation.diagnostics.is_empty());
    let module = compilation.module.expect("test frontend should compile");
    assert_eq!(module.id.as_str(), "test");
    assert_eq!(module.models[0].id.as_str(), "test.item");
    assert_eq!(module.constraints[0].model_id.as_str(), "test.item");
}

#[test]
fn compiler_preserves_frontend_reference_spans_in_diagnostics() {
    let compilation = compile_with_frontend(&TestFrontend, "missing model");

    assert!(compilation.module.is_none());
    let diagnostic = compilation
        .diagnostics
        .iter()
        .find(|diagnostic| diagnostic.message_key == "semantic.model.not_found")
        .expect("missing model diagnostic");
    assert_eq!(diagnostic.span, span(40, 50));
}

#[test]
fn compiler_preserves_action_mutation_source_provenance() {
    let source_text = "x".repeat(90);
    let compilation = compile_source_with_frontend(
        &TestFrontend,
        Source::new("contract-test.rspdl", source_text.as_str()),
    );

    assert!(compilation.diagnostics.is_empty());
    let definition = &compilation
        .module
        .as_ref()
        .expect("test frontend should compile")
        .action_data_mutations[0];
    assert_eq!(definition.span, span(70, 90));
    let mutation = &compilation.action_data_mutation_provenance[0];
    assert!(mutation.span.end <= source_text.len());
    assert_eq!(mutation.source_id.as_str(), "contract-test.rspdl");
    assert_eq!(mutation.span, span(70, 90));
}
