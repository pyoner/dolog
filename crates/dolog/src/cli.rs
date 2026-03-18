use std::{collections::BTreeSet, fs, path::PathBuf};

use clap::{Args, Parser, Subcommand, ValueEnum};

use crate::log_export::{export_logs, log_status, preview_logs};
use crate::trigger::{
    AppError, ExecutionPlan, ManagedTrigger, Operation, TriggerManager, open_connection,
};

#[derive(Debug, Parser)]
#[command(name = "dolog")]
#[command(
    about = "Manage SQLite change capture and log export",
    long_about = "Manage SQLite trigger generation, trigger status, pending log status, and JSONL log export."
)]
pub struct Cli {
    #[command(subcommand)]
    command: Command,
}

pub fn run(cli: Cli) -> Result<(), AppError> {
    match cli.command {
        Command::Log(log) => log.run(),
        Command::Trigger(trigger) => trigger.run(),
    }
}

#[derive(Debug, Subcommand)]
enum Command {
    #[command(
        about = "Inspect and export captured change rows",
        long_about = "Inspect pending rows in the dolog log table and export those rows as JSON Lines."
    )]
    Log(LogCommand),
    #[command(
        about = "Generate trigger SQL and inspect trigger coverage",
        long_about = "Generate SQLite trigger SQL for one or more tables, optionally apply it directly, and inspect trigger coverage."
    )]
    Trigger(TriggerCommand),
}

#[derive(Debug, Args)]
struct LogCommand {
    #[command(subcommand)]
    action: LogAction,
}

impl LogCommand {
    fn run(self) -> Result<(), AppError> {
        match self.action {
            LogAction::Export(args) => args.run(),
            LogAction::Status(args) => args.run(),
        }
    }
}

#[derive(Debug, Subcommand)]
enum LogAction {
    #[command(
        about = "Export pending change rows as JSON Lines",
        long_about = "Export rows from the dolog log table as JSON Lines. In normal mode, the command appends rows to an output file and then removes those rows from the database. In dry-run mode, it writes the same JSONL rows to stdout and does not delete them.",
        after_help = "Examples:\n  dolog log export db.sqlite changes.jsonl\n  dolog log export db.sqlite changes.jsonl --limit 100\n  dolog log export db.sqlite --dry-run"
    )]
    Export(LogExportArgs),
    #[command(
        about = "Show pending log rows grouped by table and operation",
        long_about = "Show the pending rows currently stored in the dolog log table, grouped by source table and operation. This command only reads from the database.",
        after_help = "Example:\n  dolog log status db.sqlite"
    )]
    Status(LogStatusArgs),
}

#[derive(Debug, Args)]
struct LogExportArgs {
    #[arg(help = "SQLite database file to read pending change rows from")]
    db: PathBuf,
    #[arg(
        conflicts_with = "dry_run",
        help = "Write exported JSONL rows to this file"
    )]
    output: Option<PathBuf>,
    #[arg(
        long,
        default_value = "_dolog_changes",
        help = "Name of the dolog log table"
    )]
    log_table: String,
    #[arg(long, help = "Export at most this many rows")]
    limit: Option<usize>,
    #[arg(
        long,
        conflicts_with = "output",
        help = "Write JSONL rows to stdout without deleting them from the database"
    )]
    dry_run: bool,
}

impl LogExportArgs {
    fn run(self) -> Result<(), AppError> {
        let mut connection = open_connection(&self.db)?;
        if self.dry_run {
            let lines = preview_logs(&connection, &self.log_table, self.limit)?;
            for line in lines {
                println!("{line}");
            }
            return Ok(());
        }

        let output = self.output.ok_or(AppError::MissingExportOutput)?;
        let result = export_logs(&mut connection, &self.log_table, &output, self.limit)?;

        println!(
            "Exported {} change rows to '{}'.",
            result.exported,
            output.display()
        );
        Ok(())
    }
}

