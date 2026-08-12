//! RSPDL-specific executable EBNF compiler and deterministic parser runtime.

#![forbid(unsafe_code)]

mod compiler;
mod runtime;

pub use compiler::{CompileError, CompileErrorKind, CompiledGrammar, GrammarCompiler};
pub use runtime::{
    Capture, Expr, Grammar, InputAdapter, ParseError, ParseFailure, ParseMatch, Rule, TerminalMatch,
};
