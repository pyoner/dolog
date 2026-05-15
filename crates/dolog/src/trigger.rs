use std::path::Path;

use rusqlite::{Connection, OpenFlags};
use sha2::{Digest, Sha256};
use thiserror::Error;

pub fn open_connection(path: &Path) -> Result<Connection, AppError> {
    Connection::open_with_flags(
        path,
        OpenFlags::SQLITE_OPEN_READ_WRITE | OpenFlags::SQLITE_OPEN_URI,
    )
    .map_err(|source| AppError::OpenDatabase {
        path: path.display().to_string(),
        source,
    })
}

pub struct TriggerManager {
    log_table: String,
    trigger_prefix: String,
}

impl TriggerManager {
    pub fn new(log_table: String, trigger_prefix: String) -> Self {
        Self {
            log_table,
            trigger_prefix,
        }
    }

    pub fn create(&self, connection: &mut Connection, table: &str) -> Result<(), AppError> {
        let plan = self.plan_create(connection, table, &Operation::all())?;
        self.apply_plan(connection, &plan)
    }

    pub fn update(&self, connection: &mut Connection, table: &str) -> Result<(), AppError> {
        self.ensure_log_table(connection)?;
        let plan = self.plan_update(connection, table, &Operation::all())?;
        self.apply_plan(connection, &plan)
    }

    pub fn delete(&self, connection: &mut Connection, table: &str) -> Result<(), AppError> {
        let plan = self.plan_delete(connection, table, &Operation::all())?;
        self.apply_plan(connection, &plan)
    }

    pub fn preview_create(
        &self,
        connection: &Connection,
        table: &str,
    ) -> Result<Vec<String>, AppError> {
        Ok(self
            .plan_create(connection, table, &Operation::all())?
            .into_statements())
    }

    pub fn preview_update(
        &self,
        connection: &Connection,
        table: &str,
    ) -> Result<Vec<String>, AppError> {
        Ok(self
            .plan_update(connection, table, &Operation::all())?
            .into_statements())
    }

    pub fn preview_delete(
        &self,
        connection: &Connection,
        table: &str,
    ) -> Result<Vec<String>, AppError> {
        Ok(self
            .plan_delete(connection, table, &Operation::all())?
            .into_statements())
    }

    pub fn plan_create(
        &self,
        connection: &Connection,
        table: &str,
        operations: &[Operation],
    ) -> Result<ExecutionPlan, AppError> {
        let target = self.describe_target(connection, table)?;
        let mut statements = vec![self.create_log_table_sql()];
        statements.extend(
            operations
                .iter()
                .copied()
                .map(|operation| self.create_trigger_sql(&target, operation)),
        );
        Ok(ExecutionPlan::new(statements))
    }

    pub fn plan_update(
        &self,
        connection: &Connection,
        table: &str,
        operations: &[Operation],
    ) -> Result<ExecutionPlan, AppError> {
        let target = self.describe_target(connection, table)?;
        let existing = self.existing_triggers(connection, &target.name)?;
        let mut statements = Vec::new();

        for operation in operations.iter().copied() {
            let desired_name = self.trigger_name(&target, operation);
            let desired_sql = self.create_trigger_sql(&target, operation);
            let operation_triggers = existing
                .iter()
                .filter(|trigger| trigger.operation == operation)
                .collect::<Vec<_>>();
            let current = operation_triggers
                .iter()
                .copied()
                .find(|trigger| trigger.name == desired_name);
            let current_matches = current
                .map(|trigger| sql_matches(&trigger.sql, &desired_sql))
                .unwrap_or(false);
            let stale_triggers = operation_triggers
                .iter()
                .copied()
                .filter(|trigger| trigger.name != desired_name)
                .collect::<Vec<_>>();

            if current_matches && stale_triggers.is_empty() {
                continue;
            }

            statements.extend(
                stale_triggers
                    .iter()
                    .map(|trigger| self.drop_named_trigger_sql(&trigger.name)),
            );

            if current_matches {
                continue;
            }

            if current.is_some() {
                statements.push(self.drop_named_trigger_sql(&desired_name));
            }
            statements.push(desired_sql);
        }

        Ok(ExecutionPlan::new(statements))
    }

