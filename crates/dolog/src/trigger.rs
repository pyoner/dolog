use std::path::Path;

use rusqlite::Connection;
use thiserror::Error;

pub fn open_connection(path: &Path) -> Result<Connection, AppError> {
    Connection::open(path).map_err(|source| AppError::OpenDatabase {
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
        let plan = self.plan_create(connection, table)?;
        self.apply_plan(connection, &plan)
    }

    pub fn update(&self, connection: &mut Connection, table: &str) -> Result<(), AppError> {
        let plan = self.plan_update(connection, table)?;
        self.apply_plan(connection, &plan)
    }

    pub fn delete(&self, connection: &mut Connection, table: &str) -> Result<(), AppError> {
        let plan = self.plan_delete(connection, table)?;
        self.apply_plan(connection, &plan)
    }

    pub fn preview_create(
        &self,
        connection: &Connection,
        table: &str,
    ) -> Result<Vec<String>, AppError> {
        Ok(self.plan_create(connection, table)?.into_statements())
    }

    pub fn preview_update(
        &self,
        connection: &Connection,
        table: &str,
    ) -> Result<Vec<String>, AppError> {
        Ok(self.plan_update(connection, table)?.into_statements())
    }

    pub fn preview_delete(
        &self,
        connection: &Connection,
        table: &str,
    ) -> Result<Vec<String>, AppError> {
        Ok(self.plan_delete(connection, table)?.into_statements())
    }

    pub fn plan_create(
        &self,
        connection: &Connection,
        table: &str,
    ) -> Result<ExecutionPlan, AppError> {
        let target = self.describe_target(connection, table)?;
        let mut statements = vec![self.create_log_table_sql()];
        statements.extend(
            Operation::all()
                .into_iter()
                .map(|operation| self.create_trigger_sql(&target, operation)),
        );
        Ok(ExecutionPlan::new(statements))
    }

    pub fn plan_update(
        &self,
        connection: &Connection,
        table: &str,
    ) -> Result<ExecutionPlan, AppError> {
        let target = self.describe_target(connection, table)?;
        let mut statements = vec![self.create_log_table_sql()];
        statements.extend(
            Operation::all()
                .into_iter()
                .map(|operation| self.drop_trigger_sql(&target.name, operation)),
        );
        statements.extend(
            Operation::all()
                .into_iter()
                .map(|operation| self.create_trigger_sql(&target, operation)),
        );
        Ok(ExecutionPlan::new(statements))
    }

    pub fn plan_delete(
        &self,
        connection: &Connection,
        table: &str,
    ) -> Result<ExecutionPlan, AppError> {
        let target = self.describe_target(connection, table)?;
        Ok(ExecutionPlan::new(
            Operation::all()
                .into_iter()
                .map(|operation| self.drop_trigger_sql(&target.name, operation))
                .collect(),
        ))
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

    fn describe_target(
        &self,
        connection: &Connection,
        table: &str,
    ) -> Result<TableDefinition, AppError> {
        if table == self.log_table {
            return Err(AppError::ReservedLogTable(table.to_owned()));
        }

        ensure_table_exists(connection, table)?;
        let columns = table_columns(connection, table)?;

        if columns.is_empty() {
            return Err(AppError::NoColumns(table.to_owned()));
        }

        Ok(TableDefinition {
            name: table.to_owned(),
            columns,
        })
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
        let trigger_name = self.trigger_name(&table.name, operation);
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

    fn drop_trigger_sql(&self, table: &str, operation: Operation) -> String {
        let trigger_name = self.trigger_name(table, operation);
        format!("DROP TRIGGER IF EXISTS {};", quote_ident(&trigger_name))
    }

    fn trigger_name(&self, table: &str, operation: Operation) -> String {
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

#[derive(Clone, Copy)]
enum Operation {
    Insert,
    Update,
    Delete,
}

impl Operation {
    fn all() -> [Self; 3] {
        [Self::Insert, Self::Update, Self::Delete]
    }

    fn as_suffix(self) -> &'static str {
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
}

fn ensure_table_exists(connection: &Connection, table: &str) -> Result<(), AppError> {
    let mut statement = connection.prepare(
        "SELECT 1
         FROM sqlite_master
         WHERE type = 'table' AND name = ?1
         LIMIT 1",
    )?;

    let exists = statement.exists([table])?;
    if exists {
        Ok(())
    } else {
        Err(AppError::MissingTable(table.to_owned()))
    }
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
    use super::{json_object_expr, quote_ident, quote_string};

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
}
