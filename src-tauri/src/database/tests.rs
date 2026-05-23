//! Tests for the database adapter module.

#[cfg(test)]
#[allow(clippy::module_inception)]
mod tests {
    use crate::database::*;

    #[test]
    fn test_connection_config_builder() {
        let config = ConnectionConfig::new(DatabaseType::PostgreSQL, "localhost", 5432, "testuser")
            .with_database("testdb")
            .with_password("testpass")
            .with_ssl_mode(SslMode::Require);

        assert_eq!(config.db_type, DatabaseType::PostgreSQL);
        assert_eq!(config.host, "localhost");
        assert_eq!(config.port, 5432);
        assert_eq!(config.username, "testuser");
        assert_eq!(config.database, Some("testdb".to_string()));
        assert_eq!(config.password, Some("testpass".to_string()));
        assert_eq!(config.ssl_mode, SslMode::Require);
    }

    #[test]
    fn test_pool_config_default() {
        let config = PoolConfig::default();
        assert_eq!(config.min_connections, 1);
        assert_eq!(config.max_connections, 10);
    }

    #[test]
    fn test_query_result_new() {
        let columns = vec!["id".to_string(), "name".to_string()];
        let result = QueryResult::new(columns.clone());
        assert_eq!(result.columns, columns);
        assert_eq!(result.rows.len(), 0);
        assert_eq!(result.rows_affected, None);
    }

    #[test]
    fn test_query_result_affected() {
        let result = QueryResult::affected(5);
        assert_eq!(result.rows_affected, Some(5));
        assert_eq!(result.columns.len(), 0);
        assert_eq!(result.rows.len(), 0);
    }

    #[test]
    fn test_db_error_creation() {
        let error = DbError::Connection("Failed to connect".to_string());
        assert_eq!(error.to_string(), "Connection error: Failed to connect");

        let error = DbError::new("Custom error");
        assert!(error.to_string().contains("Custom error"));
    }

    #[test]
    fn test_pool_stats_creation() {
        let stats = PoolStats::new(10);
        assert_eq!(stats.max_connections, 10);
        assert_eq!(stats.active_connections, 0);
        assert_eq!(stats.idle_connections, 0);
    }
}
