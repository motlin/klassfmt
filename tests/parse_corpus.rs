//! M1 checkpoint: every file in tests/corpus/ must parse with zero
//! ERROR or MISSING nodes.

use std::fs;
use std::path::Path;

use tree_sitter::{Node, Parser};

/// Walks the whole tree and collects a description of every error/missing node.
fn collect_problems(source: &str, root: Node) -> Vec<String> {
	let mut problems = Vec::new();
	let mut cursor = root.walk();

	// Iterative pre-order traversal.
	let mut stack = vec![root];
	while let Some(node) = stack.pop() {
		if node.is_error() || node.is_missing() {
			let start = node.start_position();
			let kind = if node.is_missing() {
				"MISSING"
			} else {
				"ERROR"
			};
			let snippet = node
				.utf8_text(source.as_bytes())
				.unwrap_or("<non-utf8>")
				.lines()
				.next()
				.unwrap_or("")
				.trim();
			problems.push(format!(
				"{} at {}:{} kind={} near {:?}",
				kind,
				start.row + 1,
				start.column + 1,
				node.kind(),
				snippet,
			));
		}
		for child in node.children(&mut cursor) {
			stack.push(child);
		}
	}
	problems
}

#[test]
fn every_corpus_file_parses_cleanly() {
	let mut parser = Parser::new();
	parser
		.set_language(&klassfmt::language())
		.expect("load klass grammar");

	let corpus_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/corpus");
	let mut entries: Vec<_> = fs::read_dir(&corpus_dir)
		.expect("read corpus dir")
		.map(|e| e.expect("dir entry").path())
		.filter(|p| p.extension().map(|e| e == "klass").unwrap_or(false))
		.collect();
	entries.sort();

	assert!(!entries.is_empty(), "corpus directory is empty");

	let mut failures = Vec::new();
	for path in &entries {
		let source = fs::read_to_string(path).expect("read corpus file");
		let tree = parser.parse(&source, None).expect("parse produced a tree");
		let problems = collect_problems(&source, tree.root_node());
		if !problems.is_empty() {
			failures.push(format!(
				"{}:\n    {}",
				path.file_name().unwrap().to_string_lossy(),
				problems.join("\n    "),
			));
		}
	}

	assert!(
		failures.is_empty(),
		"{} of {} corpus files failed to parse cleanly:\n{}",
		failures.len(),
		entries.len(),
		failures.join("\n"),
	);

	eprintln!("all {} corpus files parsed cleanly", entries.len());
}
