# `dolog trigger status`

Show whether `dolog`-managed `INSERT`, `UPDATE`, and `DELETE` triggers exist for each selected table.

## Behavior

- When `--table` is omitted, status is shown for all user tables except the log table.
- Repeat `--table` to inspect multiple specific tables.
- Use `--trigger-prefix` if managed trigger names use a prefix other than `dolog`.
- Use `--log-table` if the log table name differs from `_dolog_changes`.

## Examples

Check all user tables:

```bash
dolog trigger status db.sqlite
```

Check one table:

```bash
dolog trigger status db.sqlite --table users
```

Check multiple tables:

```bash
dolog trigger status db.sqlite --table users --table posts
```
