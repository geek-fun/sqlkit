//! Run against disposable PostgreSQL/MySQL instances using their standard test environment variables.
use sqlkit_lib::connection::handle::ConnectionHandle;
use sqlkit_lib::database::QueryValue;
use sqlkit_lib::state::{AppState, ServerConfig};
use std::time::Duration;

async fn check_idle_actions(db_type: &str, prefix: &str, default_port: u16) {
    let value = |suffix: &str, default: &str| {
        std::env::var(format!("{prefix}_{suffix}")).unwrap_or_else(|_| default.into())
    };
    let mut config = ServerConfig::new(
        "idle recovery".into(),
        db_type.into(),
        value("HOST", "127.0.0.1"),
        value("PORT", &default_port.to_string()).parse().unwrap(),
        value(
            "USER",
            if prefix == "POSTGRES" {
                "postgres"
            } else {
                "root"
            },
        ),
    );
    config.id = "recovery".into();
    config.password = Some(value("PASSWORD", ""));
    config.database = Some(value("DB", "sqlkit_recovery_test"));
    config.ssl_mode = Some("disable".into());
    let admin = sqlkit_lib::commands::helpers::create_and_connect_adapter(
        db_type,
        config.to_connection_config().unwrap(),
    )
    .await
    .unwrap();
    let state = AppState::new();
    state
        .configs
        .write()
        .await
        .insert(config.id.clone(), config.clone());
    let connection = state.ensure_connection("recovery").await.unwrap();
    connection
        .execute_query("DROP TABLE IF EXISTS idle_recovery_items")
        .await
        .unwrap();
    connection
        .execute_query(
            "CREATE TABLE idle_recovery_items (id INTEGER PRIMARY KEY, label VARCHAR(50))",
        )
        .await
        .unwrap();

    for (index, sql) in [
        "SELECT * FROM idle_recovery_items",
        "INSERT INTO idle_recovery_items (id, label) VALUES (1, 'once')",
        "ALTER TABLE idle_recovery_items ADD COLUMN value INTEGER",
        "UPDATE idle_recovery_items SET label = 'updated' WHERE id = 1",
        "DELETE FROM idle_recovery_items WHERE id = 1",
    ]
    .iter()
    .enumerate()
    {
        let old = state.ensure_connection("recovery").await.unwrap();
        let session_query = if prefix == "POSTGRES" {
            "SELECT pg_backend_pid() AS session_id"
        } else {
            "SELECT CONNECTION_ID() AS session_id"
        };
        let session = old.execute_query(session_query).await.unwrap();
        let QueryValue::Int(id) = session.rows[0]["session_id"] else {
            panic!("Expected a numeric session ID")
        };
        let terminate = if prefix == "POSTGRES" {
            format!("SELECT pg_terminate_backend({id})")
        } else {
            format!("KILL CONNECTION {id}")
        };
        admin.execute_query(&terminate).await.unwrap();
        tokio::time::sleep(Duration::from_millis(1100)).await;

        let ready = state.ensure_connection("recovery").await.unwrap();
        if index == 0 {
            let schema = if prefix == "POSTGRES" {
                Some("public")
            } else {
                None
            };
            let tables = ready
                .list_tables(config.database.as_deref(), schema)
                .await
                .unwrap();
            assert!(tables
                .iter()
                .any(|table| table.name == "idle_recovery_items"));
        }
        ready.execute_query(sql).await.unwrap();
        if index == 1 {
            let count = ready
                .execute_query("SELECT COUNT(*) AS count FROM idle_recovery_items")
                .await
                .unwrap();
            assert_eq!(count.rows[0]["count"], QueryValue::Int(1));
        }
    }
    let ready = state.ensure_connection("recovery").await.unwrap();
    ready
        .execute_query("DROP TABLE idle_recovery_items")
        .await
        .unwrap();
    ready.disconnect().await.unwrap();
    admin.disconnect().await.unwrap();
}

#[tokio::test]
#[ignore] // Requires PostgreSQL
async fn postgres_idle_actions_reconnect_before_metadata_sql_edits_and_ddl() {
    check_idle_actions("PostgreSQL", "POSTGRES", 5432).await;
}

#[tokio::test]
#[ignore] // Requires MySQL
async fn mysql_idle_actions_reconnect_before_metadata_sql_edits_and_ddl() {
    check_idle_actions("MySQL", "MYSQL", 3306).await;
}
