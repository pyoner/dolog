# dolog

`dolog` is a Rust CLI for SQLite change capture.

It helps you attach SQLite triggers to your tables, collect inserts/updates/deletes into a change log table, inspect what is waiting to be exported, and write those captured rows out as JSON Lines.

This project is early-stage, but the current CLI already covers the core trigger lifecycle:
- generate trigger SQL for one or more tables
- apply generated trigger SQL directly to a database
- remove managed triggers with generated drop SQL
- check trigger coverage for selected tables
- inspect pending captured rows
- export captured rows from SQLite to a JSONL file

## Who It Is For

`dolog` is aimed at SQLite app developers who want a simple, reviewable way to capture table changes without building a full sync pipeline first.

Good fit if you want to:
- audit table changes locally
- feed SQLite changes into files or downstream systems later
- keep trigger management explicit instead of hand-writing trigger SQL

## Quick Start

Prerequisites:
- Rust toolchain
- Cargo
- a SQLite database file to manage

From the repository root, create a sample database from `seed.sql`:

```bash
sqlite3 db.sqlite < seed.sql
```

Generate trigger SQL for the `users` table:

```bash
cargo run -p dolog -- trigger generate db.sqlite --table users
```

Apply those triggers directly to the database:

```bash
cargo run -p dolog -- trigger generate db.sqlite --table users --apply
```

Check trigger coverage:

```bash
cargo run -p dolog -- trigger status db.sqlite
```

Inspect pending captured rows:

```bash
cargo run -p dolog -- log status db.sqlite
```

Preview export output without deleting anything:

```bash
cargo run -p dolog -- log export db.sqlite --dry-run
```

Export captured rows to JSONL:

```bash
cargo run -p dolog -- log export db.sqlite changes.jsonl
```

## Install And Run

Show top-level help:

```bash
cargo run -p dolog -- --help
```

Show help for trigger commands:

```bash
cargo run -p dolog -- trigger --help
```

Build the binary:

```bash
cargo build -p dolog
./target/debug/dolog --help
```

## Common Workflows

### 1. Add or refresh triggers after a schema change

Generate SQL to stdout:

```bash
cargo run -p dolog -- trigger generate db.sqlite --table users
```

Write SQL to a migration file instead:

```bash
cargo run -p dolog -- trigger generate db.sqlite 001_users_triggers.sql --table users
```

Apply the generated SQL directly:

```bash
cargo run -p dolog -- trigger generate db.sqlite --table users --apply
```

Generate triggers for multiple tables:

```bash
cargo run -p dolog -- trigger generate db.sqlite --table users --table posts
```

Generate triggers for all user tables:

```bash
cargo run -p dolog -- trigger generate db.sqlite --all-tables
```

### 2. Remove managed triggers

Generate drop SQL:

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

### 3. Check what is configured

Show trigger coverage for all user tables:

```bash
cargo run -p dolog -- trigger status db.sqlite
```

Check a specific table:

```bash
cargo run -p dolog -- trigger status db.sqlite --table users
```

Example output:

```text
Trigger status for db.sqlite

TABLE  INSERT  UPDATE  DELETE
users  yes     yes     yes
posts  yes     no      yes
```

### 4. Inspect and export captured rows

See pending rows grouped by table and operation:

```bash
cargo run -p dolog -- log status db.sqlite
```

Example output:

```text
Pending log rows for db.sqlite

TABLE  OPERATION  COUNT
users  INSERT         1
users  UPDATE         1

TOTAL                2
```

Preview the next batch without writing or deleting rows:

```bash
cargo run -p dolog -- log export db.sqlite --dry-run
```

Export the next batch to a file:

```bash
cargo run -p dolog -- log export db.sqlite changes.jsonl
```

Export only a limited batch:

```bash
cargo run -p dolog -- log export db.sqlite changes.jsonl --limit 100
```

Example JSONL record:

```json
{"id":1,"table_name":"users","operation":"INSERT","old_values":null,"new_values":{"id":1,"email":"ada@example.com"},"changed_at":"2026-03-17 12:00:00"}
```

Important: normal `log export` appends rows to the output file and then deletes those exported rows from the database. Use `--dry-run` when you want a safe preview.

## Command Guide

### `trigger generate`

This is the main trigger-management command.

By default it prints generated SQL to stdout:

```bash
cargo run -p dolog -- trigger generate db.sqlite --table users
```

You can limit generation to specific operations:

```bash
cargo run -p dolog -- trigger generate db.sqlite --table users --operation insert
cargo run -p dolog -- trigger generate db.sqlite --table users --operation insert --operation update
```

