mod adapter;
mod constraint;
mod creation;
mod declarations;
mod field_producer;
mod policy;
mod provenance;
mod relation;
mod relation_producer;

use rspdl_grammar_compiler::{Capture, ParseMatch};

fn required_capture(parsed: &ParseMatch, name: &str) -> Capture {
    parsed
        .capture(name)
        .unwrap_or_else(|| panic!("validated grammar always captures {name}"))
        .clone()
}
