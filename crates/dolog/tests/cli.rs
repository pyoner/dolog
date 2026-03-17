use std::time::{SystemTime, UNIX_EPOCH};

use assert_cmd::Command;
use predicates::prelude::*;
use rusqlite::Connection;

#[test]
fn create_dry_run_prints_sql_without_modifying_database() {
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
            "create",
            "--db",
            db_path.to_str().expect("utf8 path"),
            "--table",
            "users",
            "--dry-run",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "CREATE TABLE IF NOT EXISTS \"_dolog_changes\"",
        ))
        .stdout(predicate::str::contains(
            "CREATE TRIGGER \"dolog_users_insert\"",
        ));

    let connection = Connection::open(&db_path).expect("open sqlite database");
    assert!(!table_exists(&connection, "_dolog_changes"));
    assert!(trigger_names(&connection).is_empty());

    std::fs::remove_file(db_path).expect("remove temp db");
}

#[test]
fn create_output_writes_sql_file_without_modifying_database() {
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
            "create",
            "--db",
            db_path.to_str().expect("utf8 path"),
            "--table",
            "users",
            "--output",
            output_path.to_str().expect("utf8 path"),
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("Wrote SQL plan to"));

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
fn create_supports_repeated_table_flags() {
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
            "create",
            "--db",
            db_path.to_str().expect("utf8 path"),
            "--table",
            "users",
            "--table",
            "posts",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "Created triggers for tables 'users', 'posts'.",
        ));

    let connection = Connection::open(&db_path).expect("open sqlite database");
    assert_eq!(trigger_names(&connection).len(), 6);

    std::fs::remove_file(db_path).expect("remove temp db");
}

#[test]
fn create_all_tables_ignores_dolog_log_table() {
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
            "create",
            "--db",
            db_path.to_str().expect("utf8 path"),
            "--all-tables",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "Created triggers for tables 'posts', 'users'.",
        ));

    let connection = Connection::open(&db_path).expect("open sqlite database");
    let names = trigger_names(&connection);
    assert_eq!(names.len(), 6);
    assert!(names.iter().all(|name| !name.contains("_dolog_changes")));

    std::fs::remove_file(db_path).expect("remove temp db");
}

#[test]
fn list_reports_created_triggers_from_real_sqlite_database() {
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

    Command::cargo_bin("dolog")
        .expect("build dolog binary")
        .args([
            "trigger",
            "create",
            "--db",
            db_path.to_str().expect("utf8 path"),
            "--table",
            "users",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "Created triggers for table 'users'.",
        ));

    Command::cargo_bin("dolog")
        .expect("build dolog binary")
        .args([
            "trigger",
            "list",
            "--db",
            db_path.to_str().expect("utf8 path"),
            "--table",
            "users",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("dolog_users_insert (users)"))
        .stdout(predicate::str::contains("dolog_users_update (users)"))
        .stdout(predicate::str::contains("dolog_users_delete (users)"));

    assert_eq!(trigger_names(&connection).len(), 3);

    std::fs::remove_file(db_path).expect("remove temp db");
}

#[test]
fn preview_all_tables_ignores_dolog_log_table() {
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
            "preview",
            "create",
            "--db",
            db_path.to_str().expect("utf8 path"),
            "--all-tables",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "CREATE TRIGGER \"dolog_users_insert\"",
        ))
        .stdout(predicate::str::contains(
            "CREATE TRIGGER \"dolog_posts_insert\"",
        ))
        .stdout(predicate::str::contains("CREATE TRIGGER \"dolog__dolog_changes_").not());

    std::fs::remove_file(db_path).expect("remove temp db");
}

#[test]
fn preview_create_supports_repeated_table_flags() {
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
            "preview",
            "create",
            "--db",
            db_path.to_str().expect("utf8 path"),
            "--table",
            "users",
            "--table",
            "posts",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "CREATE TRIGGER \"dolog_users_insert\"",
        ))
        .stdout(predicate::str::contains(
            "CREATE TRIGGER \"dolog_posts_insert\"",
        ));

    std::fs::remove_file(db_path).expect("remove temp db");
}

#[test]
fn preview_delete_matches_trigger_drop_sql() {
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
            "preview",
            "delete",
            "--db",
            db_path.to_str().expect("utf8 path"),
            "--table",
            "users",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "DROP TRIGGER IF EXISTS \"dolog_users_insert\";",
        ))
        .stdout(predicate::str::contains(
            "DROP TRIGGER IF EXISTS \"dolog_users_update\";",
        ))
        .stdout(predicate::str::contains(
            "DROP TRIGGER IF EXISTS \"dolog_users_delete\";",
        ));

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
