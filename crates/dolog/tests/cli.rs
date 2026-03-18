use std::time::{SystemTime, UNIX_EPOCH};

use assert_cmd::Command;
use predicates::prelude::*;
use rusqlite::Connection;

#[test]
fn top_level_help_describes_trigger_and_log_commands() {
    Command::cargo_bin("dolog")
        .expect("build dolog binary")
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "Manage SQLite trigger generation, trigger status, pending log status, and JSONL log export.",
        ))
        .stdout(predicate::str::contains(
            "Generate trigger SQL and inspect trigger coverage",
        ))
        .stdout(predicate::str::contains(
            "Inspect and export captured change rows",
        ));
}

#[test]
fn trigger_generate_help_includes_notes_and_examples() {
    Command::cargo_bin("dolog")
        .expect("build dolog binary")
        .args(["trigger", "generate", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "Generate SQLite trigger SQL for the selected tables and operations.",
        ))
        .stdout(predicate::str::contains(
            "By default the SQL is written to stdout.",
        ))
        .stdout(predicate::str::contains(
            "Target all user tables except the dolog log table",
        ))
        .stdout(predicate::str::contains(
            "Generate DROP TRIGGER statements instead of create-or-refresh SQL",
        ))
        .stdout(predicate::str::contains(
            "dolog trigger generate db.sqlite --table users",
        ))
        .stdout(predicate::str::contains(
            "dolog trigger generate db.sqlite --drop --table users",
        ));
}

#[test]
fn trigger_status_help_describes_default_scope() {
    Command::cargo_bin("dolog")
        .expect("build dolog binary")
        .args(["trigger", "status", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "Show whether dolog-managed INSERT, UPDATE, and DELETE triggers are present",
        ))
        .stdout(predicate::str::contains(
            "When --table is omitted, status is shown for all user tables",
        ))
        .stdout(predicate::str::contains(
            "dolog trigger status db.sqlite --table users",
        ));
}

#[test]
fn log_export_help_describes_dry_run_and_output_modes() {
    Command::cargo_bin("dolog")
        .expect("build dolog binary")
        .args(["log", "export", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "Export rows from the dolog log table as JSON Lines.",
        ))
        .stdout(predicate::str::contains(
            "In dry-run mode, it writes the same JSONL rows to stdout and does not delete them.",
        ))
        .stdout(predicate::str::contains(
            "Write exported JSONL rows to this file",
        ))
        .stdout(predicate::str::contains(
            "dolog log export db.sqlite --dry-run",
        ));
}

#[test]
fn log_status_help_describes_read_only_status() {
    Command::cargo_bin("dolog")
        .expect("build dolog binary")
        .args(["log", "status", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "Show the pending rows currently stored in the dolog log table",
        ))
        .stdout(predicate::str::contains(
            "This command only reads from the database.",
        ))
        .stdout(predicate::str::contains("dolog log status db.sqlite"));
}

#[test]
fn generate_prints_sql_to_stdout_without_modifying_database() {
    let db_path = unique_db_path();
    let connection = Connection::open(&db_path).expect("create sqlite database");
    connection
        .execute_batch(
            "CREATE TABLE users (
                id INTEGER PRIMARY KEY,
                email TEXT NOT NULL
            );",
        )
        .expect("create users table");
    drop(connection);

    Command::cargo_bin("dolog")
        .expect("build dolog binary")
        .args([
            "trigger",
            "generate",
            db_path.to_str().expect("utf8 path"),
            "--table",
            "users",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "CREATE TABLE IF NOT EXISTS \"_dolog_changes\"",
        ))
        .stdout(predicate::str::contains(
            "CREATE TRIGGER \"dolog_users_insert\"",
        ))
        .stdout(predicate::str::contains(
            "DROP TRIGGER IF EXISTS \"dolog_users_insert\";",
        ));

    let connection = Connection::open(&db_path).expect("open sqlite database");
    assert!(!table_exists(&connection, "_dolog_changes"));
    assert!(trigger_names(&connection).is_empty());

    std::fs::remove_file(db_path).expect("remove temp db");
}

#[test]
fn generate_supports_operation_selection() {
    let db_path = unique_db_path();
    let connection = Connection::open(&db_path).expect("create sqlite database");
    connection
        .execute_batch(
            "CREATE TABLE users (
                id INTEGER PRIMARY KEY,
                email TEXT NOT NULL
            );",
        )
        .expect("create users table");
    drop(connection);

    Command::cargo_bin("dolog")
        .expect("build dolog binary")
        .args([
            "trigger",
            "generate",
            db_path.to_str().expect("utf8 path"),
            "--table",
            "users",
            "--operation",
            "insert",
            "--apply",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "Applied trigger SQL for table 'users'.",
        ));

    let connection = Connection::open(&db_path).expect("open sqlite database");
    assert_eq!(
        trigger_names(&connection),
        vec!["dolog_users_insert".to_owned()]
    );

    std::fs::remove_file(db_path).expect("remove temp db");
}

