# AGENTS.md

## Project
`dolog` is a Rust workspace for SQLite change capture.

Today the repo is centered on one CLI crate, `dolog`, which generates managed SQLite triggers, applies or removes them, reports trigger coverage, reports pending captured log rows, and exports captured rows as JSON Lines.

The current design center is:
- CLI-first workflows
- explicit and reviewable SQLite behavior
- minimal abstractions
- easy future expansion toward file and API sinks

## Workspace Layout
- Workspace root: `Cargo.toml`
- Primary crate: `crates/dolog`
- CLI entrypoint: `crates/dolog/src/main.rs`
- Library entrypoint: `crates/dolog/src/lib.rs`
- CLI parsing and command wiring: `crates/dolog/src/cli.rs`
- Trigger logic: `crates/dolog/src/trigger.rs`
- Log export logic: `crates/dolog/src/log_export.rs`
- Integration tests: `crates/dolog/tests/`
- Sample assets: `seed.sql`, `migrations/`, `db.sqlite`
- Agent skill docs: `skills/`

## Repo-Specific Rules
No repo-local Cursor or Copilot instruction files were found.

Specifically, these files are absent:
- `.cursorrules`
- `.cursor/rules/`
- `.github/copilot-instructions.md`

## Canonical Commands
Run commands from the repository root unless a task requires otherwise.

### Build And Check
- Check the main crate: `cargo check -p dolog`
- Check the full workspace: `cargo check --workspace`
- Build the CLI crate: `cargo build -p dolog`
- Build all targets: `cargo build --workspace --all-targets`

### Format And Lint
- Format all Rust code: `cargo fmt --all`
- Check formatting only: `cargo fmt --all -- --check`
- Run clippy on all targets: `cargo clippy --workspace --all-targets -- -D warnings`

Notes:
- `cargo fmt --all -- --check` currently reports formatting drift.
- `cargo clippy --workspace --all-targets -- -D warnings` currently fails on `clippy::ptr_arg` in `crates/dolog/src/cli.rs` for functions that take `&PathBuf` instead of `&Path`.

### Test
- Run the main crate test suite: `cargo test -p dolog`
- Run the full workspace test suite: `cargo test --workspace`
- Run library unit tests only: `cargo test -p dolog --lib`
- Run CLI integration tests only: `cargo test -p dolog --test cli`
- Run trigger lifecycle integration tests only: `cargo test -p dolog --test trigger_lifecycle`
- List all tests: `cargo test -- --list`

### Run The CLI
- Top-level help: `cargo run -p dolog -- --help`
- Trigger help: `cargo run -p dolog -- trigger --help`
- Log help: `cargo run -p dolog -- log --help`
- Generate trigger SQL: `cargo run -p dolog -- trigger generate db.sqlite --table users`
- Apply trigger SQL: `cargo run -p dolog -- trigger generate db.sqlite --table users --apply`
- Show trigger coverage: `cargo run -p dolog -- trigger status db.sqlite`
- Show pending log rows: `cargo run -p dolog -- log status db.sqlite`
- Preview export without deletion: `cargo run -p dolog -- log export db.sqlite --dry-run`
- Export JSONL: `cargo run -p dolog -- log export db.sqlite changes.jsonl`

## Running A Single Test
Use `-- --exact` when you want only one named test.

- Exact unit test:
  `cargo test -p dolog --lib trigger::tests::quotes_identifiers -- --exact`
- Exact CLI integration test:
  `cargo test -p dolog --test cli top_level_help_describes_trigger_and_log_commands -- --exact`
- Exact trigger lifecycle integration test:
  `cargo test -p dolog --test trigger_lifecycle manages_trigger_lifecycle_for_a_table -- --exact`

Use substring filters when iterating on a feature area.

- Trigger apply behavior: `cargo test -p dolog generate_apply`
- Trigger generation behavior: `cargo test -p dolog generate_`
- Log export behavior: `cargo test -p dolog log_export`
- Help text behavior: `cargo test -p dolog help`

