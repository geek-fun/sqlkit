use super::cache::PoolKey;
use super::handle::ConnectionHandle;
use super::read::tests::fixture;
use crate::database::QueryValue;
use crate::state::{AppState, ServerConfig};
use serde_json::json;
use wiremock::{matchers::method, Mock, MockServer, ResponseTemplate};

#[tokio::test]
async fn idle_session_recovers_before_sql_writes_and_metadata_actions() {
    let (_directory, state, stale) = fixture().await;
    stale.disconnect().await.unwrap();
    state.cache.expire_validation("test").await;
    let connection = state.ensure_connection("test").await.unwrap();
    assert!(!connection.same_session(&stale));
    connection
        .execute_query("INSERT INTO items VALUES (4)")
        .await
        .unwrap();
    let count = connection
        .execute_query("SELECT COUNT(*) AS count FROM items")
        .await
        .unwrap();
    assert_eq!(count.rows[0]["count"], QueryValue::Int(4));

    connection.disconnect().await.unwrap();
    state.cache.expire_validation("test").await;
    let connection = state.ensure_connection("test").await.unwrap();
    assert!(connection
        .list_tables(None, None)
        .await
        .unwrap()
        .iter()
        .any(|table| table.name == "items"));
    assert_eq!(
        connection
            .list_columns(None, None, "items")
            .await
            .unwrap()
            .len(),
        1
    );
    connection
        .execute_query("ALTER TABLE items ADD COLUMN label TEXT")
        .await
        .unwrap();
    connection
        .execute_query("UPDATE items SET label = 'updated' WHERE id = 4")
        .await
        .unwrap();
    connection
        .execute_query("DELETE FROM items WHERE id = 4")
        .await
        .unwrap();
}

#[tokio::test]
async fn cached_handles_cannot_bypass_validation() {
    let (_directory, state, stale) = fixture().await;
    let key = PoolKey::new("test", Some("sample_database"));
    state.cache.get_or_create(&key, &state).await.unwrap();
    stale.disconnect().await.unwrap();
    state.cache.expire_validation("test").await;
    let connection = state.cache.get_or_create(&key, &state).await.unwrap();
    assert!(!connection.same_session(&stale));
    assert!(connection.test_connection().await.unwrap().is_connected);
}

#[tokio::test]
async fn concurrent_idle_actions_receive_the_same_reconnected_session() {
    let (_directory, state, stale) = fixture().await;
    stale.disconnect().await.unwrap();
    state.cache.expire_validation("test").await;
    let (first, second) = tokio::join!(
        state.ensure_connection("test"),
        state.ensure_connection("test")
    );
    let (first, second) = (first.unwrap(), second.unwrap());
    assert!(first.same_session(&second));
    assert!(!first.same_session(&stale));
}

#[tokio::test]
async fn failed_idle_reconnect_can_recover_on_a_later_action() {
    let (_directory, state, stale) = fixture().await;
    stale.disconnect().await.unwrap();
    state.cache.expire_validation("test").await;
    state.configs.write().await.get_mut("test").unwrap().db_type = "unsupported".into();
    assert!(state.ensure_connection("test").await.is_err());
    assert!(!state.connections.read().await.contains_key("test"));
    state.configs.write().await.get_mut("test").unwrap().db_type = "sqlite".into();
    assert!(
        state
            .ensure_connection("test")
            .await
            .unwrap()
            .test_connection()
            .await
            .unwrap()
            .is_connected
    );
}

#[tokio::test]
async fn http_adapter_families_share_the_idle_reconnect_policy() {
    for db_type in ["ClickHouse", "Trino", "Presto", "RQLite"] {
        let server = MockServer::start().await;
        let body = match db_type {
            "ClickHouse" => {
                json!({"meta":[{"name":"one","type":"UInt8"}],"data":[{"one":1}],"rows":1})
            }
            "RQLite" => json!({"results":[{"columns":["one"],"types":["integer"],"values":[[1]]}]}),
            _ => json!({"columns":[{"name":"one"}],"data":[[1]]}),
        };
        Mock::given(method("POST"))
            .respond_with(ResponseTemplate::new(200).set_body_json(body))
            .mount(&server)
            .await;
        let state = AppState::new();
        let mut config = ServerConfig::new(
            "test".into(),
            db_type.into(),
            "127.0.0.1".into(),
            server.address().port(),
            "test".into(),
        );
        config.id = "test".into();
        config.ssl_mode = Some("disable".into());
        state.configs.write().await.insert("test".into(), config);
        let stale = state.ensure_connection("test").await.unwrap();
        stale.disconnect().await.unwrap();
        state.cache.expire_validation("test").await;
        let connection = state.ensure_connection("test").await.unwrap();
        assert!(!connection.same_session(&stale), "{db_type}");
        assert_eq!(
            connection
                .execute_query("SELECT 1 AS one")
                .await
                .unwrap()
                .rows
                .len(),
            1,
            "{db_type}"
        );
    }
}

#[tokio::test]
async fn authentication_errors_are_reported_without_recreating_the_session() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({"columns":[],"data":[]})))
        .mount(&server)
        .await;
    let state = AppState::new();
    let mut config = ServerConfig::new(
        "test".into(),
        "Trino".into(),
        "127.0.0.1".into(),
        server.address().port(),
        "test".into(),
    );
    config.id = "test".into();
    config.ssl_mode = Some("disable".into());
    state.configs.write().await.insert("test".into(), config);
    let original = state.ensure_connection("test").await.unwrap();
    state.cache.expire_validation("test").await;
    server.reset().await;
    Mock::given(method("POST"))
        .respond_with(ResponseTemplate::new(401))
        .expect(1)
        .mount(&server)
        .await;
    let error = state.ensure_connection("test").await.err().unwrap();
    assert!(error.contains("Authentication"));
    assert!(state.connections.read().await["test"].same_session(&original));
}

#[tokio::test]
async fn turso_closed_sessions_take_the_shared_replacement_path() {
    let state = AppState::new();
    let config = ServerConfig::new(
        "test".into(),
        "Turso".into(),
        "test.invalid".into(),
        443,
        "test".into(),
    );
    let adapter = crate::database::TursoAdapter::new(config.to_connection_config().unwrap());
    let connection = crate::state::ActiveConnection::Turso(std::sync::Arc::new(
        tokio::sync::Mutex::new(adapter),
    ));
    state
        .connections
        .write()
        .await
        .insert("test".into(), connection);
    // An invalid replacement configuration makes the attempted recovery observable without a remote service.
    let mut replacement = config;
    replacement.db_type = "unsupported".into();
    state
        .configs
        .write()
        .await
        .insert("test".into(), replacement);
    assert!(state
        .ensure_connection("test")
        .await
        .err()
        .unwrap()
        .contains("Unsupported database type"));
    assert!(!state.connections.read().await.contains_key("test"));
}
