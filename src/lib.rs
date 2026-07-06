//! klassfmt: a standalone formatter for the Klass DSL.
//!
//! Pipeline: source text -> tree-sitter CST -> Wadler `Doc` IR -> rendered text.
//! The style target is the hand-written canonical style of the vendored corpus
//! in `tests/corpus/`.

use tree_sitter::{Language, Parser};

mod markdown;
mod printer;

pub use markdown::format_markdown;

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

/// The default number of columns one indentation level occupies.
///
/// Matches the klass repo's `.prettierrc.json5` `tabWidth`.
pub const DEFAULT_TAB_WIDTH: usize = 4;

/// Formatting configuration. Defaults match the klass repo's `.prettierrc.json5`
/// (`printWidth: 120`, `useTabs: true`, `tabWidth: 4`), so the corpus's
/// tab-indented convention is reproduced.
#[derive(Debug, Clone, Copy)]
pub struct Config {
    /// Maximum line width before wrapping.
    pub print_width: usize,
    /// Indent with a tab per level when true; otherwise `tab_width` spaces.
    pub use_tabs: bool,
    /// Columns per indentation level (also the visual width of a tab, used for
    /// wrapping math).
    pub tab_width: usize,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            print_width: DEFAULT_PRINT_WIDTH,
            use_tabs: true,
            tab_width: DEFAULT_TAB_WIDTH,
        }
    }
}

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

/// Formats Klass source text using the default configuration.
pub fn format(source: &str) -> Result<String, FormatError> {
    format_with_config(source, Config::default())
}

/// Formats Klass source text, wrapping at `width` columns (other settings
/// default). Retained for convenience; prefer [`format_with_config`].
pub fn format_with_width(source: &str, width: usize) -> Result<String, FormatError> {
    format_with_config(
        source,
        Config {
            print_width: width,
            ..Config::default()
        },
    )
}

/// Formats Klass source text with the given [`Config`].
pub fn format_with_config(source: &str, config: Config) -> Result<String, FormatError> {
    let mut parser = Parser::new();
    parser
        .set_language(&language())
        .expect("load klass grammar");
    let tree = parser.parse(source, None).ok_or(FormatError::ParseFailed)?;
    let root = tree.root_node();

    if let Some(message) = first_syntax_error(root, source) {
        return Err(FormatError::SyntaxError { message });
    }

    Ok(printer::print(root, source, config))
}

/// Sentinel package name used to make a package-less fragment parseable.
const FRAGMENT_PACKAGE: &str = "package __klassfmt_fragment__";

/// Formats a Klass fragment: source that may omit the required leading
/// `package` declaration, as embedded documentation examples typically do.
///
/// First tries to format `source` as a complete compilation unit. If that fails
/// (e.g. no `package`), retries by wrapping the source in a synthetic package,
/// formatting, and stripping the synthetic lines back off — so a bare
/// `class Foo { ... }` example still gets canonicalized. Returns an error only
/// if neither form parses.
pub fn format_fragment_with_config(source: &str, config: Config) -> Result<String, FormatError> {
    if let Ok(formatted) = format_with_config(source, config) {
        return Ok(formatted);
    }

    // Retry as a fragment under a synthetic package.
    let wrapped = format!("{FRAGMENT_PACKAGE}\n\n{source}");
    let formatted = format_with_config(&wrapped, config)?;

    // Strip the synthetic package line and the blank line that follows it.
    let body = formatted
        .strip_prefix(FRAGMENT_PACKAGE)
        .and_then(|rest| rest.strip_prefix('\n'))
        .map(|rest| rest.strip_prefix('\n').unwrap_or(rest))
        .unwrap_or(&formatted);

    Ok(body.to_string())
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
