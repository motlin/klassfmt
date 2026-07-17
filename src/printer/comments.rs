//! Comment/trivia attachment.
//!
//! tree-sitter keeps `line_comment` and `block_comment` nodes in the tree (they
//! are declared as `extras`), floating between the structural nodes rather than
//! attached to the node they document. This module buckets every comment onto a
//! nearby *named* node as either:
//!
//!   * **leading**  — the comment sits on its own line(s) above the node, or
//!                    immediately before it; emitted before the node.
//!   * **trailing** — the comment is on the same source line as the end of the
//!                    node; emitted after the node on the same line.
//!
//! Comments inside an otherwise-empty block attach as leading trivia of the
//! block's closing context via the enclosing node (handled by the printer).
//!
//! Attachment rule: each comment is assigned to the nearest named node it
//! borders. If a non-comment token sits on the same line to the comment's left,
//! the comment trails the named node ending on that line; otherwise it leads the
//! next named node that begins after it.

use std::collections::HashMap;

use tree_sitter::Node;

/// A single captured comment, with whether a blank line separated it from the
/// preceding content (so the printer can reproduce paragraph breaks).
#[derive(Clone)]
pub(crate) struct Comment {
	pub(crate) text: String,
	/// True if there was a blank line between this comment and whatever came
	/// before it (previous comment or code).
	pub(crate) blank_before: bool,
}

#[derive(Default)]
pub(crate) struct CommentMap {
	leading: HashMap<usize, Vec<Comment>>,
	trailing: HashMap<usize, Vec<Comment>>,
	block_trailing: HashMap<usize, Vec<Comment>>,
	/// Standalone comments after the last declaration (own line at EOF).
	trailing_file: Vec<Comment>,
}

impl CommentMap {
	pub(crate) fn new(root: Node, source: &str) -> Self {
		let mut map = CommentMap::default();

		// Gather all comment nodes in source order.
		let comments = collect_comments(root);
		// Gather all *named*, non-comment nodes in source order; these are the
		// candidates a comment can attach to.
		let mut anchors: Vec<Node> = Vec::new();
		collect_anchors(root, &mut anchors);
		anchors.sort_by_key(|n| n.start_byte());
		let mut blocks: Vec<Node> = Vec::new();
		collect_blocks(root, &mut blocks);
		blocks.sort_by_key(|n| n.start_byte());

		for comment in comments {
			let c_start = comment.start_byte();
			let c_end = comment.end_byte();

			// Is there code on the same line to the left of the comment? If so,
			// the comment trails the nearest named node that ends on that line.
			let line_start = source[..c_start].rfind('\n').map(|i| i + 1).unwrap_or(0);
			let before_on_line = &source[line_start..c_start];
			let has_code_before = !before_on_line.trim().is_empty();

			let text = source[c_start..c_end].to_string();

			if has_code_before {
				// Trailing: attach to the deepest named node ending at or before
				// the comment on this same line.
				if let Some(anchor) = anchors
					.iter()
					.filter(|n| n.end_byte() <= c_start && n.end_byte() >= line_start)
					.max_by_key(|n| n.end_byte())
				{
					map.trailing.entry(anchor.id()).or_default().push(Comment {
						text,
						blank_before: false,
					});
					continue;
				}
			}

			// Leading: attach to the next named node starting at or after the
			// comment's end. Prefer the smallest such node (deepest/closest).
			let blank_before = blank_between(source, prev_content_end(source, c_start), c_start);
			if let Some(block) = innermost_containing_block(&blocks, c_start, c_end) {
				let next_anchor_in_block = anchors
					.iter()
					.any(|n| n.start_byte() >= c_end && n.end_byte() <= block.end_byte());
				if !next_anchor_in_block {
					map.block_trailing
						.entry(block.id())
						.or_default()
						.push(Comment { text, blank_before });
					continue;
				}
			}
			if let Some(anchor) = anchors
				.iter()
				.filter(|n| n.start_byte() >= c_end)
				.min_by_key(|n| (n.start_byte(), n.end_byte()))
			{
				map.leading
					.entry(anchor.id())
					.or_default()
					.push(Comment { text, blank_before });
			} else {
				// No following node. A comment that stood on its own line at EOF
				// stays a standalone trailing-file comment; a same-line one
				// trails the last node.
				map.trailing_file.push(Comment { text, blank_before });
			}
		}

		map
	}

