use std::path::PathBuf;

fn main() {
	let src_dir: PathBuf = ["src"].iter().collect();

	let mut build = cc::Build::new();
	build.include(&src_dir);
	build.warnings(false);
	build.file(src_dir.join("parser.c"));

	// The scanner is only compiled if the grammar generated one.
	let scanner = src_dir.join("scanner.c");
	if scanner.exists() {
		build.file(scanner);
	}

	build.compile("tree-sitter-klass");

	println!("cargo:rerun-if-changed=src/parser.c");
	println!("cargo:rerun-if-changed=src/scanner.c");
	println!("cargo:rerun-if-changed=grammar.js");
}
