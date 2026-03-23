# `dolog trigger generate`

Generate managed trigger SQL for one or more tables.

## Use It For

- printing trigger SQL to stdout
- writing trigger SQL to a file
- applying trigger SQL directly to a live SQLite database
- generating drop SQL for managed triggers
- generating from a live database, migration directory, or schema SQL file

## Important Rules

- Use repeated `--table` to target selected tables.
- Use `--all-tables` to target all non-SQLite user tables except the log table.
- Use repeated `--operation insert|update|delete` to limit which trigger types are generated.
- If `--operation` is omitted, `dolog` generates `insert`, `update`, and `delete` triggers.
- `--apply` is only supported when the schema source is a real SQLite database file.
- `--drop` switches generation into trigger-removal mode.

## Common Workflows

### Generate SQL to stdout

```bash
dolog trigger generate db.sqlite --table users
```

### Write SQL to a file

```bash
dolog trigger generate db.sqlite 001_users_triggers.sql --table users
```

### Apply triggers directly to a live database

```bash
dolog trigger generate db.sqlite --table users --apply
```

Use this after schema changes when the goal is to refresh installed managed triggers.

When `--apply` is used, `dolog` only changes triggers that are missing or no longer match the current table definition.

### Limit generation to selected operations

```bash
dolog trigger generate db.sqlite --table users --operation insert
dolog trigger generate db.sqlite --table users --operation insert --operation update
```

### Generate for multiple tables

```bash
dolog trigger generate db.sqlite --table users --table posts
```

### Generate for all user tables

```bash
dolog trigger generate db.sqlite --all-tables
```

### Generate from a migrations directory

```bash
dolog trigger generate migrations --table users
dolog trigger generate migrations 001_users_triggers.sql --all-tables
```

`dolog` loads `*.sql` files in lexicographic order into an in-memory SQLite database before generating trigger SQL.

### Generate from a single schema script

```bash
dolog trigger generate schema.sql --table users
dolog trigger generate schema.sql 001_users_triggers.sql --all-tables
```

### Generate drop SQL

```bash
dolog trigger generate db.sqlite --drop --table users
```

### Apply trigger removal directly

```bash
dolog trigger generate db.sqlite --drop --table users --apply
```

## Safety Notes

- Prefer stdout or a SQL file when the user wants reviewable changes.
- Prefer `--apply` when the user clearly wants the database updated in place.
- Do not use `--apply` with a migration directory or `.sql` schema file.
- Avoid targeting the log table itself as a tracked table.