	pub(crate) fn leading(&self, node: Node) -> &[Comment] {
		self.leading
			.get(&node.id())
			.map(|v| v.as_slice())
			.unwrap_or(&[])
	}

	pub(crate) fn trailing(&self, node: Node) -> &[Comment] {
		self.trailing
			.get(&node.id())
			.map(|v| v.as_slice())
			.unwrap_or(&[])
	}

	pub(crate) fn block_trailing(&self, node: Node) -> &[Comment] {
		self.block_trailing
			.get(&node.id())
			.map(|v| v.as_slice())
			.unwrap_or(&[])
	}

	pub(crate) fn trailing_file(&self) -> &[Comment] {
		&self.trailing_file
	}
}

fn collect_comments<'a>(root: Node<'a>) -> Vec<Node<'a>> {
	let mut out = Vec::new();
	let mut cursor = root.walk();
	let mut stack = vec![root];
	while let Some(node) = stack.pop() {
		if is_comment(node) {
			out.push(node);
		}
		for child in node.children(&mut cursor) {
			stack.push(child);
		}
	}
	out.sort_by_key(|n| n.start_byte());
	out
}

/// The node kinds the printer emits as standalone units, and thus the only
/// nodes a comment attaches to. Restricting anchors to these keeps a comment
/// above `class Foo` attached to the whole declaration rather than to the
/// `class` keyword or the identifier inside it.
const ANCHOR_KINDS: &[&str] = &[
	"package_declaration",
	"top_level_declaration",
	"class_member",
	"interface_member",
	"enumeration_literal",
	"association_end",
	"relationship",
	"projection_member",
	"url_declaration",
	"service_declaration",
	"service_multiplicity_declaration",
	"service_criteria_declaration",
	"service_projection_dispatch",
	"service_order_by_declaration",
	"criteria_expression",
];

/// Collects the anchor nodes (of [`ANCHOR_KINDS`]) that a comment may attach to.
fn collect_anchors<'a>(node: Node<'a>, out: &mut Vec<Node<'a>>) {
	let mut cursor = node.walk();
	for child in node.named_children(&mut cursor) {
		if is_comment(child) {
			continue;
		}
		if ANCHOR_KINDS.contains(&child.kind()) {
			out.push(child);
		}
		collect_anchors(child, out);
	}
}

const BLOCK_KINDS: &[&str] = &[
	"class_block",
	"interface_block",
	"enumeration_block",
	"association_block",
	"projection_block",
	"service_group_block",
	"service_block",
];

fn collect_blocks<'a>(node: Node<'a>, out: &mut Vec<Node<'a>>) {
	let mut cursor = node.walk();
	for child in node.named_children(&mut cursor) {
		if is_comment(child) {
			continue;
		}
		if BLOCK_KINDS.contains(&child.kind()) {
			out.push(child);
		}
		collect_blocks(child, out);
	}
}

fn innermost_containing_block<'a>(
	blocks: &[Node<'a>],
	start_byte: usize,
	end_byte: usize,
) -> Option<Node<'a>> {
	blocks
		.iter()
		.copied()
		.filter(|b| b.start_byte() <= start_byte && b.end_byte() >= end_byte)
		.min_by_key(|b| b.end_byte() - b.start_byte())
}

/// The byte offset just past the previous non-whitespace content before `pos`.
fn prev_content_end(source: &str, pos: usize) -> usize {
	source[..pos].trim_end().len()
}

/// Whether the gap `source[from..to]` contains a blank line (two+ newlines).
fn blank_between(source: &str, from: usize, to: usize) -> bool {
	if from >= to {
		return false;
	}
	source[from..to].bytes().filter(|b| *b == b'\n').count() > 1
}

fn is_comment(node: Node) -> bool {
	matches!(node.kind(), "line_comment" | "block_comment")
}
