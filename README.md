# klassfmt

A standalone formatter for the [Klass](https://github.com/motlin/klass) DSL,
plus its embedded use in Markdown documentation.

`klassfmt` parses `.klass` source with a tree-sitter grammar, lowers it to a
Wadler-style `Doc` IR, and prints it in the canonical Klass style. It also
formats the ` ```klass ` fenced code blocks inside Markdown files, leaving all
prose and every other fence untouched.

## Usage

```sh
# Format a file to stdout
klassfmt path/to/model.klass

# Format stdin (declare the filename so Markdown vs Klass is detected)
klassfmt --stdin-filepath model.klass < model.klass

# Rewrite files in place
klassfmt --write src/**/*.klass

# Exit non-zero if any file is not already formatted (CI gate)
klassfmt --check src/**/*.klass

# List only the files that would change
klassfmt --list-different src/**/*.klass
```

### Markdown

Files ending in `.md` / `.markdown` are processed as Markdown: only their
` ```klass ` code blocks are reformatted, and everything else is preserved
byte-for-byte. A block whose contents are not valid Klass (documentation
fragments) is left exactly as written.

```sh
klassfmt --write docs/**/*.md          # auto-detected from the extension
klassfmt --markdown --check CHANGES.md # force Markdown mode for any name
```

The same modes (`--check`, `--write`, `--list-different`, `--stdin-filepath`)
apply to both `.klass` and Markdown inputs, and a single invocation may mix them.

### Options

| Flag | Default | Meaning |
| --- | --- | --- |
| `--print-width <n>` | `120` | Column width the printer wraps at. |
| `--use-tabs [true\|false]` | `true` | Indent with a tab per level (or `--use-tabs false` for spaces). |
| `--tab-width <n>` | `4` | Columns per indentation level (tab display width for wrap math). |
| `--markdown` | off | Force Markdown mode regardless of file extension. |

Defaults match the Klass repo's `.prettierrc.json5`.

## Pre-commit hook

`klassfmt` can be wired into [pre-commit](https://pre-commit.com/) as a `local`
hook so that both `.klass` files and the ` ```klass ` fences inside Markdown are
formatted (or checked) on every commit. Build the binary first:

```sh
cargo install --path .   # installs `klassfmt` onto your PATH
```

Then add these hooks to your project's `.pre-commit-config.yaml`. The first
formats `.klass` files; the second formats `klass` fences in Markdown. Use the
`--write` variant to auto-format, or the `--check` variant (shown commented) to
fail the commit without modifying files.

```yaml
- repo: local
  hooks:
    - id: klassfmt
      name: klassfmt (format .klass files)
      entry: klassfmt --write
      language: system
      files: '\.klass$'
      # For a check-only gate instead of auto-formatting, use:
      # entry: klassfmt --check
      # pass_filenames: true

    - id: klassfmt-markdown
      name: klassfmt (format ```klass fences in Markdown)
      entry: klassfmt --markdown --write
      language: system
      files: '\.(md|markdown)$'
```

Install the hooks with `pre-commit install`. To run them across the whole
repository once (e.g. right after adding the config):

```sh
pre-commit run klassfmt --all-files
pre-commit run klassfmt-markdown --all-files
```

> This snippet is provided for you to add to a repository yourself; klassfmt does
> not modify any other repo.

## Development

```sh
cargo test                 # unit, corpus, CLI, golden, and markdown tests
cargo test --test corpus_churn churn -- --ignored --nocapture   # churn report
```

The 117 vendored `.klass` files in `tests/corpus/` are the canonical style
reference. See the notes in `src/printer.rs` for the deliberate canonicalization
choices (no colon alignment; stacked modifiers; tab indentation) made where the
hand-written corpus is internally inconsistent.