If `--operation` is omitted, `dolog` uses all three operations: `insert`, `update`, and `delete`.

### `trigger status`

Shows whether `dolog`-managed `INSERT`, `UPDATE`, and `DELETE` triggers exist for each selected table.

When `--table` is omitted, status is shown for all user tables except the dolog log table.

### `log status`

Shows pending rows in the log table, grouped by source table and operation.

This command is read-only.

### `log export`

Reads rows from the log table and outputs them as JSON Lines.

- normal mode writes to a file and removes exported rows
- `--dry-run` writes JSONL to stdout and does not remove rows
- `--query` prints export SQL as JSON instead of reading from a database

Print platform-agnostic export queries:

```bash
cargo run -p dolog -- log export --query
cargo run -p dolog -- log export --query --limit 100
```

With `--query`, the output includes `select.sql` and `delete.sql`. With `--limit`, the limit is inlined into `select.sql`; otherwise the query keeps the `:limit` placeholder.

## Schema Sources For Trigger Generation

`trigger generate <schema-source>` accepts three kinds of input:

### SQLite database file

Use a real database when you want to inspect current tables or apply SQL directly:

```bash
cargo run -p dolog -- trigger generate db.sqlite --table users
cargo run -p dolog -- trigger generate db.sqlite --table users --apply
```

### Directory of migration files

Use a directory when you want to generate trigger SQL from ordered `*.sql` migrations without opening a real database:

```bash
cargo run -p dolog -- trigger generate migrations --table users
cargo run -p dolog -- trigger generate migrations 001_users_triggers.sql --all-tables
```

`dolog` loads `*.sql` files in lexicographic order into an in-memory SQLite database before generating trigger SQL.

### Single schema script

Use a single `.sql` file when your schema lives in one script:

```bash
cargo run -p dolog -- trigger generate schema.sql --table users
cargo run -p dolog -- trigger generate schema.sql 001_users_triggers.sql --all-tables
```

`dolog` loads that file into an in-memory SQLite database before generating trigger SQL.

## How It Works

For each target table, `dolog` can generate up to three SQLite triggers:
- `AFTER INSERT`
- `AFTER UPDATE`
- `AFTER DELETE`

Those triggers write rows into `_dolog_changes` with:
- `table_name`
- `operation`
- `old_values`
- `new_values`
- `changed_at`

`old_values` and `new_values` are stored as JSON built from the table columns visible at trigger generation time.

In practice, the flow looks like this:
- your app changes a row in a tracked table
- the trigger writes a change record into `_dolog_changes`
- `dolog log status` shows what is waiting
- `dolog log export` writes those rows to JSONL for downstream processing

Current scope:
- Rust workspace managed with Cargo
- one CLI crate: `dolog`
- SQLite trigger generation for `INSERT`, `UPDATE`, and `DELETE`
- change records written into a `_dolog_changes` table

Planned future scope:
- writing captured changes to log files
- exporting changes to third-party APIs

## Example Database

This repo includes `seed.sql`, which creates a small demo schema with `users`, `posts`, and `audit_notes` tables.

Create the example database:

```bash
sqlite3 db.sqlite < seed.sql
```

## Development

Run the full test suite:

```bash
cargo test
```

The test suite includes:
- library-level integration tests against real SQLite database files
- CLI-level integration tests that execute the `dolog` binary

## Reference Notes

- `trigger generate` is the recommended command after schema changes because it regenerates trigger SQL from the current table columns.
- `--table` can be repeated to target multiple tables in one command.
- `--all-tables` targets all non-SQLite tables except the dolog log table.
- `--drop` switches `trigger generate` into trigger-removal mode.
- `trigger generate <db> [sql-file]` writes SQL to stdout by default, or to a file if a SQL path is supplied.
- `trigger generate <schema-source> [sql-file]` accepts a SQLite database file, a directory of `*.sql` migrations, or a single `.sql` schema file.
- `trigger generate <db> --apply` executes the generated SQL directly against the database.
- `trigger generate <schema-source> --apply` is only supported when `<schema-source>` is a real SQLite database file.
- `trigger generate` does not allow a SQL file path and `--apply` together.
- `trigger status` defaults to all user tables when no table selector is provided.
- the log table defaults to `_dolog_changes`
- the trigger prefix defaults to `dolog`
- `log export <db> <output-file>` appends the next 100 exported rows to the output file by default and removes those rows from `_dolog_changes`
- `log export <db> --dry-run` writes the next 100 export rows to stdout by default without deleting them
- `log export <db> --query` prints a JSON payload with platform-agnostic export SQL

## Status

This repository is in early development.
