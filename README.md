# dolog

`dolog` is a Rust CLI for managing SQLite triggers that capture table changes for downstream logging.

The current milestone focuses on:
- generating trigger SQL for one or more tables
- applying generated trigger SQL directly to a database when needed
- removing triggers through generated drop SQL
- showing trigger coverage status
- exporting captured change rows from SQLite to a file

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

This repo includes a simple example seed file at `seed.sql`.

Create a local example database:

```bash
sqlite3 db.sqlite < seed.sql
```

You can then run the CLI against that file:

```bash
cargo run -p dolog -- trigger generate db.sqlite --table users
cargo run -p dolog -- trigger generate db.sqlite --table users --apply
cargo run -p dolog -- trigger status db.sqlite
cargo run -p dolog -- log status db.sqlite
cargo run -p dolog -- trigger generate db.sqlite --drop --table users
cargo run -p dolog -- log export db.sqlite --dry-run
cargo run -p dolog -- log export db.sqlite changes.jsonl
```

## Trigger Workflow

`dolog trigger generate` is the primary trigger-management command.

By default it generates SQL to stdout:

```bash
cargo run -p dolog -- trigger generate db.sqlite --table users
```

Generate SQL from ordered migration files without opening a real database:

```bash
cargo run -p dolog -- trigger generate --from-migration migrations --table users
```

Write SQL to a file for migrations:

```bash
cargo run -p dolog -- trigger generate db.sqlite 001_users_triggers.sql --table users
cargo run -p dolog -- trigger generate --from-migration migrations 001_users_triggers.sql --all-tables
```

Apply the generated SQL directly:

```bash
cargo run -p dolog -- trigger generate db.sqlite --table users --apply
```

Generate SQL only for specific operations:

```bash
cargo run -p dolog -- trigger generate db.sqlite --table users --operation insert
cargo run -p dolog -- trigger generate db.sqlite --table users --operation insert --operation update
```

Generate SQL for multiple tables:

```bash
cargo run -p dolog -- trigger generate db.sqlite --table users --table posts
```

Generate SQL for all user tables:

```bash
cargo run -p dolog -- trigger generate db.sqlite --all-tables
```

Drop trigger SQL instead of create/refresh SQL:

```bash
cargo run -p dolog -- trigger generate db.sqlite --drop --table users
```

Write drop SQL to a file:

```bash
cargo run -p dolog -- trigger generate db.sqlite 003_drop_users_triggers.sql --drop --table users
```

Apply trigger removal directly:

```bash
cargo run -p dolog -- trigger generate db.sqlite --drop --table users --apply
```

## Trigger Status

Show trigger status:

```bash
cargo run -p dolog -- trigger status db.sqlite
cargo run -p dolog -- trigger status db.sqlite --table users
```

Example status output:

```text
Trigger status for db.sqlite

TABLE  INSERT  UPDATE  DELETE
users  yes     yes     yes
posts  yes     no      yes
```

## Log Export

Export captured logs to JSON Lines:

```bash
cargo run -p dolog -- log status db.sqlite
cargo run -p dolog -- log export db.sqlite changes.jsonl
```

Export only the next batch:

```bash
cargo run -p dolog -- log export db.sqlite changes.jsonl --limit 100
```

Dry run without writing or deleting:

```bash
cargo run -p dolog -- log export db.sqlite --dry-run
```

Print platform-agnostic export queries as JSON:

```bash
cargo run -p dolog -- log export --query
cargo run -p dolog -- log export --query --limit 100
```

`dolog log export` reads rows from `_dolog_changes`, appends them to a JSONL file, and then deletes the exported rows from the database.

`dolog log export --dry-run` writes those same JSONL rows to stdout without removing them from the database.

`dolog log export --query` prints a JSON payload with `select.sql` and `delete.sql` statements for platform-side export. With `--limit`, the limit is inlined into `select.sql`; otherwise the query keeps the `:limit` placeholder.

`dolog log status` shows pending change rows grouped by table and operation before export.

Example log status output:

```text
Pending log rows for db.sqlite

TABLE  OPERATION  COUNT
users  INSERT         1
users  UPDATE         1

TOTAL                2
```

Example JSONL record:

```json
{"id":1,"table_name":"users","operation":"INSERT","old_values":null,"new_values":{"id":1,"email":"ada@example.com"},"changed_at":"2026-03-17 12:00:00"}
```

## How It Works

For a target table, `dolog` can generate up to three SQLite triggers:
- `AFTER INSERT`
- `AFTER UPDATE`
- `AFTER DELETE`

These triggers write rows into `_dolog_changes` with:
- `table_name`
- `operation`
- `old_values`
- `new_values`
- `changed_at`

`old_values` and `new_values` are stored as JSON generated from the table columns visible at trigger generation time.

## Test

Run the full test suite:

```bash
cargo test
```

The test suite includes:
- library-level integration tests against real SQLite database files
- CLI-level integration tests that execute the `dolog` binary

## Notes

- `trigger generate` is the recommended command after schema changes because it regenerates trigger SQL from the current table columns.
- If `--operation` is omitted, `dolog` uses all three operations.
- `--table` can be repeated to target multiple tables in one command.
- `--all-tables` targets all non-SQLite tables except the dolog log table.
- `--drop` switches `trigger generate` into trigger-removal mode.
- `trigger status` defaults to all user tables when no table selector is provided.
- `trigger generate <db> [sql-file]` writes SQL to stdout by default, or to a file if a SQL path is supplied.
- `trigger generate --from-migration <dir> [sql-file]` loads `*.sql` files from the directory in lexicographic order into an in-memory SQLite database before generating trigger SQL.
- `trigger generate <db> --apply` executes the generated SQL directly against the database.
- `trigger generate --from-migration <dir> --apply` is not supported because there is no target database to modify.
- `trigger generate` does not allow a SQL file path and `--apply` together.
- The log table defaults to `_dolog_changes`.
- The trigger prefix defaults to `dolog`.
- `log export <db> <output-file>` appends exported rows to the output file and removes those rows from `_dolog_changes`.
- `log export <db> --dry-run` writes the export rows to stdout in JSONL format without deleting them.
- `log export <db> --query` prints a JSON payload with platform-agnostic export SQL.
