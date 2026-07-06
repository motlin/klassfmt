//! klassfmt: a standalone formatter for the Klass DSL.
//!
//! Pipeline: source text -> tree-sitter CST -> Wadler `Doc` IR -> rendered text.
//! The style target is the hand-written canonical style of the vendored corpus
//! in `tests/corpus/`.

use tree_sitter::{Language, Parser};

mod printer;

extern "C" {
    fn tree_sitter_klass() -> Language;
}

/// Returns the tree-sitter [`Language`] for the Klass grammar.
pub fn language() -> Language {
    unsafe { tree_sitter_klass() }
}

/// The maximum line width the printer targets before wrapping.
///
/// Matches the klass repo's `.prettierrc.json5` `printWidth`.
pub const DEFAULT_PRINT_WIDTH: usize = 120;

/// Errors that can occur while formatting.
#[derive(Debug)]
pub enum FormatError {
    /// The source text could not be parsed into a tree at all.
    ParseFailed,
    /// The parsed tree contained ERROR or MISSING nodes; formatting a file with
    /// syntax errors is refused so we never emit corrupted output.
    SyntaxError { message: String },
}

impl std::fmt::Display for FormatError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FormatError::ParseFailed => write!(f, "failed to parse input"),
            FormatError::SyntaxError { message } => write!(f, "syntax error: {message}"),
        }
    }
}

impl std::error::Error for FormatError {}

/// Formats Klass source text using the default print width.
pub fn format(source: &str) -> Result<String, FormatError> {
    format_with_width(source, DEFAULT_PRINT_WIDTH)
}

/// Formats Klass source text, wrapping at `width` columns.
pub fn format_with_width(source: &str, width: usize) -> Result<String, FormatError> {
    let mut parser = Parser::new();
    parser
        .set_language(&language())
        .expect("load klass grammar");
    let tree = parser.parse(source, None).ok_or(FormatError::ParseFailed)?;
    let root = tree.root_node();

    if let Some(message) = first_syntax_error(root, source) {
        return Err(FormatError::SyntaxError { message });
    }

    Ok(printer::print(root, source, width))
}

/// Returns a human-readable description of the first ERROR/MISSING node, if any.
fn first_syntax_error(root: tree_sitter::Node, source: &str) -> Option<String> {
    let mut cursor = root.walk();
    let mut stack = vec![root];
    while let Some(node) = stack.pop() {
        if node.is_error() || node.is_missing() {
            let pos = node.start_position();
            let kind = if node.is_missing() { "missing" } else { "error" };
            let snippet = node
                .utf8_text(source.as_bytes())
                .unwrap_or("")
                .lines()
                .next()
                .unwrap_or("")
                .trim();
            return Some(format!(
                "{kind} node at line {}, column {} near {snippet:?}",
                pos.row + 1,
                pos.column + 1
            ));
        }
        for child in node.children(&mut cursor) {
            stack.push(child);
        }
    }
    None
}