#[test]
fn generate_writes_sql_file_without_modifying_database() {
    let db_path = unique_db_path();
    let output_path = unique_sql_path();
    let connection = Connection::open(&db_path).expect("create sqlite database");
    connection
        .execute_batch(
            "CREATE TABLE users (
                id INTEGER PRIMARY KEY,
                email TEXT NOT NULL
            );",
        )
        .expect("create users table");
    drop(connection);

    Command::cargo_bin("dolog")
        .expect("build dolog binary")
        .args([
            "trigger",
            "generate",
            db_path.to_str().expect("utf8 path"),
            output_path.to_str().expect("utf8 path"),
            "--table",
            "users",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("Wrote trigger SQL to"));

    let sql = std::fs::read_to_string(&output_path).expect("read output file");
    assert!(sql.contains("CREATE TABLE IF NOT EXISTS \"_dolog_changes\""));
    assert!(sql.contains("CREATE TRIGGER \"dolog_users_insert\""));

    let connection = Connection::open(&db_path).expect("open sqlite database");
    assert!(!table_exists(&connection, "_dolog_changes"));
    assert!(trigger_names(&connection).is_empty());

    std::fs::remove_file(db_path).expect("remove temp db");
    std::fs::remove_file(output_path).expect("remove temp sql");
}

#[test]
fn generate_supports_repeated_table_flags() {
    let db_path = unique_db_path();
    let connection = Connection::open(&db_path).expect("create sqlite database");
    connection
        .execute_batch(
            "CREATE TABLE users (
                id INTEGER PRIMARY KEY,
                email TEXT NOT NULL
            );
            CREATE TABLE posts (
                id INTEGER PRIMARY KEY,
                title TEXT NOT NULL
            );",
        )
        .expect("create tables");
    drop(connection);

    Command::cargo_bin("dolog")
        .expect("build dolog binary")
        .args([
            "trigger",
            "generate",
            db_path.to_str().expect("utf8 path"),
            "--table",
            "users",
            "--table",
            "posts",
            "--apply",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "Applied trigger SQL for tables 'users', 'posts'.",
        ));

    let connection = Connection::open(&db_path).expect("open sqlite database");
    assert_eq!(trigger_names(&connection).len(), 6);

    std::fs::remove_file(db_path).expect("remove temp db");
}

#[test]
fn generate_all_tables_ignores_dolog_log_table() {
    let db_path = unique_db_path();
    let connection = Connection::open(&db_path).expect("create sqlite database");
    connection
        .execute_batch(
            "CREATE TABLE users (
                id INTEGER PRIMARY KEY,
                email TEXT NOT NULL
            );
            CREATE TABLE posts (
                id INTEGER PRIMARY KEY,
                title TEXT NOT NULL
            );
            CREATE TABLE _dolog_changes (
                id INTEGER PRIMARY KEY,
                table_name TEXT NOT NULL,
                operation TEXT NOT NULL,
                old_values TEXT,
                new_values TEXT,
                changed_at TEXT NOT NULL
            );",
        )
        .expect("create tables");
    drop(connection);

    Command::cargo_bin("dolog")
        .expect("build dolog binary")
        .args([
            "trigger",
            "generate",
            db_path.to_str().expect("utf8 path"),
            "--all-tables",
            "--apply",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "Applied trigger SQL for tables 'posts', 'users'.",
        ));

    let connection = Connection::open(&db_path).expect("open sqlite database");
    let names = trigger_names(&connection);
    assert_eq!(names.len(), 6);
    assert!(names.iter().all(|name| !name.contains("_dolog_changes")));

    std::fs::remove_file(db_path).expect("remove temp db");
}

#[test]
fn generate_drop_removes_selected_operations() {
    let db_path = unique_db_path();
    let connection = Connection::open(&db_path).expect("create sqlite database");
    connection
        .execute_batch(
            "CREATE TABLE users (
                id INTEGER PRIMARY KEY,
                email TEXT NOT NULL
            );",
        )
        .expect("create users table");
    drop(connection);

    Command::cargo_bin("dolog")
        .expect("build dolog binary")
        .args([
            "trigger",
            "generate",
            db_path.to_str().expect("utf8 path"),
            "--table",
            "users",
            "--apply",
        ])
        .assert()
        .success();

    Command::cargo_bin("dolog")
        .expect("build dolog binary")
        .args([
            "trigger",
            "generate",
            db_path.to_str().expect("utf8 path"),
            "--table",
            "users",
            "--drop",
            "--operation",
            "delete",
            "--apply",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "Applied trigger SQL for table 'users'.",
        ));

    let connection = Connection::open(&db_path).expect("open sqlite database");
    assert_eq!(
        trigger_names(&connection),
        vec![
            "dolog_users_insert".to_owned(),
            "dolog_users_update".to_owned()
        ]
    );

    std::fs::remove_file(db_path).expect("remove temp db");
}

