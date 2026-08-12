mod adapter;
mod policy;

use rspdl_grammar_compiler::{Capture, ParseMatch};

fn required_capture(parsed: &ParseMatch, name: &str) -> Capture {
    parsed
        .capture(name)
        .unwrap_or_else(|| panic!("validated grammar always captures {name}"))
        .clone()
}
