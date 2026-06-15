use crate::database::{
    adapter::DatabaseAdapter,
    config::ConnectionConfig,
    error::{DbError, DbResult},
    pool::ConnectionPool,
    types::{ConnectionStatus, QueryResult, QueryRow, QueryValue},
};
use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;

pub enum HttpSqlDialect {
    Trino,
    Presto,
}

pub struct HttpSqlPool {
    client: reqwest::Client,
    #[allow(dead_code)]
    base_url: String,
    #[allow(dead_code)]
    dialect: HttpSqlDialect,
    #[allow(dead_code)]
    username: String,
    #[allow(dead_code)]
    password: String,
}

#[async_trait]
impl ConnectionPool for HttpSqlPool {
    type Connection = reqwest::Client;

    async fn get_connection(&self) -> DbResult<Arc<Self::Connection>> {
        Ok(Arc::new(self.client.clone()))
    }

    async fn return_connection(&self, _conn: Arc<Self::Connection>) -> DbResult<()> {
        Ok(())
    }

    fn active_connections(&self) -> usize {
        0
    }
    fn idle_connections(&self) -> usize {
        0
    }
    fn max_connections(&self) -> usize {
        0
    }

    async fn close(&self) -> DbResult<()> {
        Ok(())
    }
    async fn health_check(&self) -> DbResult<()> {
        Ok(())
    }
}

pub struct HttpSqlAdapter {
    pub config: ConnectionConfig,
    client: Option<reqwest::Client>,
    pool: Option<Arc<HttpSqlPool>>,
}

impl HttpSqlAdapter {
    pub fn new(config: ConnectionConfig) -> Self {
        Self {
            config,
            client: None,
            pool: None,
        }
    }

    fn dialect(&self) -> HttpSqlDialect {
        match self.config.db_type {
            _ => HttpSqlDialect::Trino,
        }
    }

    fn base_url(&self) -> String {
        format!("http://{}:{}", self.config.host, self.config.port)
    }
}

#[async_trait]
impl DatabaseAdapter for HttpSqlAdapter {
    type Pool = HttpSqlPool;

    async fn connect(&mut self) -> DbResult<()> {
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .map_err(|e| DbError::Connection(e.to_string()))?;

        let resp = client
            .post(format!("{}/v1/statement", self.base_url()))
            .header("X-Trino-User", &self.config.username)
            .body("SELECT 1")
            .send()
            .await
            .map_err(|e| DbError::Connection(e.to_string()))?;

        if !resp.status().is_success() {
            return Err(DbError::Connection(format!(
                "Connection failed: HTTP {}",
                resp.status()
            )));
        }

        let pool = Arc::new(HttpSqlPool {
            client: client.clone(),
            base_url: self.base_url(),
            dialect: self.dialect(),
            username: self.config.username.clone(),
            password: self.config.password.clone().unwrap_or_default(),
        });

        self.client = Some(client);
        self.pool = Some(pool);
        Ok(())
    }

    async fn disconnect(&mut self) -> DbResult<()> {
        self.client = None;
        self.pool = None;
        Ok(())
    }

    async fn test_connection(&self) -> DbResult<ConnectionStatus> {
        let client = self
            .client
            .as_ref()
            .ok_or_else(|| DbError::Connection("Not connected".into()))?;

        let resp = client
            .post(format!("{}/v1/statement", self.base_url()))
            .header("X-Trino-User", &self.config.username)
            .body("SELECT 1")
            .send()
            .await
            .map_err(|e| DbError::Connection(e.to_string()))?;

        if !resp.status().is_success() {
            return Err(DbError::Connection("Connection test failed".into()));
        }

        Ok(ConnectionStatus {
            is_connected: true,
            server_version: None,
            current_database: None,
            current_user: None,
            metadata: HashMap::new(),
        })
    }

    async fn execute_query(&self, query: &str) -> DbResult<QueryResult> {
        let client = self
            .client
            .as_ref()
            .ok_or_else(|| DbError::Connection("Not connected".into()))?;
        let start = Instant::now();

        let resp = client
            .post(format!("{}/v1/statement", self.base_url()))
            .header("X-Trino-User", &self.config.username)
            .body(query.to_owned())
            .send()
            .await
            .map_err(|e| DbError::QueryExecution(e.to_string()))?;

        let exec_ms = start.elapsed().as_millis();
        let status = resp.status();

        if !status.is_success() {
            let body = resp.text().await.unwrap_or_default();
            return Err(DbError::QueryExecution(format!(
                "HTTP {}: {}",
                status.as_u16(),
                body
            )));
        }

        let body = resp
            .text()
            .await
            .map_err(|e| DbError::Serialization(e.to_string()))?;

        let json: serde_json::Value =
            serde_json::from_str(&body).map_err(|e| DbError::Serialization(e.to_string()))?;

        let columns: Vec<String> = json
            .get("columns")
            .and_then(|c| c.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|col| col.get("name").and_then(|n| n.as_str()).map(String::from))
                    .collect()
            })
            .unwrap_or_default();

        let data: Vec<Vec<serde_json::Value>> = json
            .get("data")
            .and_then(|d| d.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|row| row.as_array().map(|r| r.clone()))
                    .collect()
            })
            .unwrap_or_default();

        let rows: Vec<QueryRow> = data
            .into_iter()
            .map(|row| {
                let mut map = HashMap::new();
                for (i, val) in row.into_iter().enumerate() {
                    let col_name = columns
                        .get(i)
                        .cloned()
                        .unwrap_or_else(|| format!("col_{}", i));
                    let qv = match val {
                        serde_json::Value::Null => QueryValue::Null,
                        serde_json::Value::Bool(b) => QueryValue::Bool(b),
                        serde_json::Value::Number(n) => {
                            if let Some(i) = n.as_i64() {
                                QueryValue::Int(i)
                            } else if let Some(f) = n.as_f64() {
                                QueryValue::Float(f)
                            } else {
                                QueryValue::String(n.to_string())
                            }
                        }
                        serde_json::Value::String(s) => QueryValue::String(s),
                        other => QueryValue::String(other.to_string()),
                    };
                    map.insert(col_name, qv);
                }
                map
            })
            .collect();

        let mut result = QueryResult::new(columns);
        for row in rows {
            result.add_row(row);
        }
        result.execution_time_ms = Some(exec_ms as u64);
        Ok(result)
    }

    fn get_pool(&self) -> Option<Arc<Self::Pool>> {
        self.pool.clone()
    }

    fn get_config(&self) -> &ConnectionConfig {
        &self.config
    }
}
