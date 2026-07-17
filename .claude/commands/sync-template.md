# Rust Template Sync

This template is the source of truth for Rust/Cargo project configurations.

## Managed Tools

### Mise Tools

Read tool versions from `.mise/config.toml` in this template. mise is the single
source of truth for the Rust toolchain and the cargo helper tools — there is no
`rust-toolchain.toml`.

- `rust` (channel + components: `clippy`, `rustfmt`, `llvm-tools-preview`)
- `just`, `node`, `pre-commit`
- `aqua:nextest-rs/nextest/cargo-nextest`
- `aqua:EmbarkStudios/cargo-deny`
- `aqua:taiki-e/cargo-llvm-cov`

### Cargo (Cargo.toml)

- `edition`
- `[profile.release]` (thin LTO + strip)
- `[lints.rust]` / `[lints.clippy]` policy

### GitHub Workflows

- Auto-fix commit message format (`fmt-fix`, `clippy-fix`, `pre-commit-fix`)
- CI/CD patterns (merge-group gate, push build)

## Rust Version Policy

- **Own projects**: use the `stable` channel unless a project pins a specific
  version for a reason.
- **Components**: always include `clippy,rustfmt,llvm-tools-preview` (llvm-tools is
  required by `cargo-llvm-cov`).
- **Never add `rust-toolchain.toml`** — keep mise as the single source of truth.

## Projects

Read the list of projects from `.llm/projects.yaml`. This file is gitignored so each
user can configure their own projects.

Example `.llm/projects.yaml`:

```yaml
# Your Rust projects synced from this template
own:
    - ~/projects/my-rust-project-1
    - ~/projects/my-rust-project-2

# Forks - sync carefully, preserve upstream-specific config
forks:
    - ~/projects/some-fork
```

## Workflow

### Step 1: Update This Template

Check whether this template's pinned versions are the latest:

```bash
# mise can resolve latest versions directly
mise ls-remote rust | tail -1
mise ls-remote just | tail -1
mise ls-remote "aqua:nextest-rs/nextest/cargo-nextest" | tail -1
mise ls-remote "aqua:EmbarkStudios/cargo-deny" | tail -1
mise ls-remote "aqua:taiki-e/cargo-llvm-cov" | tail -1
```

Compare with `.mise/config.toml`. If outdated, update them first.

### Step 2: Pull Improvements from Projects

Read `.llm/projects.yaml` and scan each project's `.mise/config.toml`, `justfile`,
`.just/*.just`, and `.github/workflows/*`. If any project has a newer version or a
better CI pattern (new auto-fix job, useful recipe), verify it is intentional, update
this template, then push to the others.

### Step 3: Push Template Versions to Projects

For each project in `.llm/projects.yaml`, compare against this template and create
tasks for any mismatches. Use `/markdown-tasks:add-one-task` to add tasks to each
project's `.llm/todo.md`.

### Task Templates

**Mise tool update:**

```
Update just <current> → <target>
  Edit .mise/config.toml
  Change: just = "<current>"
  To: just = "<target>"
```

**Adopt modular justfile includes:**

```
Adopt .just/*.just includes
  Replace flat justfile with imports of console.just, cargo.just, git.just, git-test.just
```

## Report Format

After syncing, report:

### This Template Status

- Current versions in this template
- Any updates made

### Improvements Pulled In

- List any newer versions or patterns found in siblings

### Tasks Distributed

- Number of siblings that received tasks
- Total tasks created
