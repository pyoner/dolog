use std::{
    fs::OpenOptions,
    io::{BufWriter, Write},
    path::Path,
};

use rusqlite::{Connection, params_from_iter};
use serde::Serialize;
use serde_json::Value;

use crate::trigger::AppError;

pub fn export_logs(
    connection: &mut Connection,
    log_table: &str,
    output: &Path,
    limit: Option<usize>,
) -> Result<ExportResult, AppError> {
    let entries = read_entries(connection, log_table, limit)?;

    if entries.is_empty() {
        return Ok(ExportResult { exported: 0 });
    }

    write_entries(output, &entries)?;
    delete_entries(connection, log_table, &entries)?;

    Ok(ExportResult {
        exported: entries.len(),
    })
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ExportResult {
    pub exported: usize,
}

#[derive(Debug, Serialize)]
struct LogEntry {
    id: i64,
    table_name: String,
    operation: String,
    old_values: Option<Value>,
    new_values: Option<Value>,
    changed_at: String,
}

#[derive(Debug)]
struct RawLogEntry {
    id: i64,
    table_name: String,
    operation: String,
    old_values: Option<String>,
    new_values: Option<String>,
    changed_at: String,
}

fn read_entries(
    connection: &Connection,
    log_table: &str,
    limit: Option<usize>,
) -> Result<Vec<LogEntry>, AppError> {
    let mut sql = format!(
        "SELECT id, table_name, operation, old_values, new_values, changed_at
         FROM {}
         ORDER BY id",
        quote_ident(log_table)
    );

    if let Some(limit) = limit {
        sql.push_str(&format!(" LIMIT {limit}"));
    }

    let mut statement = connection.prepare(&sql)?;
    let rows = statement.query_map([], |row| {
        Ok(RawLogEntry {
            id: row.get(0)?,
            table_name: row.get(1)?,
            operation: row.get(2)?,
            old_values: row.get(3)?,
            new_values: row.get(4)?,
            changed_at: row.get(5)?,
        })
    })?;
    let raw_entries = rows
        .collect::<Result<Vec<_>, _>>()
        .map_err(AppError::from)?;

    raw_entries
        .into_iter()
        .map(|entry| {
            Ok(LogEntry {
                id: entry.id,
                table_name: entry.table_name,
                operation: entry.operation,
                old_values: parse_json_option(entry.old_values)?,
                new_values: parse_json_option(entry.new_values)?,
                changed_at: entry.changed_at,
            })
        })
        .collect()
}

fn write_entries(output: &Path, entries: &[LogEntry]) -> Result<(), AppError> {
    let file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(output)
        .map_err(|source| AppError::WriteOutput {
            path: output.display().to_string(),
            source,
        })?;

    let mut writer = BufWriter::new(file);

    for entry in entries {
        let line = serde_json::to_string(entry)?;
        writer
            .write_all(line.as_bytes())
            .map_err(|source| AppError::WriteOutput {
                path: output.display().to_string(),
                source,
            })?;
        writer
            .write_all(b"\n")
            .map_err(|source| AppError::WriteOutput {
                path: output.display().to_string(),
                source,
            })?;
    }

    writer.flush().map_err(|source| AppError::WriteOutput {
        path: output.display().to_string(),
        source,
    })?;

    Ok(())
}

fn delete_entries(
    connection: &mut Connection,
    log_table: &str,
    entries: &[LogEntry],
) -> Result<(), AppError> {
    let placeholders = vec!["?"; entries.len()].join(", ");
    let sql = format!(
        "DELETE FROM {} WHERE id IN ({placeholders})",
        quote_ident(log_table)
    );
    let ids = entries.iter().map(|entry| entry.id).collect::<Vec<_>>();
    let transaction = connection.transaction()?;
    transaction.execute(&sql, params_from_iter(ids))?;
    transaction.commit()?;
    Ok(())
}

fn parse_json_option(value: Option<String>) -> Result<Option<Value>, AppError> {
    value
        .map(|value| serde_json::from_str(&value))
        .transpose()
        .map_err(AppError::from)
}

fn quote_ident(value: &str) -> String {
    format!("\"{}\"", value.replace('"', "\"\""))
}
