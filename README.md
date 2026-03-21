# dolog

`dolog` is a Rust CLI for capturing SQLite row changes with managed triggers.

It helps you install or refresh triggers on your tables, see what changes are waiting, and export those captured rows as JSON Lines.

## Install

Install from a local clone:

```bash
cargo install --path crates/dolog
dolog --help
```

Install directly from GitHub:

```bash
cargo install --git https://github.com/pyoner/dolog.git dolog
dolog --help
```

Run without installing:

```bash
cargo run -p dolog -- --help
```

## Quick Start

Create a sample database from `seed.sql`:

```bash
sqlite3 db.sqlite < seed.sql
```

Apply managed triggers for the `users` table:

```bash
dolog trigger generate db.sqlite --table users --apply
```

Check trigger coverage:

```bash
dolog trigger status db.sqlite
```

Inspect pending captured rows:

```bash
dolog log status db.sqlite
```

Preview export output without deleting anything:

```bash
dolog log export db.sqlite --dry-run
```

Export captured rows to JSONL:

```bash
dolog log export db.sqlite changes.jsonl
```

## Common Commands

Generate trigger SQL without applying it:

```bash
dolog trigger generate db.sqlite --table users
```

Generate triggers for multiple tables:

```bash
dolog trigger generate db.sqlite --table users --table posts
```

Generate triggers for all user tables:

```bash
dolog trigger generate db.sqlite --all-tables
```

Remove managed triggers:

```bash
dolog trigger generate db.sqlite --drop --table users --apply
```

Show top-level and trigger help:

```bash
dolog --help
dolog trigger --help
```

## Important Notes

- `log export` appends exported rows to the output file and then deletes those rows from the database.
- Use `--dry-run` when you want a safe preview that does not remove anything.
- `trigger status` is read-only.
- By default, `dolog` captures `insert`, `update`, and `delete` operations.
- The default log table is `_dolog_changes`.

## More Documentation

For detailed command behavior and advanced workflows, see:

- `dolog-cli/SKILL.md`
- `dolog-cli/references/trigger-generate.md`
- `dolog-cli/references/trigger-status.md`
- `dolog-cli/references/log-status.md`
- `dolog-cli/references/log-export.md`

## Development

Run the test suite:

```bash
cargo test
```
