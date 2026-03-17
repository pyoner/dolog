# dolog

`dolog` is a Rust CLI for managing SQLite triggers that capture table changes for downstream logging.

The current milestone focuses on trigger lifecycle management:
- create triggers for a table
- update triggers after schema changes
- delete triggers
- preview generated SQL
- list managed triggers

## Status

This repository is in early development.

Current scope:
- Rust workspace managed with Cargo
- one CLI crate: `dolog`
- SQLite trigger generation for `INSERT`, `UPDATE`, and `DELETE`
- change records written into a `_dolog_changes` table

Future scope:
- writing captured changes to log files
- exporting changes to third-party APIs

## Requirements

- Rust toolchain
- Cargo
- SQLite database file to manage

## Run

From the repository root:

```bash
cargo run -p dolog -- --help
cargo run -p dolog -- trigger --help
```

Build the binary:

```bash
cargo build -p dolog
./target/debug/dolog --help
```

## Example Database

This repo includes a simple example seed file at [`seed.sql`](/home/pyoner/repo/dolog/seed.sql).

Create a local example database:

```bash
sqlite3 dev.sqlite < seed.sql
```

You can then run the CLI against that file:

```bash
cargo run -p dolog -- trigger create --db /home/pyoner/repo/dolog/dev.sqlite --table users
cargo run -p dolog -- trigger list --db /home/pyoner/repo/dolog/dev.sqlite
cargo run -p dolog -- trigger create --db /home/pyoner/repo/dolog/dev.sqlite --table users --dry-run
```

## Commands

Create triggers for a table:

```bash
cargo run -p dolog -- trigger create --db /path/to/app.sqlite --table users
```

Update triggers after a schema change:

```bash
cargo run -p dolog -- trigger update --db /path/to/app.sqlite --table users
```

Delete triggers for a table:

```bash
cargo run -p dolog -- trigger delete --db /path/to/app.sqlite --table users
```

Preview SQL without modifying the database:

```bash
cargo run -p dolog -- trigger create --db /path/to/app.sqlite --table users --dry-run
cargo run -p dolog -- trigger update --db /path/to/app.sqlite --table users --dry-run
cargo run -p dolog -- trigger delete --db /path/to/app.sqlite --table users --dry-run
```

Preview explicit subcommands:

```bash
cargo run -p dolog -- trigger preview create --db /path/to/app.sqlite --table users
cargo run -p dolog -- trigger preview update --db /path/to/app.sqlite --table users
cargo run -p dolog -- trigger preview delete --db /path/to/app.sqlite --table users
```

List managed triggers:

```bash
cargo run -p dolog -- trigger list --db /path/to/app.sqlite
cargo run -p dolog -- trigger list --db /path/to/app.sqlite --table users
```

## How It Works

For a target table, `dolog` generates three SQLite triggers:
- `AFTER INSERT`
- `AFTER UPDATE`
- `AFTER DELETE`

These triggers write rows into `_dolog_changes` with:
- `table_name`
- `operation`
- `old_values`
- `new_values`
- `changed_at`

`old_values` and `new_values` are stored as JSON generated from the table columns visible at trigger creation time.

## Test

Run the full test suite:

```bash
cargo test
```

The test suite includes:
- library-level integration tests against real SQLite database files
- CLI-level integration tests that execute the `dolog` binary

## Notes

- `update` should be run after schema changes so trigger JSON reflects the current table columns.
- The log table defaults to `_dolog_changes`.
- The trigger prefix defaults to `dolog`.
