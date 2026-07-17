//! A5: CLI behavior — modes and exit codes.

use std::io::Write;
use std::process::{Command, Stdio};

fn klassfmt_bin() -> &'static str {
	env!("CARGO_BIN_EXE_klassfmt")
}

/// Runs the CLI with the given args and stdin, returning (stdout, exit code).
fn run(args: &[&str], stdin: &str) -> (String, i32) {
	let mut child = Command::new(klassfmt_bin())
		.args(args)
		.stdin(Stdio::piped())
		.stdout(Stdio::piped())
		.stderr(Stdio::null())
		.spawn()
		.expect("spawn klassfmt");
	child
		.stdin
		.take()
		.unwrap()
		.write_all(stdin.as_bytes())
		.unwrap();
	let out = child.wait_with_output().unwrap();
	(
		String::from_utf8(out.stdout).unwrap(),
		out.status.code().unwrap_or(-1),
	)
}

#[test]
fn stdin_filepath_formats_to_stdout() {
	let (out, code) = run(
		&["--stdin-filepath", "x.klass"],
		"package p\nclass C{id:Long key;}\n",
	);
	assert_eq!(code, 0);
	assert_eq!(out, "package p\n\nclass C\n{\n\tid: Long key;\n}\n");
}

#[test]
fn check_reports_and_exits_nonzero_on_difference() {
	let dir = tempdir();
	let path = dir.join("messy.klass");
	std::fs::write(&path, "package p\nclass C{id:Long key;}\n").unwrap();

	let (out, code) = run(&["--check", path.to_str().unwrap()], "");
	assert_eq!(code, 1, "check should exit 1 when a file differs");
	assert!(
		out.contains("messy.klass"),
		"should name the differing file"
	);
}

#[test]
fn check_is_silent_and_zero_on_formatted_input() {
	let dir = tempdir();
	let path = dir.join("clean.klass");
	let formatted = "package p\n\nclass C\n{\n\tid: Long key;\n}\n";
	std::fs::write(&path, formatted).unwrap();

	let (out, code) = run(&["--check", path.to_str().unwrap()], "");
	assert_eq!(code, 0, "check should exit 0 for already-formatted input");
	assert!(out.is_empty(), "check should print nothing for clean input");
}

#[test]
fn write_formats_in_place() {
	let dir = tempdir();
	let path = dir.join("w.klass");
	std::fs::write(&path, "package p\nclass C{id:Long key;}\n").unwrap();

	let (_out, code) = run(&["--write", path.to_str().unwrap()], "");
	assert_eq!(code, 0);
	let after = std::fs::read_to_string(&path).unwrap();
	assert_eq!(after, "package p\n\nclass C\n{\n\tid: Long key;\n}\n");
}

#[test]
fn syntax_error_is_reported_and_nonzero() {
	let (_out, code) = run(&["--stdin-filepath", "bad.klass"], "package p\nclass {{{");
	assert_ne!(code, 0, "malformed input should fail");
}

#[test]
fn default_indentation_is_tabs() {
	let (out, code) = run(
		&["--stdin-filepath", "x.klass"],
		"package p\nclass C{id:Long key;}\n",
	);
	assert_eq!(code, 0);
	assert!(
		out.contains("\n\tid: Long key;"),
		"default should indent with a tab: {out:?}"
	);
}

#[test]
fn use_tabs_false_indents_with_spaces() {
	let (out, code) = run(
		&["--stdin-filepath", "x.klass", "--use-tabs", "false"],
		"package p\nclass C{id:Long key;}\n",
	);
	assert_eq!(code, 0);
	assert!(
		out.contains("\n    id: Long key;"),
		"--use-tabs false should use spaces: {out:?}"
	);
}

#[test]
fn markdown_stdin_formats_only_klass_fences() {
	let (out, code) = run(
		&["--stdin-filepath", "doc.md"],
		"# Doc\n\n```klass\nclass C{id:Long key;}\n```\n\n```json\n{}\n```\n",
	);
	assert_eq!(code, 0);
	assert!(
		out.contains("\n\tid: Long key;\n"),
		"klass block not formatted: {out:?}"
	);
	assert!(
		out.contains("```json\n{}\n```"),
		"json block changed: {out:?}"
	);
	assert!(out.starts_with("# Doc\n"));
}

#[test]
fn markdown_file_write_and_check() {
	let dir = tempdir();
	let path = dir.join("doc.md");
	std::fs::write(&path, "# Doc\n\n```klass\nclass C{id:Long key;}\n```\n").unwrap();

	// Unformatted: --check exits 1 and names the file.
	let (out, code) = run(&["--check", path.to_str().unwrap()], "");
	assert_eq!(code, 1);
	assert!(out.contains("doc.md"));

	// --write formats in place, then --check is clean.
	let (_o, code) = run(&["--write", path.to_str().unwrap()], "");
	assert_eq!(code, 0);
	let after = std::fs::read_to_string(&path).unwrap();
	assert!(
		after.contains("\n\tid: Long key;\n"),
		"not formatted in place: {after:?}"
	);
	let (_o, code) = run(&["--check", path.to_str().unwrap()], "");
	assert_eq!(code, 0, "should be clean after write");
}

/// A unique temp directory for a test (avoids pulling in the `tempfile` crate).
fn tempdir() -> std::path::PathBuf {
	let base = std::env::temp_dir();
	let unique = format!(
		"klassfmt-test-{}-{}",
		std::process::id(),
		std::time::SystemTime::now()
			.duration_since(std::time::UNIX_EPOCH)
			.unwrap()
			.as_nanos()
	);
	let dir = base.join(unique);
	std::fs::create_dir_all(&dir).unwrap();
	dir
}