    pub fn plan_apply_changed(
        &self,
        connection: &Connection,
        table: &str,
        operations: &[Operation],
    ) -> Result<ExecutionPlan, AppError> {
        self.plan_update(connection, table, operations)
    }

    pub fn plan_delete(
        &self,
        connection: &Connection,
        table: &str,
        operations: &[Operation],
    ) -> Result<ExecutionPlan, AppError> {
        let target = self.describe_target(connection, table)?;
        let existing = self.existing_triggers(connection, &target.name)?;
        let mut statements = Vec::new();

        for operation in operations.iter().copied() {
            let mut matched_existing = false;

            for trigger in existing
                .iter()
                .filter(|trigger| trigger.operation == operation)
            {
                matched_existing = true;
                statements.push(self.drop_named_trigger_sql(&trigger.name));
            }

            if !matched_existing {
                let desired_name = self.trigger_name(&target, operation);
                statements.push(self.drop_named_trigger_sql(&desired_name));
            }
        }

        Ok(ExecutionPlan::new(statements))
    }

    pub fn list_triggers(
        &self,
        connection: &Connection,
        table: Option<&str>,
    ) -> Result<Vec<ManagedTrigger>, AppError> {
        let like_pattern = match table {
            Some(table) => format!("{}_{}_%", self.trigger_prefix, table),
            None => format!("{}_%", self.trigger_prefix),
        };

        let mut statement = connection.prepare(
            "SELECT name, tbl_name, sql
             FROM sqlite_master
             WHERE type = 'trigger' AND name LIKE ?1
             ORDER BY name",
        )?;

        let rows = statement.query_map([like_pattern], |row| {
            Ok(ManagedTrigger {
                name: row.get(0)?,
                table: row.get(1)?,
                sql: row.get(2)?,
            })
        })?;

        rows.collect::<Result<Vec<_>, _>>().map_err(AppError::from)
    }

    pub fn list_target_tables(&self, connection: &Connection) -> Result<Vec<String>, AppError> {
        let mut statement = connection.prepare(
            "SELECT name
             FROM sqlite_master
             WHERE type = 'table'
               AND name NOT LIKE 'sqlite_%'
               AND name != ?1
             ORDER BY name",
        )?;

        let rows = statement.query_map([self.log_table.as_str()], |row| row.get(0))?;
        rows.collect::<Result<Vec<_>, _>>().map_err(AppError::from)
    }

    pub fn resolve_target_table(
        &self,
        connection: &Connection,
        table: &str,
    ) -> Result<String, AppError> {
        if table.eq_ignore_ascii_case(&self.log_table) {
            return Err(AppError::ReservedLogTable(self.log_table.clone()));
        }

        let name = resolve_table_name(connection, table)?;

        if name.eq_ignore_ascii_case(&self.log_table) {
            return Err(AppError::ReservedLogTable(name));
        }

        Ok(name)
    }

    pub fn apply_plan(
        &self,
        connection: &mut Connection,
        plan: &ExecutionPlan,
    ) -> Result<(), AppError> {
        let transaction = connection.transaction()?;

        for statement in plan.statements() {
            transaction.execute_batch(statement)?;
        }

        transaction.commit()?;
        Ok(())
    }

    pub fn ensure_log_table(&self, connection: &Connection) -> Result<(), AppError> {
        connection
            .execute_batch(&self.create_log_table_sql())
            .map_err(AppError::from)
    }

    fn describe_target(
        &self,
        connection: &Connection,
        table: &str,
    ) -> Result<TableDefinition, AppError> {
        let name = self.resolve_target_table(connection, table)?;

        let columns = table_columns(connection, &name)?;

        if columns.is_empty() {
            return Err(AppError::NoColumns(name));
        }

        let column_hash = column_hash(&columns);

        Ok(TableDefinition {
            name,
            columns,
            column_hash,
        })
    }