#[derive(Debug, Args)]
struct LogStatusArgs {
    #[arg(help = "SQLite database file to inspect")]
    db: PathBuf,
    #[arg(
        long,
        default_value = "_dolog_changes",
        help = "Name of the dolog log table"
    )]
    log_table: String,
}

impl LogStatusArgs {
    fn run(self) -> Result<(), AppError> {
        let connection = open_connection(&self.db)?;
        let rows = log_status(&connection, &self.log_table)?;

        if rows.is_empty() {
            println!("No pending log rows for {}.", self.db.display());
            return Ok(());
        }

        print_log_status_table(&self.db, &rows);
        Ok(())
    }
}

#[derive(Debug, Args)]
struct TriggerCommand {
    #[command(subcommand)]
    action: TriggerAction,
}

impl TriggerCommand {
    fn run(self) -> Result<(), AppError> {
        match self.action {
            TriggerAction::Generate(args) => args.run(),
            TriggerAction::Status(args) => args.run(),
        }
    }
}

#[derive(Debug, Subcommand)]
enum TriggerAction {
    #[command(
        about = "Generate trigger SQL for one or more tables",
        long_about = "Generate SQLite trigger SQL for the selected tables and operations. By default the SQL is written to stdout. Provide a SQL file path to write a migration artifact, or use --apply to execute the generated SQL directly against the database. Use --drop to generate trigger-removal SQL instead of create-or-refresh SQL.",
        after_help = "Examples:\n  dolog trigger generate db.sqlite --table users\n  dolog trigger generate db.sqlite 001_users_triggers.sql --table users\n  dolog trigger generate db.sqlite --table users --apply\n  dolog trigger generate db.sqlite --drop --table users"
    )]
    Generate(TriggerGenerateArgs),
    #[command(
        about = "Show trigger coverage for one or more tables",
        long_about = "Show whether dolog-managed INSERT, UPDATE, and DELETE triggers are present for each selected table. When --table is omitted, status is shown for all user tables except the dolog log table.",
        after_help = "Examples:\n  dolog trigger status db.sqlite\n  dolog trigger status db.sqlite --table users"
    )]
    Status(StatusArgs),
}

#[derive(Debug, Args)]
struct TriggerGenerateArgs {
    #[arg(help = "SQLite database file to inspect or modify")]
    db: PathBuf,
    #[arg(
        conflicts_with = "apply",
        help = "Write generated SQL to this file instead of stdout"
    )]
    sql_file: Option<PathBuf>,
    #[arg(
        long,
        help = "Generate DROP TRIGGER statements instead of create-or-refresh SQL"
    )]
    drop: bool,
    #[arg(
        long,
        conflicts_with = "all_tables",
        required_unless_present = "all_tables",
        help = "Target a specific table; repeat to target multiple tables"
    )]
    table: Vec<String>,
    #[arg(
        long,
        conflicts_with = "table",
        help = "Target all user tables except the dolog log table"
    )]
    all_tables: bool,
    #[arg(
        long,
        default_value = "_dolog_changes",
        help = "Name of the dolog log table"
    )]
    log_table: String,
    #[arg(
        long,
        default_value = "dolog",
        help = "Prefix used for managed trigger names"
    )]
    trigger_prefix: String,
    #[arg(
        long,
        value_enum,
        help = "Limit generation to specific operations; defaults to insert, update, and delete"
    )]
    operation: Vec<OperationArg>,
    #[arg(
        long,
        conflicts_with = "sql_file",
        help = "Apply the generated SQL directly to the database"
    )]
    apply: bool,
}

