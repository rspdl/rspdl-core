use std::env;
use std::fs;
use std::path::PathBuf;

use rspdl_grammar_compiler::GrammarCompiler;

fn main() {
    let manifest_dir = PathBuf::from(
        env::var("CARGO_MANIFEST_DIR").expect("Cargo sets CARGO_MANIFEST_DIR for build scripts"),
    );
    let grammar_path = manifest_dir.join("src/grammar/policy.ebnf");
    println!("cargo::rerun-if-changed={}", grammar_path.display());

    let source = fs::read_to_string(&grammar_path)
        .unwrap_or_else(|error| panic!("failed to read {}: {error}", grammar_path.display()));
    let compiled = GrammarCompiler::new(["marked_ref"])
        .compile(&source)
        .unwrap_or_else(|error| panic!("invalid {}: {error}", grammar_path.display()));
    let generated = compiled
        .emit_rust("generated_policy_grammar")
        .expect("the generated function name is a valid Rust identifier");

    let output_path =
        PathBuf::from(env::var("OUT_DIR").expect("Cargo sets OUT_DIR")).join("policy_grammar.rs");
    fs::write(&output_path, generated)
        .unwrap_or_else(|error| panic!("failed to write {}: {error}", output_path.display()));
}
