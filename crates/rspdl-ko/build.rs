use std::env;
use std::fs;
use std::path::PathBuf;

use rspdl_grammar_compiler::GrammarCompiler;

fn main() {
    let manifest_dir = PathBuf::from(
        env::var("CARGO_MANIFEST_DIR").expect("Cargo sets CARGO_MANIFEST_DIR for build scripts"),
    );
    let grammar_dir = manifest_dir.join("src/grammar");
    println!("cargo::rerun-if-changed={}", grammar_dir.display());
    let mut grammar_paths = fs::read_dir(&grammar_dir)
        .unwrap_or_else(|error| panic!("failed to read {}: {error}", grammar_dir.display()))
        .map(|entry| entry.expect("grammar directory entry is readable").path())
        .filter(|path| {
            path.extension()
                .is_some_and(|extension| extension == "ebnf")
        })
        .collect::<Vec<_>>();
    grammar_paths.sort();

    let output_dir = PathBuf::from(env::var("OUT_DIR").expect("Cargo sets OUT_DIR"));
    for grammar_path in grammar_paths {
        compile_grammar(&grammar_path, &output_dir);
    }
}

fn compile_grammar(grammar_path: &std::path::Path, output_dir: &std::path::Path) {
    let stem = grammar_path
        .file_stem()
        .and_then(|value| value.to_str())
        .expect("grammar files have a UTF-8 stem");
    let source = fs::read_to_string(grammar_path)
        .unwrap_or_else(|error| panic!("failed to read {}: {error}", grammar_path.display()));
    let compiled = GrammarCompiler::new([
        "action_input_type",
        "annotated_decl",
        "canonical_id",
        "comma_ref",
        "enum_value",
        "field_item",
        "integer",
        "integer_before",
        "literal",
        "marked_ref",
        "natural_decl",
        "quoted_equal",
        "quoted_not_equal",
        "screen_model_ref",
        "source_direct",
        "string_equal",
        "string_not_equal",
        "surface_name",
        "word_equal",
        "word_not_equal",
    ])
    .compile(&source)
    .unwrap_or_else(|error| panic!("invalid {}: {error}", grammar_path.display()));
    let generated = compiled
        .emit_rust(&format!("generated_{stem}_grammar"))
        .expect("grammar stems are valid Rust identifier fragments");

    let output_path = output_dir.join(format!("{stem}_grammar.rs"));
    fs::write(&output_path, generated)
        .unwrap_or_else(|error| panic!("failed to write {}: {error}", output_path.display()));
}