## Agent Skill
This repo includes a repo-local Agent Skill for the CLI:

- Main skill file: `skills/dolog-cli/SKILL.md`
- Trigger generate reference: `skills/dolog-cli/references/trigger-generate.md`
- Trigger status reference: `skills/dolog-cli/references/trigger-status.md`
- Log status reference: `skills/dolog-cli/references/log-status.md`
- Log export reference: `skills/dolog-cli/references/log-export.md`

Use the `dolog-cli` skill when the task is about operating the CLI rather than changing Rust internals.

Validate the skill structure with:

`uvx --from skills-ref agentskills validate ./skills/dolog-cli`

This is the preferred no-install validation command in this repo.

Related checks:
- Run `cargo test -p dolog --test cli` when command help text or examples change.
- Update `skills/` when CLI behavior, defaults, or safety warnings change.

## Code Style Guidelines
Infer conventions from the current code before changing them.

### Imports
- Group imports in this order: `std`, third-party crates, then `crate::...` imports.
- Prefer nested `std` imports when they reduce repetition.

### Formatting
- Follow default `rustfmt` output.
- Prefer multiline argument lists and attribute blocks once lines get long.
- Keep embedded SQL strings readable and consistently indented.

### Types And APIs
- Prefer explicit function signatures and concrete return types.
- Use `Path` for borrowed path parameters and `PathBuf` for owned path values.
- Use small focused structs and enums rather than generic maps for app logic.
- Prefer `String` for owned CLI/config text that crosses layers.

### Naming
- Types, structs, and enums: `PascalCase`
- Functions, methods, modules, and tests: `snake_case`
- Enum variants: `PascalCase`
- Constants: `SCREAMING_SNAKE_CASE`
- Test names should describe behavior, for example `generate_apply_replaces_drifted_trigger`

### Error Handling
- Use `Result<_, AppError>` for application flows.
- Prefer the shared `AppError` enum with `thiserror` for domain errors.
- Propagate errors with `?`.
- Include table names, file paths, or schema source context in user-facing errors.
- Do not use `panic!` for normal runtime failures.
- `expect(...)` is acceptable in tests and for invariants already enforced by `clap`.

### SQLite And SQL
- Keep SQLite behavior explicit and reviewable.
- Prefer helper functions for quoting identifiers and literals.
- Preserve deterministic SQL generation so tests can assert on exact output.
- Use transactions around multi-statement apply or delete operations.
- Avoid hidden schema mutations and implicit side effects.

### CLI And Help Text
- Use `clap` derive APIs for commands and arguments.
- Keep `about` short and action-oriented.
- Use `long_about` to explain behavior, defaults, and safety implications.
- Use `after_help` for concrete examples.
- Call out read-only versus destructive behavior explicitly.

When adding or changing a command or flag:
- update parser definitions and help text together
- add or update CLI integration tests in `crates/dolog/tests/cli.rs`
- update `README.md` and `skills/` if user-facing behavior changed

## Testing Conventions
- Prefer integration tests against real SQLite files over mocks.
- Use `assert_cmd` for CLI execution tests.
- Use `predicates` for stdout and stderr assertions.
- Use temporary SQLite files in the system temp directory.
- Assert user-visible output, not only internal state.
- Add focused unit tests for SQL normalization and helper functions.

## Architecture Guidance
- Keep CLI parsing separate from trigger and export logic.
- Isolate SQLite-specific logic behind small internal APIs.
- Prefer straightforward structs like `TriggerManager` over premature traits.
- Optimize for readable trigger lifecycle code before future sink abstractions.

## Working Style For Agents
- Keep changes brief and easy to review.
- Prefer practical defaults over speculative framework setup.
- Document new commands and structure as they are introduced.
- Choose the smallest reasonable tooling addition.
- If help text changes, run `cargo test -p dolog --test cli`.
- If trigger behavior changes, run `cargo test -p dolog --test trigger_lifecycle` and relevant filtered tests.
- Before finishing substantial Rust changes, prefer running format, clippy, and the crate test suite.
