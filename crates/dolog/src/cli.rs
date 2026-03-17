use std::{collections::BTreeSet, fs, path::PathBuf};

use clap::{Args, Parser, Subcommand};

use crate::trigger::{AppError, ExecutionPlan, TriggerManager, open_connection};

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
                |manager, connection, table| manager.plan_create(connection, table),
                "Created",
            ),
            TriggerAction::Update(args) => args.run(
                |manager, connection, table| manager.plan_update(connection, table),
                "Updated",
            ),
            TriggerAction::Delete(args) => args.run(
                |manager, connection, table| manager.plan_delete(connection, table),
                "Deleted",
            ),
            TriggerAction::List(args) => args.run(),
            TriggerAction::Preview(args) => args.run(),
        }
    }
}

#[derive(Debug, Subcommand)]
enum TriggerAction {
    Create(TriggerArgs),
    Update(TriggerArgs),
    Delete(TriggerArgs),
    List(ListTriggerArgs),
    Preview(PreviewTriggerArgs),
}

#[derive(Debug, Args)]
struct TriggerArgs {
    #[arg(long)]
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
        ) -> Result<ExecutionPlan, AppError>,
        success_verb: &str,
    ) -> Result<(), AppError> {
        let mut connection = open_connection(&self.db)?;
        let manager = TriggerManager::new(self.log_table, self.trigger_prefix);
        let tables = resolve_tables(&manager, &connection, self.table, self.all_tables)?;
        let plan = collect_plan(&manager, &connection, &tables, planner)?;

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
struct ListTriggerArgs {
    #[arg(long)]
    db: PathBuf,
    #[arg(
        long,
        conflicts_with = "all_tables",
        required_unless_present = "all_tables"
    )]
    table: Vec<String>,
    #[arg(long, conflicts_with = "table")]
    all_tables: bool,
    #[arg(long, default_value = "dolog")]
    trigger_prefix: String,
    #[arg(long, default_value = "_dolog_changes")]
    log_table: String,
}

impl ListTriggerArgs {
    fn run(self) -> Result<(), AppError> {
        let connection = open_connection(&self.db)?;
        let manager = TriggerManager::new(self.log_table, self.trigger_prefix);
        let tables = resolve_tables(&manager, &connection, self.table, self.all_tables)?;
        let triggers = if tables.is_empty() {
            manager.list_triggers(&connection, None)?
        } else {
            let mut triggers = Vec::new();
            for table in &tables {
                triggers.extend(manager.list_triggers(&connection, Some(table))?);
            }
            triggers
        };

        if triggers.is_empty() {
            println!("No managed triggers found.");
            return Ok(());
        }

        for trigger in triggers {
            println!("{} ({})", trigger.name, trigger.table);
        }

        Ok(())
    }
}

#[derive(Debug, Args)]
struct PreviewTriggerArgs {
    #[command(subcommand)]
    action: PreviewAction,
}

impl PreviewTriggerArgs {
    fn run(self) -> Result<(), AppError> {
        match self.action {
            PreviewAction::Create(args) => args.run(
                |manager, connection, table| manager.plan_create(connection, table),
                "",
            ),
            PreviewAction::Update(args) => args.run(
                |manager, connection, table| manager.plan_update(connection, table),
                "",
            ),
            PreviewAction::Delete(args) => args.run(
                |manager, connection, table| manager.plan_delete(connection, table),
                "",
            ),
        }
    }
}

#[derive(Debug, Subcommand)]
enum PreviewAction {
    Create(PreviewArgs),
    Update(PreviewArgs),
    Delete(PreviewArgs),
}

#[derive(Debug, Args)]
struct PreviewArgs {
    #[arg(long)]
    db: PathBuf,
    #[arg(long, conflicts_with = "all_tables")]
    table: Vec<String>,
    #[arg(long, conflicts_with = "table")]
    all_tables: bool,
    #[arg(long, default_value = "_dolog_changes")]
    log_table: String,
    #[arg(long, default_value = "dolog")]
    trigger_prefix: String,
}

impl PreviewArgs {
    fn run(
        self,
        planner: impl Fn(
            &TriggerManager,
            &rusqlite::Connection,
            &str,
        ) -> Result<ExecutionPlan, AppError>,
        _unused_success_verb: &str,
    ) -> Result<(), AppError> {
        let connection = open_connection(&self.db)?;
        let manager = TriggerManager::new(self.log_table, self.trigger_prefix);
        let tables = resolve_tables(&manager, &connection, self.table, self.all_tables)?;
        let plan = collect_plan(&manager, &connection, &tables, planner)?;
        print_statements(plan.statements());
        Ok(())
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

fn collect_plan(
    manager: &TriggerManager,
    connection: &rusqlite::Connection,
    tables: &[String],
    planner: impl Fn(&TriggerManager, &rusqlite::Connection, &str) -> Result<ExecutionPlan, AppError>,
) -> Result<ExecutionPlan, AppError> {
    let mut statements = Vec::new();

    for table in tables {
        let plan = planner(manager, connection, table)?;
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
