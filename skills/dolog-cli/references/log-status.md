# `dolog log status`

Show pending captured rows grouped by table and operation.

## Behavior

- This command is read-only.
- It reads from the dolog log table, which defaults to `_dolog_changes`.
- Use `--log-table` to inspect a custom log table name.

## Examples

Inspect pending rows:

```bash
dolog log status db.sqlite
```

Inspect a custom log table:

```bash
dolog log status db.sqlite --log-table custom_changes
```