#[test]
fn generate_rejects_sql_file_with_apply() {
    let db_path = unique_db_path();
    let output_path = unique_sql_path();
    let connection = Connection::open(&db_path).expect("create sqlite database");
    connection
        .execute_batch(
            "CREATE TABLE users (
                id INTEGER PRIMARY KEY,
                email TEXT NOT NULL
            );",
        )
        .expect("create users table");
    drop(connection);

    Command::cargo_bin("dolog")
        .expect("build dolog binary")
        .args([
            "trigger",
            "generate",
            db_path.to_str().expect("utf8 path"),
            output_path.to_str().expect("utf8 path"),
            "--table",
            "users",
            "--apply",
        ])
        .assert()
        .failure();

    std::fs::remove_file(db_path).expect("remove temp db");
}

#[test]
fn status_reports_operation_coverage() {
    let db_path = unique_db_path();
    let connection = Connection::open(&db_path).expect("create sqlite database");
    connection
        .execute_batch(
            "CREATE TABLE users (
                id INTEGER PRIMARY KEY,
                email TEXT NOT NULL
            );",
        )
        .expect("create users table");
    drop(connection);

    Command::cargo_bin("dolog")
        .expect("build dolog binary")
        .args([
            "trigger",
            "generate",
            db_path.to_str().expect("utf8 path"),
            "--table",
            "users",
            "--operation",
            "insert",
            "--operation",
            "delete",
            "--apply",
        ])
        .assert()
        .success();

    Command::cargo_bin("dolog")
        .expect("build dolog binary")
        .args([
            "trigger",
            "status",
            db_path.to_str().expect("utf8 path"),
            "--table",
            "users",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("Trigger status for"))
        .stdout(predicate::str::contains("TABLE"))
        .stdout(predicate::str::contains("users  yes"));

    std::fs::remove_file(db_path).expect("remove temp db");
}

#[test]
fn status_defaults_to_all_tables_without_flags() {
    let db_path = unique_db_path();
    let connection = Connection::open(&db_path).expect("create sqlite database");
    connection
        .execute_batch(
            "CREATE TABLE users (
                id INTEGER PRIMARY KEY,
                email TEXT NOT NULL
            );
            CREATE TABLE posts (
                id INTEGER PRIMARY KEY,
                title TEXT NOT NULL
            );",
        )
        .expect("create tables");
    drop(connection);

    Command::cargo_bin("dolog")
        .expect("build dolog binary")
        .args([
            "trigger",
            "generate",
            db_path.to_str().expect("utf8 path"),
            "--table",
            "users",
            "--operation",
            "insert",
            "--apply",
        ])
        .assert()
        .success();

    Command::cargo_bin("dolog")
        .expect("build dolog binary")
        .args(["trigger", "status", db_path.to_str().expect("utf8 path")])
        .assert()
        .success()
        .stdout(predicate::str::contains("Trigger status for"))
        .stdout(predicate::str::contains("posts"))
        .stdout(predicate::str::contains("users"));

    std::fs::remove_file(db_path).expect("remove temp db");
}

#[test]
fn log_export_writes_jsonl_and_deletes_exported_rows() {
    let db_path = unique_db_path();
    let output_path = unique_jsonl_path();
    let connection = Connection::open(&db_path).expect("create sqlite database");
    connection
        .execute_batch(
            "CREATE TABLE users (
                id INTEGER PRIMARY KEY,
                email TEXT NOT NULL
            );",
        )
        .expect("create users table");
    drop(connection);

    Command::cargo_bin("dolog")
        .expect("build dolog binary")
        .args([
            "trigger",
            "generate",
            db_path.to_str().expect("utf8 path"),
            "--table",
            "users",
            "--apply",
        ])
        .assert()
        .success();

    let connection = Connection::open(&db_path).expect("open sqlite database");
    connection
        .execute("INSERT INTO users (email) VALUES (?1)", ["ada@example.com"])
        .expect("insert user");
    connection
        .execute(
            "UPDATE users SET email = ?1 WHERE id = 1",
            ["ada+updated@example.com"],
        )
        .expect("update user");
    drop(connection);

    Command::cargo_bin("dolog")
        .expect("build dolog binary")
        .args([
            "log",
            "export",
            db_path.to_str().expect("utf8 path"),
            output_path.to_str().expect("utf8 path"),
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("Exported 2 change rows to"));

    let exported = std::fs::read_to_string(&output_path).expect("read exported file");
    assert!(exported.contains("\"table_name\":\"users\""));
    assert!(exported.contains("\"operation\":\"INSERT\""));
    assert!(exported.contains("\"operation\":\"UPDATE\""));

    let connection = Connection::open(&db_path).expect("open sqlite database");
    let remaining: i64 = connection
        .query_row("SELECT COUNT(*) FROM _dolog_changes", [], |row| row.get(0))
        .expect("count remaining logs");
    assert_eq!(remaining, 0);

    std::fs::remove_file(db_path).expect("remove temp db");
    std::fs::remove_file(output_path).expect("remove temp export");
}

