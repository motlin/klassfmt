ci := env("CI", "")

# cargo build
[group('build')]
build:
    cargo build --all-targets

# cargo nextest run
[group('build')]
test:
    cargo nextest run {{ if ci != "" { "--profile ci" } else { "" } }}

# cargo clippy (all targets, warnings as errors)
[group('build')]
clippy:
    cargo clippy --all-targets --all-features -- -D warnings

# cargo fmt (--check in CI, fix locally)
[group('build')]
fmt:
    cargo fmt --all {{ if ci != "" { "-- --check" } else { "" } }}

# cargo deny check (advisories, licenses, bans, sources)
[group('build')]
deny:
    cargo deny check

# build + test + clippy + fmt + deny + coverage (mirrors the CI gate)
[group('build')]
check: build test clippy fmt deny coverage

# cargo llvm-cov (coverage; writes lcov.info in CI)
[group('build')]
coverage:
    cargo llvm-cov nextest {{ if ci != "" { "--lcov --output-path lcov.info" } else { "" } }}

# cargo run -- {{ARGS}}
[group('run')]
run *ARGS:
    cargo run -- {{ARGS}}
