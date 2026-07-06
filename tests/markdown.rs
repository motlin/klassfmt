//! A6: markdown ```klass fence formatting.

use std::fs;
use std::path::{Path, PathBuf};

use klassfmt::{format_markdown, Config};

fn fmt(src: &str) -> String {
    format_markdown(src, Config::default())
}

fn fixtures() -> Vec<PathBuf> {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/markdown");
    let mut v: Vec<PathBuf> = fs::read_dir(dir)
        .expect("read fixtures dir")
        .map(|e| e.unwrap().path())
        .filter(|p| p.extension().map(|e| e == "md").unwrap_or(false))
        .collect();
    v.sort();
    v
}

#[test]
fn klass_fence_is_formatted_to_canonical_tabs() {
    let src = "\
# Doc

```klass
class Question
{
    id        : Long key id;
    title     : String maxLength(150);
}
```
";
    let out = fmt(src);
    // Colons de-aligned, indentation switched to a tab.
    assert!(out.contains("\n\tid: Long key id;\n"), "got:\n{out}");
    assert!(out.contains("\n\ttitle: String maxLength(150);\n"), "got:\n{out}");
    // Fence and heading preserved.
    assert!(out.starts_with("# Doc\n\n```klass\n"));
    assert!(out.trim_end().ends_with("```"));
}

#[test]
fn prose_and_non_klass_fences_are_byte_identical() {
    let src = "\
# Title

Some **prose** with `inline code` and a klass word that is not a fence.

```json
{ \"a\": 1 }
```

```klass
class C
{
    id: Long key;
}
```

More prose.
";
    let out = fmt(src);

    // The json block and all prose lines survive unchanged.
    assert!(out.contains("```json\n{ \"a\": 1 }\n```"), "json fence changed:\n{out}");
    assert!(out.contains("Some **prose** with `inline code` and a klass word that is not a fence."));
    assert!(out.contains("\nMore prose.\n"));
    // Only the klass block changed: it now uses a tab.
    assert!(out.contains("\n\tid: Long key;\n"), "klass not formatted:\n{out}");
}

#[test]
fn unparseable_block_is_left_untouched() {
    // `String(30000)` is not valid Klass; the block must be preserved verbatim.
    let src = "\
```klass
class Answer
{
    body      : String(30000);
}
```
";
    assert_eq!(fmt(src), src, "invalid block should be byte-identical");
}

#[test]
fn indented_fence_keeps_its_offset() {
    // A klass fence indented under a list item keeps its indentation.
    let src = "\
- item:

  ```klass
  class C
  {
      id: Long key;
  }
  ```
";
    let out = fmt(src);
    // Content stays indented 2 spaces; the property is indented 2 + one tab.
    assert!(out.contains("\n  ```klass\n"), "opening fence offset lost:\n{out}");
    assert!(out.contains("\n  \tid: Long key;\n"), "content offset/tab wrong:\n{out}");
    assert!(out.contains("\n  ```\n"), "closing fence offset lost:\n{out}");
}

#[test]
fn info_string_with_trailing_attributes_is_recognized() {
    let src = "\
```klass title=\"example\"
class C
{
    id: Long key;
}
```
";
    let out = fmt(src);
    assert!(out.contains("\n\tid: Long key;\n"), "attributed fence not formatted:\n{out}");
    // The info string itself is preserved on the fence line.
    assert!(out.starts_with("```klass title=\"example\"\n"));
}

#[test]
fn tilde_fences_are_supported() {
    let src = "\
~~~klass
class C
{
    id: Long key;
}
~~~
";
    let out = fmt(src);
    assert!(out.contains("\n\tid: Long key;\n"), "tilde fence not formatted:\n{out}");
    assert!(out.starts_with("~~~klass\n"));
    assert!(out.trim_end().ends_with("~~~"));
}

#[test]
fn a_klass_word_that_is_not_a_fence_language_is_ignored() {
    // `klassy` is not the `klass` language; the block must be untouched.
    let src = "\
```klassy
class C { }
```
";
    assert_eq!(fmt(src), src);
}

#[test]
fn real_doc_fixtures_format_and_are_idempotent() {
    let files = fixtures();
    assert!(!files.is_empty(), "no markdown fixtures found");
    for path in files {
        let src = fs::read_to_string(&path).unwrap();
        let once = fmt(&src);
        let twice = fmt(&once);
        assert_eq!(
            once,
            twice,
            "{} is not idempotent under markdown formatting",
            path.display()
        );
    }
}

#[test]
fn real_doc_fixture_changes_only_klass_blocks() {
    // For 1_classes.md, every changed line must be inside a klass fence.
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/markdown/part1__1_classes.md");
    let src = fs::read_to_string(&path).unwrap();
    let out = fmt(&src);

    // Reconstruct the set of prose (non-fenced-content) lines from both and
    // confirm they match. We track fence state the same way the formatter does.
    assert_eq!(prose_only(&src), prose_only(&out), "prose diverged");
}

/// Returns the document with the interiors of ```klass blocks removed, so two
/// documents can be compared for prose/fence equality regardless of block body.
fn prose_only(doc: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut in_klass = false;
    for line in doc.lines() {
        let trimmed = line.trim_start();
        if !in_klass && trimmed.starts_with("```klass") {
            in_klass = true;
            out.push(line.to_string());
        } else if in_klass && trimmed.starts_with("```") {
            in_klass = false;
            out.push(line.to_string());
        } else if !in_klass {
            out.push(line.to_string());
        }
    }
    out
}