impl TriggerGenerateArgs {
    fn run(self) -> Result<(), AppError> {
        let mut connection = open_connection(&self.db)?;
        let manager = TriggerManager::new(self.log_table, self.trigger_prefix);
        let tables = resolve_tables(&manager, &connection, self.table, self.all_tables)?;
        let operations = resolve_operations(self.operation);
        let plan = if self.drop {
            collect_plan(
                &manager,
                &connection,
                &tables,
                &operations,
                |manager, connection, table, operations| {
                    manager.plan_delete(connection, table, operations)
                },
            )?
        } else {
            collect_plan(
                &manager,
                &connection,
                &tables,
                &operations,
                |manager, connection, table, operations| {
                    manager.plan_update(connection, table, operations)
                },
            )?
        };

        if self.apply {
            manager.apply_plan(&mut connection, &plan)?;
            println!("Applied trigger SQL for {}.", format_table_targets(&tables));
            return Ok(());
        }

        if let Some(path) = self.sql_file {
            write_plan(&path, &plan)?;
            println!("Wrote trigger SQL to '{}'.", path.display());
            return Ok(());
        }

        print_statements(plan.statements());
        Ok(())
    }
}

#[derive(Debug, Args)]
struct StatusArgs {
    #[arg(help = "SQLite database file to inspect")]
    db: PathBuf,
    #[arg(help = "Show status for a specific table; repeat to check multiple tables")]
    #[arg(long)]
    table: Vec<String>,
    #[arg(
        long,
        default_value = "dolog",
        help = "Prefix used for managed trigger names"
    )]
    trigger_prefix: String,
    #[arg(
        long,
        default_value = "_dolog_changes",
        help = "Name of the dolog log table"
    )]
    log_table: String,
}

impl StatusArgs {
    fn run(self) -> Result<(), AppError> {
        let connection = open_connection(&self.db)?;
        let manager = TriggerManager::new(self.log_table, self.trigger_prefix.clone());
        let tables = if self.table.is_empty() {
            manager.list_target_tables(&connection)?
        } else {
            unique_tables(self.table)
        };
        let triggers = manager.list_triggers(&connection, None)?;

        if tables.is_empty() {
            println!("No matching tables found.");
            return Ok(());
        }

        let rows = tables
            .into_iter()
            .map(|table| {
                let status = TableStatus::from_triggers(&self.trigger_prefix, &table, &triggers);
                StatusRow { table, status }
            })
            .collect::<Vec<_>>();

        print_status_table(&self.db, &rows);

        Ok(())
    }
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd, ValueEnum)]
enum OperationArg {
    Insert,
    Update,
    Delete,
}

#[derive(Default)]
struct TableStatus {
    insert: bool,
    update: bool,
    delete: bool,
}

struct StatusRow {
    table: String,
    status: TableStatus,
}

impl TableStatus {
    fn from_triggers(prefix: &str, table: &str, triggers: &[ManagedTrigger]) -> Self {
        let mut status = Self::default();

        for trigger in triggers.iter().filter(|trigger| trigger.table == table) {
            match operation_from_trigger_name(prefix, table, &trigger.name) {
                Some(Operation::Insert) => status.insert = true,
                Some(Operation::Update) => status.update = true,
                Some(Operation::Delete) => status.delete = true,
                None => {}
            }
        }

        status
    }
}

fn operation_from_trigger_name(prefix: &str, table: &str, trigger_name: &str) -> Option<Operation> {
    let stem = format!("{prefix}_{table}_");
    let suffix = trigger_name.strip_prefix(&stem)?;

    match suffix {
        "insert" => Some(Operation::Insert),
        "update" => Some(Operation::Update),
        "delete" => Some(Operation::Delete),
        _ => None,
    }
}

fn yes_no(value: bool) -> &'static str {
    if value { "yes" } else { "no" }
}

fn print_status_table(db: &PathBuf, rows: &[StatusRow]) {
    let table_width = rows
        .iter()
        .map(|row| row.table.len())
        .max()
        .unwrap_or(5)
        .max("TABLE".len());

    println!("Trigger status for {}", db.display());
    println!();
    println!(
        "{:<table_width$}  {:<6}  {:<6}  {:<6}",
        "TABLE",
        "INSERT",
        "UPDATE",
        "DELETE",
        table_width = table_width,
    );

    for row in rows {
        println!(
            "{:<table_width$}  {:<6}  {:<6}  {:<6}",
            row.table,
            yes_no(row.status.insert),
            yes_no(row.status.update),
            yes_no(row.status.delete),
            table_width = table_width,
        );
    }
}

