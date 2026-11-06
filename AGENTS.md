# MonoRelease – Agent Guide

MonoRelease is a language-agnostic release manager for monorepos, written in Rust.
It is currently in active development and **not yet ready for production use**.

---

## Project Structure

```text
src/
  lib.rs        – Crate root; defines the central MonoRelease struct
  main.rs       – Binary entry point
  cli.rs        – CLI argument parsing and command dispatch (clap)
  config.rs     – Config file loading/writing and directory path constants
  init.rs       – `monorelease init` logic (creates the folder structure)
  changes.rs    – Change file creation (`releases/next/<name>.md`)
  waves.rs      – Wave creation (promotes changes from next/ to waves/<n>/)
  logging.rs    – Tracing/logging setup
  testing.rs    – Shared test helpers (`my_test!` macro, `default_monorelease_env`)

nix/            – Nix flake outputs (dev shell, apps, shared tooling config)
website/        – Docusaurus documentation site (pnpm-based)
```

### Key Concepts

- **Change file** – A Markdown file in `releases/next/` with YAML frontmatter
  listing which projects are affected and how their version should be bumped
  (e.g. `major`, `minor`, `patch`).
- **Wave** – A numbered snapshot of changes. Running `monorelease waves create`
  moves all files from `releases/next/` into `releases/waves/<n>/`, incrementing
  the wave number automatically.
- **Project** – A sub-package within the monorepo tracked by MonoRelease
  (config lives in `releases/projects/`).

### Directory Layout (at runtime)

```
releases/
  next/         – Pending change files
  waves/        – Numbered wave snapshots (waves/1/, waves/2/, …)
  projects/     – Per-project version state
```

The constants for these paths are defined in [`config.rs`](src/config.rs):
`NEXT_DIR_PATH`, `WAVES_DIR_PATH`, `PROJECTS_DIR_PATH`.

---

## Development Environment

The project uses **Nix** to manage all dependencies, including the Rust
toolchain (pinned in [`rust-toolchain.toml`](rust-toolchain.toml) to Rust
**1.99.1**, edition 2024).

Enter the dev shell with:

```bash
nix develop
```

This provides `cargo`, `rustfmt`, `clippy`, and all other required tools.
Do not rely on a system-installed Rust toolchain; always work inside the Nix
dev shell to ensure a reproducible environment.

---

## Running Tests

```bash
cargo test
```

Tests use the `tempfile` crate to create isolated temporary directories.
The `my_test!` macro (defined in `testing.rs`) wraps each test with
`tracing::instrument` and initialises test logging.

`clippy.toml` relaxes a handful of lints inside test code
(`unwrap`, `expect`, `panic`, `indexing_slicing`) so tests can be written
concisely without fighting the stricter production rules.

---

## Code Quality – Clippy

The project enforces a **very strict** Clippy configuration (see
[`Cargo.toml`](Cargo.toml) `[lints.clippy]`):

- `clippy::pedantic` – **deny**
- `clippy::nursery` – **deny**
- Panicking operations (`unwrap`, `expect`, `panic`, `todo`, `unreachable`,
  `unimplemented`, `indexing_slicing`, `arithmetic_side_effects`) – **deny**
- Unsafe conversions (`as_conversions`) – **deny**

**Always run Clippy before considering a change ready for review:**

```bash
cargo clippy -- -D warnings
```

Clippy runs as a pre-commit hook (via `git-hooks.nix`) with
`denyWarnings = true`, and it also runs in CI. **A PR cannot be merged into
`main` if Clippy reports any warnings or errors**, even if the code compiles
and all tests pass.

---

## Other Pre-commit Checks

The following checks run automatically on commit (and in CI):

| Tool             | Purpose                        |
|------------------|-------------------------------|
| `rustfmt`        | Rust formatting                |
| `clippy`         | Rust linting                   |
| `cargo-sort`     | Keeps `Cargo.toml` deps sorted |
| `nixfmt`         | Nix formatting                 |
| `statix`         | Nix linting                    |
| `deadnix`        | Nix dead-code detection        |
| `markdownlint`   | Markdown style                 |
| `yamllint`       | YAML linting                   |
| `trufflehog`     | Secret scanning                |

---

## CI

CI runs on every push and pull request targeting `main` via
[`.github/workflows/ci.yaml`](.github/workflows/ci.yaml). It:

1. Installs Nix (Determinate Systems action).
2. Enters the Nix dev shell.
3. Runs `nix run .#ci`, which:
   - Fetches Cargo dependencies (`cargo fetch`)
   - Runs all pre-commit hooks against the entire codebase
   - Runs `cargo test`

You can replicate CI locally with:

```bash
nix run .#ci
```

> [!WARNING]
> `nix run .#ci` is slow on first run (it builds the CI derivation and runs all
> pre-commit hooks from scratch). Use it to do a final check before opening a
> PR, not as part of your normal inner loop.

For day-to-day work, prefer `cargo test` and `cargo clippy` individually.
