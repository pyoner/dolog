# dolog

`dolog` is a Rust CLI for managing SQLite triggers that capture table changes for downstream logging.

The current milestone focuses on trigger lifecycle management:
- create triggers for one or more tables
- update triggers after schema changes
- delete triggers
- show trigger coverage status
- write planned SQL to stdout or a file before applying it

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
cargo run -p dolog -- trigger create /home/pyoner/repo/dolog/dev.sqlite --table users
cargo run -p dolog -- trigger status /home/pyoner/repo/dolog/dev.sqlite
cargo run -p dolog -- trigger create /home/pyoner/repo/dolog/dev.sqlite --table users --dry-run
```

## Commands

Create triggers for a table:

```bash
cargo run -p dolog -- trigger create /path/to/app.sqlite --table users
```

Create triggers only for specific operations:

```bash
cargo run -p dolog -- trigger create /path/to/app.sqlite --table users --operation insert
cargo run -p dolog -- trigger create /path/to/app.sqlite --table users --operation insert --operation update
```

Create triggers for multiple tables:

```bash
cargo run -p dolog -- trigger create /path/to/app.sqlite --table users --table posts
```

Create triggers for all user tables:

```bash
cargo run -p dolog -- trigger create /path/to/app.sqlite --all-tables
```

Update triggers after a schema change:

```bash
cargo run -p dolog -- trigger update /path/to/app.sqlite --table users
```

Update only specific operations after a schema change:

```bash
cargo run -p dolog -- trigger update /path/to/app.sqlite --table users --operation insert
```

Delete triggers for a table:

```bash
cargo run -p dolog -- trigger delete /path/to/app.sqlite --table users
```

Delete only specific operations:

```bash
cargo run -p dolog -- trigger delete /path/to/app.sqlite --table users --operation delete
```

Preview SQL without modifying the database:

```bash
cargo run -p dolog -- trigger create /path/to/app.sqlite --table users --dry-run
cargo run -p dolog -- trigger update /path/to/app.sqlite --table users --dry-run
cargo run -p dolog -- trigger delete /path/to/app.sqlite --table users --dry-run
```

Preview multiple tables:

```bash
cargo run -p dolog -- trigger create /path/to/app.sqlite --table users --table posts --dry-run
```

Preview all user tables:

```bash
cargo run -p dolog -- trigger create /path/to/app.sqlite --all-tables --dry-run
```

Write SQL to a file instead of applying it:

```bash
cargo run -p dolog -- trigger create /path/to/app.sqlite --table users --output migrations/001_create_users_triggers.sql
cargo run -p dolog -- trigger update /path/to/app.sqlite --table users --output migrations/002_update_users_triggers.sql
cargo run -p dolog -- trigger delete /path/to/app.sqlite --table users --output migrations/003_delete_users_triggers.sql
```

Write one combined SQL plan for multiple tables:

```bash
cargo run -p dolog -- trigger create /path/to/app.sqlite --table users --table posts --output migrations/001_create_triggers.sql
```

Write one combined SQL plan for all user tables:

```bash
cargo run -p dolog -- trigger create /path/to/app.sqlite --all-tables --output migrations/001_create_triggers.sql
```

Show trigger status:

```bash
cargo run -p dolog -- trigger status /path/to/app.sqlite
cargo run -p dolog -- trigger status /path/to/app.sqlite --table users
cargo run -p dolog -- trigger status /path/to/app.sqlite --all-tables
```

Example status output:

```text
users | insert: yes | update: yes | delete: yes
posts | insert: yes | update: no | delete: yes
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
- `--operation` can be repeated to target specific trigger types: `insert`, `update`, `delete`.
- If `--operation` is omitted, `dolog` uses all three operations.
- `update` refreshes only the selected operations. It does not remove unrelated trigger types.
- The log table defaults to `_dolog_changes`.
- The trigger prefix defaults to `dolog`.
- `--table` can be repeated to target multiple tables in one command.
- `--all-tables` targets all non-SQLite tables except the dolog log table.
- `trigger status` defaults to all user tables when no table selector is provided.
- `--dry-run` prints the SQL plan to stdout.
- `--output <FILE>` writes the SQL plan to a file instead of applying it.