fn print_statements(statements: &[String]) {
    for (index, statement) in statements.iter().enumerate() {
        if index > 0 {
            println!();
        }
        println!("{statement}");
    }
}

fn write_plan(path: &PathBuf, plan: &ExecutionPlan) -> Result<(), AppError> {
    let contents = format!("{}\n", plan.statements().join("\n\n"));
    fs::write(path, contents).map_err(|source| AppError::WriteOutput {
        path: path.display().to_string(),
        source,
    })
}

fn print_log_status_table(db: &PathBuf, rows: &[crate::log_export::LogStatusRow]) {
    let table_width = rows
        .iter()
        .map(|row| row.table_name.len())
        .max()
        .unwrap_or(5)
        .max("TABLE".len());
    let operation_width = rows
        .iter()
        .map(|row| row.operation.len())
        .max()
        .unwrap_or(9)
        .max("OPERATION".len());
    let count_width = rows
        .iter()
        .map(|row| row.count.to_string().len())
        .max()
        .unwrap_or(5)
        .max("COUNT".len());
    let total = rows.iter().map(|row| row.count).sum::<i64>();

    println!("Pending log rows for {}", db.display());
    println!();
    println!(
        "{:<table_width$}  {:<operation_width$}  {:>count_width$}",
        "TABLE",
        "OPERATION",
        "COUNT",
        table_width = table_width,
        operation_width = operation_width,
        count_width = count_width,
    );

    for row in rows {
        println!(
            "{:<table_width$}  {:<operation_width$}  {:>count_width$}",
            row.table_name,
            row.operation,
            row.count,
            table_width = table_width,
            operation_width = operation_width,
            count_width = count_width,
        );
    }

    println!();
    println!(
        "{:<table_width$}  {:<operation_width$}  {:>count_width$}",
        "TOTAL",
        "",
        total,
        table_width = table_width,
        operation_width = operation_width,
        count_width = count_width,
    );
}

fn collect_plan(
    manager: &TriggerManager,
    connection: &rusqlite::Connection,
    tables: &[String],
    operations: &[Operation],
    planner: impl Fn(
        &TriggerManager,
        &rusqlite::Connection,
        &str,
        &[Operation],
    ) -> Result<ExecutionPlan, AppError>,
) -> Result<ExecutionPlan, AppError> {
    let mut statements = Vec::new();

    for table in tables {
        let plan = planner(manager, connection, table, operations)?;
        statements.extend_from_slice(plan.statements());
    }

    Ok(ExecutionPlan::from_statements(statements))
}

fn unique_tables(tables: Vec<String>) -> Vec<String> {
    let mut seen = BTreeSet::new();
    let mut unique = Vec::new();

    for table in tables {
        if seen.insert(table.clone()) {
            unique.push(table);
        }
    }

    unique
}

fn resolve_tables(
    manager: &TriggerManager,
    connection: &rusqlite::Connection,
    tables: Vec<String>,
    all_tables: bool,
) -> Result<Vec<String>, AppError> {
    if all_tables {
        return manager.list_target_tables(connection);
    }

    Ok(unique_tables(tables))
}

fn resolve_operations(operations: Vec<OperationArg>) -> Vec<Operation> {
    if operations.is_empty() {
        return Operation::all().to_vec();
    }

    let mut seen = BTreeSet::new();
    let mut unique = Vec::new();

    for operation in operations {
        let operation = match operation {
            OperationArg::Insert => Operation::Insert,
            OperationArg::Update => Operation::Update,
            OperationArg::Delete => Operation::Delete,
        };

        if seen.insert(operation) {
            unique.push(operation);
        }
    }

    unique
}

fn format_table_targets(tables: &[String]) -> String {
    if tables.len() == 1 {
        format!("table '{}'", tables[0])
    } else {
        let joined = tables
            .iter()
            .map(|table| format!("'{table}'"))
            .collect::<Vec<_>>()
            .join(", ");
        format!("tables {joined}")
    }
}