    fn existing_triggers(
        &self,
        connection: &Connection,
        table: &str,
    ) -> Result<Vec<ExistingTrigger>, AppError> {
        let mut statement = connection.prepare(
            "SELECT name, sql
             FROM sqlite_master
             WHERE type = 'trigger' AND tbl_name = ?1
             ORDER BY name",
        )?;

        let rows = statement.query_map([table], |row| {
            let sql = row.get::<_, Option<String>>(1)?.unwrap_or_default();
            Ok((row.get::<_, String>(0)?, sql))
        })?;

        let mut triggers = Vec::new();
        for row in rows {
            let (name, sql) = row?;
            if let Some(operation) = trigger_operation_from_name(&self.trigger_prefix, table, &name)
            {
                triggers.push(ExistingTrigger {
                    name,
                    sql,
                    operation,
                });
            }
        }

        Ok(triggers)
    }

    fn create_log_table_sql(&self) -> String {
        format!(
            "CREATE TABLE IF NOT EXISTS {log_table} (
                id INTEGER PRIMARY KEY,
                table_name TEXT NOT NULL,
                operation TEXT NOT NULL,
                old_values TEXT,
                new_values TEXT,
                changed_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
            );",
            log_table = quote_ident(&self.log_table),
        )
    }

    fn create_trigger_sql(&self, table: &TableDefinition, operation: Operation) -> String {
        let trigger_name = self.trigger_name(table, operation);
        let trigger_name = quote_ident(&trigger_name);
        let table_name = quote_ident(&table.name);
        let log_table = quote_ident(&self.log_table);
        let old_values = json_object_expr("OLD", &table.columns);
        let new_values = json_object_expr("NEW", &table.columns);

        match operation {
            Operation::Insert => format!(
                "CREATE TRIGGER {trigger_name}
                AFTER INSERT ON {table_name}
                BEGIN
                    INSERT INTO {log_table} (table_name, operation, old_values, new_values)
                    VALUES ({table_literal}, 'INSERT', NULL, {new_values});
                END;",
                table_literal = quote_string(&table.name),
            ),
            Operation::Update => format!(
                "CREATE TRIGGER {trigger_name}
                AFTER UPDATE ON {table_name}
                BEGIN
                    INSERT INTO {log_table} (table_name, operation, old_values, new_values)
                    VALUES ({table_literal}, 'UPDATE', {old_values}, {new_values});
                END;",
                table_literal = quote_string(&table.name),
            ),
            Operation::Delete => format!(
                "CREATE TRIGGER {trigger_name}
                AFTER DELETE ON {table_name}
                BEGIN
                    INSERT INTO {log_table} (table_name, operation, old_values, new_values)
                    VALUES ({table_literal}, 'DELETE', {old_values}, NULL);
                END;",
                table_literal = quote_string(&table.name),
            ),
        }
    }

    fn drop_named_trigger_sql(&self, trigger_name: &str) -> String {
        format!("DROP TRIGGER IF EXISTS {};", quote_ident(trigger_name))
    }

    fn trigger_name(&self, table: &TableDefinition, operation: Operation) -> String {
        format!(
            "{}_{}",
            self.trigger_stem(&table.name, operation),
            table.column_hash
        )
    }

    fn trigger_stem(&self, table: &str, operation: Operation) -> String {
        format!(
            "{}_{}_{}",
            self.trigger_prefix,
            table,
            operation.as_suffix()
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ManagedTrigger {
    pub name: String,
    pub table: String,
    pub sql: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExecutionPlan {
    statements: Vec<String>,
}

impl ExecutionPlan {
    fn new(statements: Vec<String>) -> Self {
        Self { statements }
    }

    pub fn from_statements(statements: Vec<String>) -> Self {
        Self { statements }
    }

    pub fn statements(&self) -> &[String] {
        &self.statements
    }

    pub fn into_statements(self) -> Vec<String> {
        self.statements
    }
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum Operation {
    Insert,
    Update,
    Delete,
}

impl Operation {
    pub fn all() -> [Self; 3] {
        [Self::Insert, Self::Update, Self::Delete]
    }

    pub fn as_suffix(self) -> &'static str {
        match self {
            Self::Insert => "insert",
            Self::Update => "update",
            Self::Delete => "delete",
        }
    }
}

struct TableDefinition {
    name: String,
    columns: Vec<String>,
    column_hash: String,
}

struct ExistingTrigger {
    name: String,
    sql: String,
    operation: Operation,
}

fn resolve_table_name(connection: &Connection, table: &str) -> Result<String, AppError> {
    let mut statement = connection.prepare(
        "SELECT name
         FROM sqlite_master
         WHERE type = 'table' AND lower(name) = lower(?1)
         LIMIT 1",
    )?;

    statement
        .query_row([table], |row| row.get(0))
        .map_err(|source| match source {
            rusqlite::Error::QueryReturnedNoRows => AppError::MissingTable(table.to_owned()),
            other => AppError::from(other),
        })
}

fn table_columns(connection: &Connection, table: &str) -> Result<Vec<String>, AppError> {
    let pragma_sql = format!("PRAGMA table_info({})", quote_string(table));
    let mut statement = connection.prepare(&pragma_sql)?;
    let mut rows = statement.query([])?;
    let mut columns = Vec::new();

    while let Some(row) = rows.next()? {
        columns.push(row.get(1)?);
    }

    Ok(columns)
}

fn json_object_expr(alias: &str, columns: &[String]) -> String {
    let entries = columns
        .iter()
        .map(|column| {
            format!(
                "{name}, {alias}.{column}",
                name = quote_string(column),
                alias = alias,
                column = quote_ident(column),
            )
        })
        .collect::<Vec<_>>()
        .join(", ");

    format!("json_object({entries})")
}

fn column_hash(columns: &[String]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(b"dolog-columns-v1");

    for column in columns {
        hasher.update((column.len() as u64).to_le_bytes());
        hasher.update(column.as_bytes());
    }

    let digest = hasher.finalize();
    digest[..8]
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

fn trigger_operation_from_name(prefix: &str, table: &str, trigger_name: &str) -> Option<Operation> {
    let stem = format!("{prefix}_{table}_").to_ascii_lowercase();
    let trigger_name = trigger_name.to_ascii_lowercase();
    let suffix = trigger_name.strip_prefix(&stem)?;

    Operation::all()
        .into_iter()
        .find(|operation| trigger_suffix_matches_operation(suffix, *operation))
}

fn trigger_suffix_matches_operation(suffix: &str, operation: Operation) -> bool {
    let operation_suffix = operation.as_suffix();

    if suffix == operation_suffix {
        return true;
    }

    suffix
        .strip_prefix(operation_suffix)
        .and_then(|suffix| suffix.strip_prefix('_'))
        .is_some_and(is_hash_suffix)
}

fn is_hash_suffix(suffix: &str) -> bool {
    suffix.len() == 16 && suffix.chars().all(|ch| ch.is_ascii_hexdigit())
}

fn sql_matches(left: &str, right: &str) -> bool {
    normalize_sql(left) == normalize_sql(right)
}

fn normalize_sql(sql: &str) -> String {
    let mut normalized = String::with_capacity(sql.len());
    let mut in_single_quote = false;
    let mut in_double_quote = false;
    let mut previous_was_whitespace = false;

    for ch in sql.trim_end_matches(';').chars() {
        match ch {
            '\'' if !in_double_quote => {
                in_single_quote = !in_single_quote;
                normalized.push(ch);
                previous_was_whitespace = false;
            }
            '"' if !in_single_quote => {
                in_double_quote = !in_double_quote;
                normalized.push(ch);
                previous_was_whitespace = false;
            }
            _ if ch.is_whitespace() => {
                if !previous_was_whitespace {
                    normalized.push(' ');
                    previous_was_whitespace = true;
                }
            }
            _ => {
                let normalized_char = if in_single_quote {
                    ch
                } else {
                    ch.to_ascii_lowercase()
                };
                normalized.push(normalized_char);
                previous_was_whitespace = false;
            }
        }
    }

    normalized.trim().to_owned()
}

fn quote_ident(value: &str) -> String {
    format!("\"{}\"", value.replace('"', "\"\""))
}

fn quote_string(value: &str) -> String {
    format!("'{}'", value.replace('\'', "''"))
}

#[derive(Debug, Error)]
pub enum AppError {
    #[error("failed to open SQLite database at '{path}': {source}")]
    OpenDatabase {
        path: String,
        #[source]
        source: rusqlite::Error,
    },
    #[error("failed to write SQL plan to '{path}': {source}")]
    WriteOutput {
        path: String,
        #[source]
        source: std::io::Error,
    },
    #[error("failed to read schema source '{path}': {source}")]
    ReadSchemaSource {
        path: String,
        #[source]
        source: std::io::Error,
    },
    #[error(
        "unsupported schema source at '{path}'; expected a SQLite database file, a directory, or a .sql file"
    )]
    UnsupportedSchemaSource { path: String },
    #[error("failed to read migration directory '{path}': {source}")]
    ReadMigrationDirectory {
        path: String,
        #[source]
        source: std::io::Error,
    },
    #[error("no .sql migration files found in '{path}'")]
    NoMigrationFiles { path: String },
    #[error("failed to read migration file '{path}': {source}")]
    ReadMigrationFile {
        path: String,
        #[source]
        source: std::io::Error,
    },
    #[error("failed to apply migration file '{path}': {source}")]
    ApplyMigration {
        path: String,
        #[source]
        source: rusqlite::Error,
    },
    #[error("--apply is only supported when the schema source path is a real SQLite database file")]
    ApplyUnsupportedWithSchemaSource,
    #[error("an output file is required unless --dry-run is used")]
    MissingExportOutput,
    #[error(transparent)]
    Json(#[from] serde_json::Error),
    #[error("table '{0}' does not exist")]
    MissingTable(String),
    #[error("table '{0}' has no columns")]
    NoColumns(String),
    #[error("table '{0}' conflicts with the configured log table")]
    ReservedLogTable(String),
    #[error(transparent)]
    Sqlite(#[from] rusqlite::Error),
}

#[cfg(test)]
mod tests {
    use super::{
        Operation, column_hash, json_object_expr, normalize_sql, quote_ident, quote_string,
        sql_matches, trigger_operation_from_name,
    };

    #[test]
    fn quotes_identifiers() {
        assert_eq!(quote_ident("a\"b"), "\"a\"\"b\"");
    }

    #[test]
    fn quotes_strings() {
        assert_eq!(quote_string("a'b"), "'a''b'");
    }

    #[test]
    fn builds_json_object() {
        let expr = json_object_expr("NEW", &["id".to_owned(), "email".to_owned()]);
        assert_eq!(
            expr,
            "json_object('id', NEW.\"id\", 'email', NEW.\"email\")"
        );
    }

    #[test]
    fn hashes_column_names_deterministically() {
        let hash = column_hash(&["id".to_owned(), "email".to_owned()]);

        assert_eq!(hash.len(), 16);
        assert_eq!(hash, column_hash(&["id".to_owned(), "email".to_owned()]));
        assert_ne!(hash, column_hash(&["email".to_owned(), "id".to_owned()]));
        assert_ne!(hash, column_hash(&["id".to_owned(), "name".to_owned()]));
    }

    #[test]
    fn parses_legacy_and_hashed_trigger_names() {
        assert_eq!(
            trigger_operation_from_name("dolog", "users", "dolog_users_insert"),
            Some(Operation::Insert)
        );
        assert_eq!(
            trigger_operation_from_name("dolog", "users", "dolog_users_update_0123456789abcdef"),
            Some(Operation::Update)
        );
        assert_eq!(
            trigger_operation_from_name("dolog", "users", "dolog_users_insert_archive"),
            None
        );
    }

    #[test]
    fn normalizes_sql_whitespace() {
        assert_eq!(
            normalize_sql("CREATE TRIGGER a\n  AFTER INSERT ON users\nBEGIN\n  SELECT 1;\nEND"),
            "create trigger a after insert on users begin select 1; end"
        );
    }

    #[test]
    fn matches_sql_after_normalization() {
        assert!(sql_matches(
            "CREATE TRIGGER a\nAFTER INSERT ON users BEGIN SELECT 1; END",
            "CREATE   TRIGGER   a AFTER INSERT ON users BEGIN SELECT 1; END"
        ));
    }

    #[test]
    fn preserves_single_quoted_string_case() {
        assert_eq!(
            normalize_sql("VALUES ('Users', 'INSERT')"),
            "values ('Users', 'INSERT')"
        );
    }
}
