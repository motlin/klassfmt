//! A2: core subset (package + class + data-type properties) formats to the
//! canonical style, including colon alignment.

/// Assert that `input` formats exactly to `expected`.
fn assert_formats(input: &str, expected: &str) {
    let got = klassfmt::format(input).expect("format should succeed");
    if got != expected {
        eprintln!("--- expected ---\n{expected}\n--- got ---\n{got}\n---");
    }
    assert_eq!(got, expected);
}

#[test]
fn formats_class_with_aligned_properties() {
    // Deliberately messy input: irregular spacing, no alignment.
    let input = "package com.example\n\
                 class Person systemTemporal versioned {\n\
                 id:Long id key;\n\
                 firstName :String?;\n\
                 lastName: String ?;\n\
                 }\n";

    let expected = "\
package com.example

class Person
    systemTemporal
    versioned
{
    id: Long id key;
    firstName: String?;
    lastName: String?;
}
";
    assert_formats(input, expected);
}

#[test]
fn already_canonical_class_is_a_near_noop() {
    // Canonical style is single-space-after-name (no colon alignment); see the
    // note on MemberDoc::into_doc for why alignment is not reproduced.
    let canonical = "\
package com.example

class Person
    systemTemporal
{
    id: Long id key;
    firstName: String?;
}
";
    assert_formats(canonical, canonical);
}

#[test]
fn formats_property_with_validation() {
    let input = "package p\nclass C { name : String minLength ( 1 ) maxLength(255); }\n";
    let expected = "\
package p

class C
{
    name: String minLength(1) maxLength(255);
}
";
    assert_formats(input, expected);
}
