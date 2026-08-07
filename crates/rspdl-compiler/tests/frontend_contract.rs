use rspdl_compiler::compile_with_frontend;
use rspdl_domain::{
    Diagnostic, Frontend, FrontendOutput, TextRange, UnlinkedDataModel, UnlinkedDeclaration,
    UnlinkedField, UnlinkedModule, UnlinkedTypeReference,
};

struct TestFrontend;

impl Frontend for TestFrontend {
    fn language_id(&self) -> &'static str {
        "test"
    }

    fn lower_source(&self, _source: &str) -> FrontendOutput {
        FrontendOutput {
            module: Some(UnlinkedModule {
                declaration: declaration("Test module", "test"),
                enums: Vec::new(),
                models: vec![UnlinkedDataModel {
                    declaration: declaration("Item", "item"),
                    fields: vec![UnlinkedField {
                        declaration: declaration("Value", "value"),
                        required: true,
                        value_type: UnlinkedTypeReference::Integer,
                    }],
                }],
                screens: Vec::new(),
                derivations: Vec::new(),
                recalculations: Vec::new(),
                field_intents: Vec::new(),
                constraints: Vec::new(),
                roles: Vec::new(),
                actions: Vec::new(),
                policies: Vec::new(),
            }),
            diagnostics: Vec::<Diagnostic>::new(),
        }
    }
}

fn declaration(name: &str, id: &str) -> UnlinkedDeclaration {
    UnlinkedDeclaration {
        name: name.into(),
        id: Some(id.into()),
        span: TextRange::default(),
    }
}

#[test]
fn compiler_accepts_a_non_korean_frontend_through_the_shared_contract() {
    let compilation = compile_with_frontend(&TestFrontend, "custom surface text");

    assert!(compilation.diagnostics.is_empty());
    let module = compilation.module.expect("test frontend should compile");
    assert_eq!(module.id.as_str(), "test");
    assert_eq!(module.models[0].id.as_str(), "test.item");
}
