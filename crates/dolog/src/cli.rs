use std::{collections::BTreeSet, fs, path::PathBuf};

use clap::{Args, Parser, Subcommand, ValueEnum};

use crate::trigger::{
    AppError, ExecutionPlan, ManagedTrigger, Operation, TriggerManager, open_connection,
};

#[derive(Debug, Parser)]
#[command(name = "dolog")]
#[command(about = "Manage SQLite triggers for change logging")]
pub struct Cli {
    #[command(subcommand)]
    command: Command,
}

pub fn run(cli: Cli) -> Result<(), AppError> {
    match cli.command {
        Command::Trigger(trigger) => trigger.run(),
    }
}

#[derive(Debug, Subcommand)]
enum Command {
    Trigger(TriggerCommand),
}

#[derive(Debug, Args)]
struct TriggerCommand {
    #[command(subcommand)]
    action: TriggerAction,
}

impl TriggerCommand {
    fn run(self) -> Result<(), AppError> {
        match self.action {
            TriggerAction::Create(args) => args.run(
                |manager, connection, table, operations| {
                    manager.plan_create(connection, table, operations)
                },
                "Created",
            ),
            TriggerAction::Update(args) => args.run(
                |manager, connection, table, operations| {
                    manager.plan_update(connection, table, operations)
                },
                "Updated",
            ),
            TriggerAction::Delete(args) => args.run(
                |manager, connection, table, operations| {
                    manager.plan_delete(connection, table, operations)
                },
                "Deleted",
            ),
            TriggerAction::Status(args) => args.run(),
        }
    }
}

#[derive(Debug, Subcommand)]
enum TriggerAction {
    Create(TriggerArgs),
    Update(TriggerArgs),
    Delete(TriggerArgs),
    Status(StatusArgs),
}

#[derive(Debug, Args)]
struct TriggerArgs {
    db: PathBuf,
    #[arg(
        long,
        conflicts_with = "all_tables",
        required_unless_present = "all_tables"
    )]
    table: Vec<String>,
    #[arg(long, conflicts_with = "table")]
    all_tables: bool,
    #[arg(long, default_value = "_dolog_changes")]
    log_table: String,
    #[arg(long, default_value = "dolog")]
    trigger_prefix: String,
    #[arg(long, value_enum)]
    operation: Vec<OperationArg>,
    #[arg(long, conflicts_with = "output")]
    dry_run: bool,
    #[arg(long, value_name = "FILE", conflicts_with = "dry_run")]
    output: Option<PathBuf>,
}

impl TriggerArgs {
    fn run(
        self,
        planner: impl Fn(
            &TriggerManager,
            &rusqlite::Connection,
            &str,
            &[Operation],
        ) -> Result<ExecutionPlan, AppError>,
        success_verb: &str,
    ) -> Result<(), AppError> {
        let mut connection = open_connection(&self.db)?;
        let manager = TriggerManager::new(self.log_table, self.trigger_prefix);
        let tables = resolve_tables(&manager, &connection, self.table, self.all_tables)?;
        let operations = resolve_operations(self.operation);
        let plan = collect_plan(&manager, &connection, &tables, &operations, planner)?;

        if self.dry_run {
            print_statements(plan.statements());
            return Ok(());
        }

        if let Some(output_path) = self.output {
            write_plan(&output_path, &plan)?;
            println!("Wrote SQL plan to '{}'.", output_path.display());
            return Ok(());
        }

        manager.apply_plan(&mut connection, &plan)?;
        println!(
            "{success_verb} triggers for {}.",
            format_table_targets(&tables)
        );
        Ok(())
    }
}

#[derive(Debug, Args)]
struct StatusArgs {
    db: PathBuf,
    #[arg(long)]
    table: Vec<String>,
    #[arg(long, default_value = "dolog")]
    trigger_prefix: String,
    #[arg(long, default_value = "_dolog_changes")]
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

        for table in tables {
            let status = TableStatus::from_triggers(&self.trigger_prefix, &table, &triggers);
            println!(
                "{table} | insert: {} | update: {} | delete: {}",
                yes_no(status.insert),
                yes_no(status.update),
                yes_no(status.delete)
            );
        }

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
