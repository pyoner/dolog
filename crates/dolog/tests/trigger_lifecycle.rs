use std::time::{SystemTime, UNIX_EPOCH};

use dolog::trigger::TriggerManager;
use rusqlite::Connection;

#[test]
fn manages_trigger_lifecycle_for_a_table() {
    let db_path = unique_db_path();
    let mut connection = Connection::open(&db_path).expect("create sqlite database");

    connection
        .execute_batch(
            "CREATE TABLE users (
                id INTEGER PRIMARY KEY,
                email TEXT NOT NULL,
                active INTEGER NOT NULL DEFAULT 1
            );",
        )
        .expect("create users table");

    let manager = TriggerManager::new("_dolog_changes".to_owned(), "dolog".to_owned());

    let preview_create = manager
        .preview_create(&connection, "users")
        .expect("preview create");
    assert_eq!(preview_create.len(), 4);
    assert!(preview_create[0].contains("CREATE TABLE IF NOT EXISTS \"_dolog_changes\""));
    assert!(preview_create[1].contains("CREATE TRIGGER \"dolog_users_insert\""));

    manager
        .create(&mut connection, "users")
        .expect("create triggers");
    assert_eq!(
        trigger_names(&connection),
        vec![
            "dolog_users_delete".to_owned(),
            "dolog_users_insert".to_owned(),
            "dolog_users_update".to_owned()
        ]
    );
    let listed = manager
        .list_triggers(&connection, Some("users"))
        .expect("list triggers");
    assert_eq!(listed.len(), 3);
    assert_eq!(listed[0].name, "dolog_users_delete");

    connection
        .execute("ALTER TABLE users ADD COLUMN name TEXT", [])
        .expect("alter users table");

    let preview_update = manager
        .preview_update(&connection, "users")
        .expect("preview update");
    assert_eq!(preview_update.len(), 7);
    assert!(preview_update[1].contains("DROP TRIGGER IF EXISTS \"dolog_users_insert\";"));
    assert!(preview_update[6].contains("CREATE TRIGGER \"dolog_users_delete\""));

    manager
        .update(&mut connection, "users")
        .expect("update triggers");
    connection
        .execute(
            "INSERT INTO users (email, active, name) VALUES (?1, ?2, ?3)",
            ("a@example.com", 1, "Ada"),
        )
        .expect("insert user");

    let new_values: String = connection
        .query_row(
            "SELECT new_values FROM _dolog_changes WHERE operation = 'INSERT' LIMIT 1",
            [],
            |row| row.get(0),
        )
        .expect("read inserted log row");
    assert!(new_values.contains("\"name\":\"Ada\""));

    let preview_delete = manager
        .preview_delete(&connection, "users")
        .expect("preview delete");
    assert_eq!(preview_delete.len(), 3);
    assert!(preview_delete[0].contains("DROP TRIGGER IF EXISTS \"dolog_users_insert\";"));

    manager
        .delete(&mut connection, "users")
        .expect("delete triggers");
    assert!(trigger_names(&connection).is_empty());

    std::fs::remove_file(db_path).expect("remove temp db");
}

#[test]
fn planning_and_dry_run_do_not_modify_real_sqlite_db() {
    let db_path = unique_db_path();
    let mut connection = Connection::open(&db_path).expect("create sqlite database");

    connection
        .execute_batch(
            "CREATE TABLE users (
                id INTEGER PRIMARY KEY,
                email TEXT NOT NULL
            );",
        )
        .expect("create users table");

    let manager = TriggerManager::new("_dolog_changes".to_owned(), "dolog".to_owned());

    let create_plan = manager
        .plan_create(&connection, "users", &dolog::trigger::Operation::all())
        .expect("plan create");
    assert_eq!(create_plan.statements().len(), 4);
    assert!(trigger_names(&connection).is_empty());
    assert!(!table_exists(&connection, "_dolog_changes"));

    let update_plan = manager
        .plan_update(&connection, "users", &dolog::trigger::Operation::all())
        .expect("plan update");
    assert_eq!(update_plan.statements().len(), 7);
    assert!(trigger_names(&connection).is_empty());
    assert!(!table_exists(&connection, "_dolog_changes"));

    manager
        .apply_plan(&mut connection, &create_plan)
        .expect("apply create plan");
    assert_eq!(trigger_names(&connection).len(), 3);
    assert!(table_exists(&connection, "_dolog_changes"));

    let delete_plan = manager
        .plan_delete(&connection, "users", &dolog::trigger::Operation::all())
        .expect("plan delete");
    assert_eq!(delete_plan.statements().len(), 3);

    manager
        .apply_plan(&mut connection, &delete_plan)
        .expect("apply delete plan");
    assert!(trigger_names(&connection).is_empty());

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

    std::env::temp_dir().join(format!("dolog_test_{nanos}.sqlite"))
}
