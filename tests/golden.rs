//! A5: golden/snapshot tests for representative corpus files.
//!
//! Each listed file is a canonical fixed point under the tab-indented style:
//! formatting it reproduces the file with only its leading 4-space indentation
//! converted to tabs (the deliberate default indentation switch). These lock in
//! the formatting of a spread of constructs (classes, associations, projections
//! with nested blocks, inheritance, parameterized properties, criteria) so
//! regressions surface immediately.

use std::fs;
use std::path::Path;

fn corpus(name: &str) -> String {
	let path = Path::new(env!("CARGO_MANIFEST_DIR"))
		.join("tests/corpus")
		.join(name);
	fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()))
}

/// Convert each line's leading run of 4 spaces into one tab, mirroring the
/// formatter's default indentation so a space-indented corpus fixture can be
/// compared against tab-indented output.
fn spaces_to_tabs(s: &str) -> String {
	s.lines()
		.map(|line| {
			let indent_len = line.len() - line.trim_start_matches(' ').len();
			let tabs = indent_len / 4;
			let rem = indent_len % 4;
			format!(
				"{}{}{}",
				"\t".repeat(tabs),
				" ".repeat(rem),
				&line[indent_len..]
			)
		})
		.collect::<Vec<_>>()
		.join("\n")
		+ "\n"
}

fn assert_fixed_point(name: &str) {
	let src = corpus(name);
	let expected = spaces_to_tabs(&src);
	let formatted = klassfmt::format(&src).expect("format");
	assert_eq!(
		formatted, expected,
		"{name} should format to itself unchanged modulo space->tab indentation"
	);
}

fn assert_formatted_contains(name: &str, expected: &str) {
	let src = corpus(name);
	let formatted = klassfmt::format(&src).expect("format");
	assert!(
		formatted.contains(expected),
		"{name} formatted output did not contain expected snippet:\n{expected}\n\nformatted:\n{formatted}"
	);
}

#[test]
fn golden_association_with_relationship() {
	assert_formatted_contains(
		"klass-model-converters__klass-compiler-tests__src__test__inputresources__cool__klass__model__converter__compiler__annotation__association__CompoundJoinMixedKeyTest.klass",
		"\trelationship this.id == Target.extraId\n\t\t&& this.extraId == Target.id\n",
	);
}

#[test]
fn golden_plural_association() {
	assert_fixed_point(
		"klass-model-converters__klass-compiler-tests__src__test__inputresources__cool__klass__model__converter__compiler__annotation__association__PluralAssociationTest.klass",
	);
}

#[test]
fn golden_inheritance_stacked_modifiers() {
	assert_fixed_point(
		"klass-model-converters__klass-compiler-tests__src__test__inputresources__cool__klass__model__converter__compiler__annotation__inheritance__CircularInheritanceErrorTest.klass",
	);
}

#[test]
fn golden_service_block() {
	assert_formatted_contains(
		"klass-model-converters__klass-compiler-tests__src__test__inputresources__cool__klass__model__converter__compiler__annotation__property__UnreferencedPrivatePropertiesTest.klass",
		"\texample: ExampleClass[0..*]\n\t\torderBy: this.privateUsedInAssociationOrderBy ascending;\n",
	);
	assert_formatted_contains(
		"klass-model-converters__klass-compiler-tests__src__test__inputresources__cool__klass__model__converter__compiler__annotation__property__UnreferencedPrivatePropertiesTest.klass",
		"\trelationship this.relatedClassId == RelatedClass.id\n\t\t&& this.privateUsedInAssociationCriteria == \"test\"\n",
	);
}
