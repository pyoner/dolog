---
name: dolog-cli
description: Use when working with the dolog Rust CLI for SQLite change capture, especially to generate or apply managed SQLite triggers, inspect trigger coverage, check pending log rows, export JSONL, or understand schema-source, log-table, and trigger-prefix options.
---

# dolog-cli

Use this skill when the task is specifically about the `dolog` CLI in this repository.

## When to Use

- The user wants to generate, apply, refresh, or remove `dolog`-managed SQLite triggers.
- The user wants to inspect trigger coverage for one or more tables.
- The user wants to inspect pending rows in the dolog log table.
- The user wants to export captured change rows to JSONL or preview an export safely.
- The user wants to generate trigger SQL from a live SQLite database, a migration directory, or a single schema SQL file.

## When Not to Use

- The task is about generic SQLite trigger authoring unrelated to `dolog`.
- The task is about changing the Rust implementation unless the user is explicitly asking for code changes in this repo.
- The task is about future sinks or third-party APIs that the current CLI does not support yet.

## Mental Model

`dolog` manages SQLite triggers that write captured row changes into a log table.

The normal flow is:

1. A tracked table receives an `INSERT`, `UPDATE`, or `DELETE`.
2. A `dolog` trigger writes a change row into `_dolog_changes` by default.
3. `dolog log status` shows what is waiting to be exported.
4. `dolog log export` previews or exports those rows as JSON Lines.

Important defaults:

- log table: `_dolog_changes`
- trigger prefix: `dolog`
- export and dry-run batch size: `100` rows by default

## Command Map

### `trigger generate`

Generate trigger SQL for selected tables.

See the [trigger generate reference](references/trigger-generate.md) for workflows, options, and examples.

### `trigger status`

Show whether `dolog`-managed `INSERT`, `UPDATE`, and `DELETE` triggers exist for each selected table.

See the [trigger status reference](references/trigger-status.md) for behavior and examples.

### `log status`

Show pending captured rows grouped by table and operation.

This command is read-only.

See the [log status reference](references/log-status.md) for examples.

### `log export`

Export pending rows as JSON Lines.


- normal mode writes to a file and deletes exported rows
- `--dry-run` writes JSONL to stdout and does not delete rows
- `--query` prints a JSON payload with portable `select.sql` and `delete.sql`

See the [log export reference](references/log-export.md) for workflows, options, and examples.

## Safety and Decision Rules

- Prefer printing SQL or writing a file when the user wants reviewable trigger changes.
- Prefer `--apply` when the user clearly wants the database updated in place.
- Prefer `--dry-run` when the user wants to inspect export output without deleting rows.
- Warn that normal `log export` deletes exported rows after writing them.
- Avoid targeting the log table itself as a tracked table.
- If the user omits `--operation`, `dolog` uses all three operations: `insert`, `update`, and `delete`.

## Useful Defaults and Options

- `--log-table <name>` changes the log table from `_dolog_changes`.
- `--trigger-prefix <prefix>` changes managed trigger names from the `dolog_*` pattern.
- Repeat `--table` to target multiple tables.
- Use `--all-tables` to target all non-SQLite user tables except the log table.
- Repeat `--operation` to limit generation to selected operations.

## Example Prompts This Skill Should Handle

- Use `dolog` to generate and apply triggers for the `users` table.
- Refresh `dolog` triggers after a schema change.
- Show pending `_dolog_changes` rows and export them to JSONL.
- Generate `dolog` trigger SQL from a migrations directory.
- Preview a `dolog` export without deleting rows.