#[test]
fn log_export_dry_run_does_not_require_output_or_delete_rows() {
    let db_path = unique_db_path();
    let connection = Connection::open(&db_path).expect("create sqlite database");
    connection
        .execute_batch(
            "CREATE TABLE users (
                id INTEGER PRIMARY KEY,
                email TEXT NOT NULL
            );",
        )
        .expect("create users table");
    drop(connection);

    Command::cargo_bin("dolog")
        .expect("build dolog binary")
        .args([
            "trigger",
            "generate",
            db_path.to_str().expect("utf8 path"),
            "--table",
            "users",
            "--apply",
        ])
        .assert()
        .success();

    let connection = Connection::open(&db_path).expect("open sqlite database");
    connection
        .execute("INSERT INTO users (email) VALUES (?1)", ["ada@example.com"])
        .expect("insert user");
    drop(connection);

    Command::cargo_bin("dolog")
        .expect("build dolog binary")
        .args([
            "log",
            "export",
            db_path.to_str().expect("utf8 path"),
            "--dry-run",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"table_name\":\"users\""))
        .stdout(predicate::str::contains("\"operation\":\"INSERT\""));

    let connection = Connection::open(&db_path).expect("open sqlite database");
    let remaining: i64 = connection
        .query_row("SELECT COUNT(*) FROM _dolog_changes", [], |row| row.get(0))
        .expect("count remaining logs");
    assert_eq!(remaining, 1);

    std::fs::remove_file(db_path).expect("remove temp db");
}

#[test]
fn log_status_reports_pending_rows_by_table_and_operation() {
    let db_path = unique_db_path();
    let connection = Connection::open(&db_path).expect("create sqlite database");
    connection
        .execute_batch(
            "CREATE TABLE users (
                id INTEGER PRIMARY KEY,
                email TEXT NOT NULL
            );",
        )
        .expect("create users table");
    drop(connection);

    Command::cargo_bin("dolog")
        .expect("build dolog binary")
        .args([
            "trigger",
            "generate",
            db_path.to_str().expect("utf8 path"),
            "--table",
            "users",
            "--apply",
        ])
        .assert()
        .success();

    let connection = Connection::open(&db_path).expect("open sqlite database");
    connection
        .execute("INSERT INTO users (email) VALUES (?1)", ["ada@example.com"])
        .expect("insert user");
    connection
        .execute(
            "UPDATE users SET email = ?1 WHERE id = 1",
            ["ada+updated@example.com"],
        )
        .expect("update user");
    drop(connection);

    Command::cargo_bin("dolog")
        .expect("build dolog binary")
        .args(["log", "status", db_path.to_str().expect("utf8 path")])
        .assert()
        .success()
        .stdout(predicate::str::contains("Pending log rows for"))
        .stdout(predicate::str::contains("TABLE"))
        .stdout(predicate::str::contains("users"))
        .stdout(predicate::str::contains("INSERT"))
        .stdout(predicate::str::contains("UPDATE"))
        .stdout(predicate::str::contains("TOTAL"));

    std::fs::remove_file(db_path).expect("remove temp db");
}

fn trigger_names(connection: &Connection) -> Vec<String> {
    let mut statement = connection
        .prepare("SELECT name FROM sqlite_master WHERE type = 'trigger' ORDER BY name")
        .expect("prepare trigger lookup");

    let rows = statement
        .query_map([], |row| row.get(0))
        .expect("query triggers");

    rows.map(|row| row.expect("row")).collect()
}

fn table_exists(connection: &Connection, table: &str) -> bool {
    connection
        .query_row(
            "SELECT EXISTS(
                SELECT 1
                FROM sqlite_master
                WHERE type = 'table' AND name = ?1
            )",
            [table],
            |row| row.get::<_, i64>(0),
        )
        .expect("query table existence")
        == 1
}

fn unique_db_path() -> std::path::PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock")
        .as_nanos();

    std::env::temp_dir().join(format!("dolog_cli_test_{nanos}.sqlite"))
}

fn unique_sql_path() -> std::path::PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock")
        .as_nanos();

    std::env::temp_dir().join(format!("dolog_cli_test_{nanos}.sql"))
}

fn unique_jsonl_path() -> std::path::PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock")
        .as_nanos();

    std::env::temp_dir().join(format!("dolog_cli_test_{nanos}.jsonl"))
}
