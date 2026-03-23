# `dolog log export`

Export pending rows from the dolog log table as JSON Lines.

## Modes

- Normal mode writes JSONL to a file and deletes the exported rows from the database.
- `--dry-run` writes JSONL to stdout and does not delete rows.
- `--query` prints a JSON payload with portable `select.sql` and `delete.sql` instead of reading from a database.

## Important Rules

- In normal export mode, the output path is required.
- In `--dry-run` mode, no output file is used.
- In `--query` mode, no database path or output file is required.
- `--limit` defaults to `100` rows for export and dry-run behavior.
- Use `--log-table` to export from a custom log table name.

## Common Workflows

### Preview export output safely

```bash
dolog log export db.sqlite --dry-run
```

### Export to JSONL

```bash
dolog log export db.sqlite changes.jsonl
```

### Export a limited batch

```bash
dolog log export db.sqlite changes.jsonl --limit 100
```

### Export from a custom log table

```bash
dolog log export db.sqlite changes.jsonl --log-table custom_changes
```

### Print portable export SQL

```bash
dolog log export --query
dolog log export --query --limit 100
```

## Safety Notes

- Prefer `--dry-run` when the user wants a safe preview.
- Warn that normal `log export` deletes exported rows after writing them.
- Use `--query` when the user wants integration-friendly SQL instead of direct export behavior.
