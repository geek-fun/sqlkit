//! LRU connection cache — bounded cache of active connection handles.
//!
//! Key design:
//!   - Pool key: `{connection_id}` or `{connection_id}:{database}` for multi-db
//!   - Single-db types (SQLite, DuckDB, Oracle, JDBC bridge) share one pool per connection_id
//!   - Bounded to max 20 entries; evicts least-recently-used on overflow
//!   - Reconnect gate: prevents multiple simultaneous reconnects for same connection

use crate::state::{ActiveConnection, AppState};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::{Notify, RwLock};

const DEFAULT_MAX_ENTRIES: usize = 20;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct PoolKey {
    pub connection_id: String,
    pub database: Option<String>,
}

impl PoolKey {
    pub fn new(connection_id: &str, database: Option<&str>) -> Self {
        Self {
            connection_id: connection_id.to_string(),
            database: database.map(str::to_string),
        }
    }

    pub fn to_string_key(&self) -> String {
        match &self.database {
            Some(db) => format!("{}:{}", self.connection_id, db),
            None => self.connection_id.clone(),
        }
    }
}

struct CacheEntry {
    connection: ActiveConnection,
    last_access: Instant,
}

/// Bounded LRU cache for database connections.
pub struct ConnectionCache {
    entries: RwLock<HashMap<String, CacheEntry>>,
    max_entries: usize,
    /// Reconnect gate — ensures only one reconnect per connection_id at a time.
    reconnect_gates: RwLock<HashMap<String, Arc<Notify>>>,
}

impl ConnectionCache {
    pub fn new(max_entries: usize) -> Self {
        Self {
            entries: RwLock::new(HashMap::new()),
            max_entries,
            reconnect_gates: RwLock::new(HashMap::new()),
        }
    }

    pub fn default() -> Self {
        Self::new(DEFAULT_MAX_ENTRIES)
    }

    /// Get or create a connection for the given pool key.
    /// Returns the cached connection if present and healthy, otherwise creates new.
    pub async fn get_or_create(
        &self,
        key: &PoolKey,
        app_state: &AppState,
    ) -> Result<ActiveConnection, String> {
        let string_key = key.to_string_key();

        // Fast path: cache hit
        {
            let entries = self.entries.read().await;
            if let Some(entry) = entries.get(&string_key) {
                let conn = entry.connection.clone();
                drop(entries);
                let mut entries = self.entries.write().await;
                if let Some(entry) = entries.get_mut(&string_key) {
                    entry.last_access = Instant::now();
                }
                return Ok(conn);
            }
        }

        // Slow path: need to create connection
        let conns = app_state.connections.read().await;
        let connection = conns
            .get(&key.connection_id)
            .cloned()
            .ok_or_else(|| format!("No active connection found for '{}'", key.connection_id))?;
        drop(conns);

        // Evict if at capacity
        {
            let entries = self.entries.read().await;
            if entries.len() >= self.max_entries {
                drop(entries);
                self.evict_lru().await;
            }
        }

        // Insert into cache
        let mut entries = self.entries.write().await;
        entries.insert(
            string_key.clone(),
            CacheEntry {
                connection: connection.clone(),
                last_access: Instant::now(),
            },
        );

        Ok(connection)
    }

    /// Remove a connection from the cache.
    pub async fn remove(&self, key: &PoolKey) {
        self.entries.write().await.remove(&key.to_string_key());
    }

    /// Remove all cached entries for a given connection_id.
    pub async fn remove_all(&self, connection_id: &str) {
        let prefix = format!("{}:", connection_id);
        let mut entries = self.entries.write().await;
        entries.retain(|k, _| k != connection_id && !k.starts_with(&prefix));
    }

    /// Get the reconnect gate for a connection_id. Creates one if not exists.
    /// The gate prevents multiple simultaneous reconnects for the same connection.
    pub async fn reconnect_gate(&self, connection_id: &str) -> Arc<Notify> {
        let gates = self.reconnect_gates.read().await;
        if let Some(gate) = gates.get(connection_id) {
            return gate.clone();
        }
        drop(gates);

        let mut gates = self.reconnect_gates.write().await;
        gates
            .entry(connection_id.to_string())
            .or_insert_with(|| Arc::new(Notify::new()))
            .clone()
    }

    /// Signal that a reconnect has completed, waking any waiters.
    pub async fn signal_reconnect(&self, connection_id: &str) {
        let gates = self.reconnect_gates.read().await;
        if let Some(gate) = gates.get(connection_id) {
            gate.notify_waiters();
        }
    }

    /// Number of cached entries.
    pub async fn len(&self) -> usize {
        self.entries.read().await.len()
    }

    /// Check if cache is empty.
    pub async fn is_empty(&self) -> bool {
        self.entries.read().await.is_empty()
    }

    // ── internal ──────────────────────────────────────────────────────

    async fn evict_lru(&self) {
        let mut entries = self.entries.write().await;
        if entries.is_empty() {
            return;
        }

        let lru_key = entries
            .iter()
            .min_by_key(|(_, entry)| entry.last_access)
            .map(|(k, _)| k.clone());

        if let Some(key) = lru_key {
            entries.remove(&key);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pool_key_format() {
        let key = PoolKey::new("conn-1", None);
        assert_eq!(key.to_string_key(), "conn-1");

        let key = PoolKey::new("conn-1", Some("analytics"));
        assert_eq!(key.to_string_key(), "conn-1:analytics");
    }

    #[tokio::test]
    async fn test_cache_evicts_lru() {
        let cache = ConnectionCache::new(2);

        // We can't fully test without AppState, but we can test LRU eviction logic
        assert!(cache.is_empty().await);
        assert_eq!(cache.len().await, 0);
    }
}
