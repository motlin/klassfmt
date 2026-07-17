//! Markdown integration: format the contents of ```klass fenced code blocks
//! while leaving prose and every other fence byte-for-byte identical.
//!
//! Fence handling follows the CommonMark rules that matter here:
//!   * A fence opens with 3+ backticks or 3+ tildes, optionally indented up to
//!     3 spaces, followed by an info string. The first word of the info string
//!     is the language; we act only when it is exactly `klass`.
//!   * The closing fence uses the same character, is at least as long as the
//!     opener, has the same indentation context, and carries no info string.
//!   * The opener's indentation is stripped from each content line before
//!     formatting and re-applied afterward, so an indented block stays indented.
//!
//! A block whose contents do not parse (documentation fragments that are not
//! valid Klass, e.g. `body: String(30000)`) is left exactly as written — the
//! markdown pass never corrupts a block it cannot format.

use crate::{Config, format_fragment_with_config};

/// Formats every ```klass code block in `source`, returning the rewritten
/// document. All bytes outside klass blocks are preserved exactly, as is the
/// document's line-ending style and whether it ends with a trailing newline.
pub fn format_markdown(source: &str, config: Config) -> String {
	// Split into lines while remembering the original line terminators so the
	// output preserves them (and the presence/absence of a final newline).
	let mut out = String::with_capacity(source.len());
	let mut lines = LineScanner::new(source);

	while let Some(line) = lines.peek() {
		if let Some(fence) = FenceOpen::parse(line.content) {
			// Consume the opening fence line verbatim.
			out.push_str(line.raw);
			lines.advance();

			// Gather content lines until the matching close (or EOF).
			let mut content_lines: Vec<Line> = Vec::new();
			let mut closing: Option<Line> = None;
			while let Some(inner) = lines.peek() {
				if fence.is_close(inner.content) {
					closing = Some(inner);
					lines.advance();
					break;
				}
				content_lines.push(inner);
				lines.advance();
			}

			if fence.is_klass {
				out.push_str(&format_block(&fence, &content_lines, config));
			} else {
				for l in &content_lines {
					out.push_str(l.raw);
				}
			}
			if let Some(close) = closing {
				out.push_str(close.raw);
			}
		} else {
			out.push_str(line.raw);
			lines.advance();
		}
	}

	out
}

/// Formats the content of a single klass block and re-applies the opener's
/// indentation. On any formatting failure the original block bytes are emitted
/// unchanged.
fn format_block(fence: &FenceOpen, content: &[Line], config: Config) -> String {
	// Reconstruct the block body with the fence indentation removed.
	let mut body = String::new();
	for (i, line) in content.iter().enumerate() {
		if i > 0 {
			body.push('\n');
		}
		body.push_str(strip_indent(line.content, fence.indent));
	}

	let formatted = match format_fragment_with_config(&body, config) {
		Ok(f) => f,
		Err(_) => {
			// Leave the block untouched.
			let mut original = String::new();
			for line in content {
				original.push_str(line.raw);
			}
			return original;
		}
	};

	// Re-apply the opener's indentation and the block's newline style.
	let indent: String = " ".repeat(fence.indent);
	let newline = content.first().map(|l| l.newline).unwrap_or("\n");
	let mut out = String::new();
	// `formatted` always ends in exactly one '\n'; iterate its logical lines.
	for line in formatted.trim_end_matches('\n').split('\n') {
		if line.is_empty() {
			// Do not indent blank lines.
			out.push_str(newline);
		} else {
			out.push_str(&indent);
			out.push_str(line);
			out.push_str(newline);
		}
	}
	out
}

/// Removes up to `n` leading space characters from `line`.
fn strip_indent(line: &str, n: usize) -> &str {
	let mut removed = 0;
	let mut idx = 0;
	for (i, ch) in line.char_indices() {
		if removed >= n || ch != ' ' {
			idx = i;
			break;
		}
		removed += 1;
		idx = i + ch.len_utf8();
	}
	&line[idx..]
}

/// A parsed opening fence.
struct FenceOpen {
	/// Leading indentation (0..=3 spaces).
	indent: usize,
	/// The fence character, '`' or '~'.
	ch: char,
	/// The number of fence characters.
	len: usize,
	/// Whether the info string's first word is exactly `klass`.
	is_klass: bool,
}

impl FenceOpen {
	/// Parses a line as an opening code fence, if it is one.
	fn parse(line: &str) -> Option<FenceOpen> {
		let indent = line.len() - line.trim_start_matches(' ').len();
		if indent > 3 {
			return None;
		}
		let rest = &line[indent..];
		let ch = rest.chars().next()?;
		if ch != '`' && ch != '~' {
			return None;
		}
		let len = rest.chars().take_while(|c| *c == ch).count();
		if len < 3 {
			return None;
		}
		let info = rest[len..].trim();
		// Backtick info strings may not contain backticks (CommonMark).
		if ch == '`' && info.contains('`') {
			return None;
		}
		let first_word = info.split_whitespace().next().unwrap_or("");
		Some(FenceOpen {
			indent,
			ch,
			len,
			is_klass: first_word == "klass",
		})
	}

	/// Whether `line` is the closing fence for this opener.
	fn is_close(&self, line: &str) -> bool {
		let indent = line.len() - line.trim_start_matches(' ').len();
		if indent > 3 {
			return false;
		}
		let rest = &line[indent..];
		let len = rest.chars().take_while(|c| *c == self.ch).count();
		if len < self.len {
			return false;
		}
		// A closing fence carries no info string.
		rest[len..].trim().is_empty()
	}
}

/// One source line: its content (without the terminator), the terminator, and
/// the raw slice including the terminator (so it can be re-emitted verbatim).
#[derive(Clone, Copy)]
struct Line<'a> {
	content: &'a str,
	newline: &'a str,
	raw: &'a str,
}

/// Iterates a source string as [`Line`]s, preserving `\n` / `\r\n` terminators
/// and any final line lacking a terminator.
struct LineScanner<'a> {
	source: &'a str,
	pos: usize,
}

impl<'a> LineScanner<'a> {
	fn new(source: &'a str) -> Self {
		LineScanner { source, pos: 0 }
	}

	fn peek(&self) -> Option<Line<'a>> {
		if self.pos >= self.source.len() {
			return None;
		}
		let rest = &self.source[self.pos..];
		match rest.find('\n') {
			Some(nl) => {
				let raw = &rest[..=nl];
				let (content, newline) = if nl > 0 && rest.as_bytes()[nl - 1] == b'\r' {
					(&rest[..nl - 1], "\r\n")
				} else {
					(&rest[..nl], "\n")
				};
				Some(Line {
					content,
					newline,
					raw,
				})
			}
			None => Some(Line {
				content: rest,
				newline: "",
				raw: rest,
			}),
		}
	}

	fn advance(&mut self) {
		if let Some(line) = self.peek() {
			self.pos += line.raw.len();
		}
	}
}
