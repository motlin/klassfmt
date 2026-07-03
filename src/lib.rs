//! klassfmt: a standalone formatter for the Klass DSL.
//!
//! This crate currently exposes only the tree-sitter parser foundation
//! (the M1 slice). The Doc-IR printer is intentionally not implemented yet.

use tree_sitter::Language;

extern "C" {
    fn tree_sitter_klass() -> Language;
}

/// Returns the tree-sitter [`Language`] for the Klass grammar.
pub fn language() -> Language {
    unsafe { tree_sitter_klass() }
}
