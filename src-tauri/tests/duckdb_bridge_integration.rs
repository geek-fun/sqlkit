//! Integration tests for the shared JDBC bridge process with DuckDB.
//!
//! These tests exercise the real Java bridge, so they require:
//! - Java 25+ on the machine (the managed JRE the app downloads also counts)
//! - the bridge JAR for the current app version in `~/.sqlkit/jdbc-bridge/`
//! - the DuckDB JDBC driver in `~/.sqlkit/jdbc-bridge/drivers/duckdb_jdbc/`
//!
//! Run with:
//! `cargo test --test duckdb_bridge_integration -- --ignored --nocapture`

use sqlkit_lib::database::{ConnectionConfig, DatabaseAdapter, DatabaseType, JdbcBridgeAdapter};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

/// DuckDB file-based connection, mirroring what `ServerConfig` produces:
/// the JDBC URL template takes the path (and URL options) from `database`.
fn file_config(path: &Path, read_only: bool) -> ConnectionConfig {
    let value = if read_only {
        format!("{};access_mode=READ_ONLY", path.display())
    } else {
        path.to_string_lossy().to_string()
    };
    ConnectionConfig::new(DatabaseType::DuckDb, value.clone(), 0, "").with_database(value)
}

/// Named in-memory DuckDB instance — the value the connection form stores.
fn memory_config(label: &str) -> ConnectionConfig {
    let value = format!("memory:{}", label);
    ConnectionConfig::new(DatabaseType::DuckDb, value.clone(), 0, "").with_database(value)
}

/// Second connection must reuse the bridge JVM: the first connection in a fresh
/// process maps DuckDB's ~107 MB native library (~1.4 s measured), which would
/// otherwise be paid again by every "Test connection" click.
#[tokio::test]
#[ignore]
async fn second_connection_reuses_the_bridge_process() {
    let mut first = JdbcBridgeAdapter::new(memory_config("sqlkit_integration_a"));
    let started = Instant::now();
    first.connect().await.expect("first connect");
    let first_elapsed = started.elapsed();
    first.disconnect().await.expect("first disconnect");

    // Warm connects: the JVM, the driver's native library and the (once per
    // session) Adoptium update check are already paid for.
    let mut warm_total = Duration::ZERO;
    const WARM_CONNECTS: u32 = 5;
    for index in 0..WARM_CONNECTS {
        let mut adapter = JdbcBridgeAdapter::new(memory_config(&format!("sqlkit_warm_{}", index)));
        let started = Instant::now();
        adapter.connect().await.expect("warm connect");
        warm_total += started.elapsed();
        adapter.disconnect().await.expect("warm disconnect");
    }
    let warm_average = warm_total / WARM_CONNECTS;

    println!(
        "cold connect: {:?}, warm average over {} connects: {:?}",
        first_elapsed, WARM_CONNECTS, warm_average
    );

    // Before the bridge process was shared, every connect started a JVM and
    // re-mapped DuckDB's native library: ~4.3 s each on this machine. With the
    // shared process (and a bridge that caches driver class loaders) it is ~5 ms,
    // so a 2.5 s average still catches a regression to the old behaviour.
    assert!(
        warm_average < Duration::from_millis(2500),
        "warm connects should reuse the bridge process: cold={:?} warm average={:?}",
        first_elapsed,
        warm_average
    );
}

/// Disconnecting must really close the pool inside the shared bridge: opening
/// the same file read-only afterwards is rejected by DuckDB when a read-write
/// instance is still alive ("same database file with a different configuration").
///
/// Read-only URLs also need a bridge build that sets HikariCP's read-only flag
/// from the URL (`ConnectionManager.isReadOnlyUrl`); with an older bridge the
/// driver rejects HikariCP's `setReadOnly(false)` call.
#[tokio::test]
#[ignore]
async fn disconnect_closes_the_pool_and_releases_the_file() {
    let dir = tempfile::tempdir().expect("temp dir");
    let path: PathBuf = dir.path().join("shared.duckdb");

    let mut writer = JdbcBridgeAdapter::new(file_config(&path, false));
    writer.connect().await.expect("read-write connect");
    writer
        .execute_query("CREATE TABLE t AS SELECT 1 AS x")
        .await
        .expect("create table");
    writer.disconnect().await.expect("read-write disconnect");

    let mut reader = JdbcBridgeAdapter::new(file_config(&path, true));
    if let Err(error) = reader.connect().await {
        let message = error.to_string();
        if message.contains("change read-only status") {
            println!(
                "SKIPPED: the installed bridge predates read-only URL support; \
                 rebuild the bridge (jdbc-bridge/) to exercise this test. Error: {}",
                message
            );
            return;
        }
        panic!(
            "read-only connect after disconnect failed (the pool must be closed): {}",
            message
        );
    }
    let result = reader
        .execute_query("SELECT count(*) AS n FROM t")
        .await
        .expect("read the table through the read-only connection");
    assert_eq!(result.rows.len(), 1);
    reader.disconnect().await.expect("read-only disconnect");

    assert!(path.exists(), "the database file should exist");
}

/// Reconnecting read-write to the same file after a disconnect keeps working,
/// which is what "Test connection" followed by "Save & Connect" does.
#[tokio::test]
#[ignore]
async fn repeated_connect_and_disconnect_on_one_file() {
    let dir = tempfile::tempdir().expect("temp dir");
    let path = dir.path().join("reopen.duckdb");

    for attempt in 1..=3 {
        let mut adapter = JdbcBridgeAdapter::new(file_config(&path, false));
        adapter
            .connect()
            .await
            .unwrap_or_else(|e| panic!("connect #{} failed: {}", attempt, e));
        adapter.disconnect().await.expect("disconnect");
    }

    let started = Instant::now();

    let mut adapter = JdbcBridgeAdapter::new(file_config(&path, false));
    adapter.connect().await.expect("final connect");
    let status = adapter.test_connection().await.expect("test connection");
    assert!(status.is_connected);
    adapter.disconnect().await.expect("final disconnect");

    println!("reconnect after three cycles: {:?}", started.elapsed());
}
