//! A2: core subset (package + class + data-type properties) formats to the
//! canonical style. Indentation is a tab per level (the default config).

/// Assert that `input` formats exactly to `expected`.
fn assert_formats(input: &str, expected: &str) {
	let got = klassfmt::format(input).expect("format should succeed");
	if got != expected {
		eprintln!("--- expected ---\n{expected}\n--- got ---\n{got}\n---");
	}
	assert_eq!(got, expected);
}

#[test]
fn formats_class_with_tab_indentation() {
	// Deliberately messy input: irregular spacing, spaces, no alignment.
	let input = "package com.example\n\
                 class Person systemTemporal versioned {\n\
                 id:Long id key;\n\
                 firstName :String?;\n\
                 lastName: String ?;\n\
                 }\n";

	let expected = "package com.example\n\
                    \n\
                    class Person\n\
                    \tsystemTemporal\n\
                    \tversioned\n\
                    {\n\
                    \tid: Long id key;\n\
                    \tfirstName: String?;\n\
                    \tlastName: String?;\n\
                    }\n";
	assert_formats(input, expected);
}

#[test]
fn already_canonical_class_is_a_near_noop() {
	// Canonical style is single-space-after-name (no colon alignment) and tab
	// indentation; see the notes on MemberDoc::into_doc and print().
	let canonical = "package com.example\n\
                     \n\
                     class Person\n\
                     \tsystemTemporal\n\
                     {\n\
                     \tid: Long id key;\n\
                     \tfirstName: String?;\n\
                     }\n";
	assert_formats(canonical, canonical);
}

#[test]
fn formats_property_with_validation() {
	let input = "package p\nclass C { name : String minLength ( 1 ) maxLength(255); }\n";
	let expected = "package p\n\
                    \n\
                    class C\n\
                    {\n\
                    \tname: String minLength(1) maxLength(255);\n\
                    }\n";
	assert_formats(input, expected);
}

#[test]
fn preserves_comment_before_relationship_in_association() {
	// Regression: standalone comments between association ends and the
	// `relationship` clause were dropped because association_block bypassed
	// with_comments. A formatter must never delete comments.
	let input = "package p\n\
	             association Foo\n\
	             {\n\
	             source: A[1..1];\n\
	             target: B[0..*];\n\
	             // keep me\n\
	             relationship this.id == B.id\n\
	             }\n";
	let expected = "package p\n\
	                \n\
	                association Foo\n\
	                {\n\
	                \tsource: A[1..1];\n\
	                \ttarget: B[0..*];\n\
	                \n\
	                \t// keep me\n\
	                \trelationship this.id == B.id\n\
	                }\n";
	assert_formats(input, expected);
}

#[test]
fn preserves_comments_inside_service_blocks() {
	// Regression: comments before a service clause (multiplicity/criteria) and
	// before a url declaration were dropped because service_block and
	// service_group_block bypassed with_comments.
	let input = "package p\n\
	             service R on C\n\
	             {\n\
	             // note before url\n\
	             /example\n\
	             GET\n\
	             {\n\
	             // note before multiplicity\n\
	             multiplicity: one;\n\
	             criteria: this.id == id;\n\
	             }\n\
	             }\n";
	let got = klassfmt::format(input).expect("format");
	assert!(
		got.contains("// note before url"),
		"url-leading comment dropped:\n{got}"
	);
	assert!(
		got.contains("// note before multiplicity"),
		"clause-leading comment dropped:\n{got}"
	);
}

#[test]
fn use_tabs_false_indents_with_spaces() {
	use klassfmt::Config;
	let input = "package p\nclass C{id:Long key;}\n";
	let expected = "package p\n\nclass C\n{\n    id: Long key;\n}\n";
	let got = klassfmt::format_with_config(
		input,
		Config {
			use_tabs: false,
			..Config::default()
		},
	)
	.expect("format");
	assert_eq!(got, expected);
}
