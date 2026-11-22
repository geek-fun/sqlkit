//! Connection Guardian — single background task that monitors connection health,
//! auto-reconnects dead connections, evicts idle connections, and broadcasts
//! state changes to the frontend via Tauri events.
//!
//! Design:
//!   - ONE tokio task, not N tasks (avoids resource proliferation)
//!   - Priority queue by (error_count desc, last_access asc)
//!   - Health state machine: Healthy -> Degraded -> Dead -> Reconnecting -> Healthy
//!   - Exponential backoff on reconnect (1s, 2s, 4s, ... max 30s)
//!   - Idle eviction after configurable TTL (default 30 min)
//!   - Emits `connection-state-changed` event for reactive frontend updates

use crate::connection::handle::ConnectionHandle;
use crate::database::DbError;
use crate::state::AppState;
use crate::APP_HANDLE;
use serde::Serialize;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tauri::{Emitter, Manager};
use tokio::sync::RwLock;
use tokio::time::MissedTickBehavior;

const GUARDIAN_INTERVAL_SECS: u64 = 5;
const DEGRADED_THRESHOLD: u32 = 2;
const DEAD_THRESHOLD: u32 = 3;
const RECONNECT_BASE_DELAY_SECS: u64 = 1;
const RECONNECT_MAX_DELAY_SECS: u64 = 30;
const DEFAULT_IDLE_EVICTION_SECS: u64 = 1800; // 30 min

// ── Types ────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum HealthState {
    Healthy,
    Degraded,
    Dead,
    Reconnecting,
}

#[derive(Debug, Clone, Serialize)]
pub struct ConnectionStateEvent {
    pub connection_id: String,
    pub state: HealthState,
    pub error: Option<String>,
}

/// Quality assessment for a single connection, exposed via Tauri command
/// and used by agent capabilities to warn about flaky connections.
#[derive(Debug, Clone, Serialize)]
pub struct ConnectionQuality {
    pub connection_id: String,
    pub state: HealthState,
    pub error_count: u32,
    pub total_queries: u64,
    pub avg_latency_ms: f64,
    pub uptime_pct: f64,
    pub score: f64,
}

#[derive(Debug)]
struct ConnectionHealth {
    state: HealthState,
    error_count: u32,
    last_healthy: Instant,
    last_access: Instant,
    reconnect_attempt: u32,
    next_reconnect_at: Option<Instant>,
    total_queries: u64,
    total_latency_ms: f64,
}

impl Default for ConnectionHealth {
    fn default() -> Self {
        let now = Instant::now();
        Self {
            state: HealthState::Healthy,
            error_count: 0,
            last_healthy: now,
            last_access: now,
            reconnect_attempt: 0,
            next_reconnect_at: None,
            total_queries: 0,
            total_latency_ms: 0.0,
        }
    }
}

// ── Guardian ──────────────────────────────────────────────────────────

pub struct ConnectionGuardian {
    health: Arc<RwLock<HashMap<String, ConnectionHealth>>>,
    idle_eviction_secs: u64,
}

impl ConnectionGuardian {
    pub fn new() -> Self {
        Self {
            health: Arc::new(RwLock::new(HashMap::new())),
            idle_eviction_secs: DEFAULT_IDLE_EVICTION_SECS,
        }
    }

    /// Start the guardian background task in the given runtime.
    /// The caller is responsible for spawning via `tauri::async_runtime::spawn`.
    pub async fn run(self: Arc<Self>) {
        self.run_loop().await;
    }

    /// Mark a connection as accessed by a user operation.
    pub async fn touch(&self, connection_id: &str) {
        let mut health_map = self.health.write().await;
        if let Some(h) = health_map.get_mut(connection_id) {
            h.last_access = Instant::now();
        }
    }

    /// Mark a connection as healthy after a successful operation.
    /// `latency_ms` is optional — when provided it updates query latency tracking
    /// for the quality scoring system.
    pub async fn mark_healthy(&self, connection_id: &str, latency_ms: Option<f64>) {
        let mut health_map = self.health.write().await;
        let changed = health_map
            .get(connection_id)
            .map(|h| h.state != HealthState::Healthy)
            .unwrap_or(true);
        let h = health_map.entry(connection_id.to_string()).or_default();
        h.state = HealthState::Healthy;
        h.error_count = 0;
        h.last_healthy = Instant::now();
        h.reconnect_attempt = 0;
        h.next_reconnect_at = None;
        if let Some(lat) = latency_ms {
            h.total_queries = h.total_queries.saturating_add(1);
            h.total_latency_ms += lat;
        }
        if changed {
            self.emit_state_change(connection_id, HealthState::Healthy, None);
        }
    }

    pub(crate) async fn mark_reconnecting(&self, connection_id: &str) {
        let mut health = self.health.write().await;
        let entry = health.entry(connection_id.to_string()).or_default();
        let changed = entry.state != HealthState::Reconnecting;
        entry.state = HealthState::Reconnecting;
        entry.last_access = Instant::now();
        if changed {
            self.emit_state_change(connection_id, HealthState::Reconnecting, None);
        }
    }

    pub(crate) async fn mark_dead(&self, connection_id: &str, error: &str) {
        let mut health = self.health.write().await;
        let entry = health.entry(connection_id.to_string()).or_default();
        entry.state = HealthState::Dead;
        entry.error_count = entry.error_count.max(DEAD_THRESHOLD);
        entry.last_access = Instant::now();
        self.emit_state_change(connection_id, HealthState::Dead, Some(error));
    }

    pub(crate) async fn forget(&self, connection_id: &str) {
        self.health.write().await.remove(connection_id);
    }

    /// Mark a connection as errored after a failed operation.
    /// `latency_ms` is optional — when provided it updates query latency tracking
    /// for the quality scoring system.
    pub async fn mark_error(&self, connection_id: &str, error: &str, latency_ms: Option<f64>) {
        let mut health_map = self.health.write().await;
        let h = health_map.entry(connection_id.to_string()).or_default();
        h.error_count = h.error_count.saturating_add(1);
        h.last_access = Instant::now();
        if let Some(lat) = latency_ms {
            h.total_queries = h.total_queries.saturating_add(1);
            h.total_latency_ms += lat;
        }

        let new_state = if h.error_count >= DEAD_THRESHOLD {
            HealthState::Dead
        } else if h.error_count >= DEGRADED_THRESHOLD {
            HealthState::Degraded
        } else {
            HealthState::Healthy
        };

        if h.state != new_state {
            h.state = new_state;
            self.emit_state_change(connection_id, new_state, Some(error));
        }
    }

    /// Public accessor for health state (used by query execution to decide retry).
    pub async fn get_state(&self, connection_id: &str) -> HealthState {
        self.health
            .read()
            .await
            .get(connection_id)
            .map(|h| h.state)
            .unwrap_or(HealthState::Healthy)
    }

    /// Compute a connection quality score (0-100) based on latency history and reliability.
    /// Returns `None` if no health data exists for this connection.
    pub async fn quality_score(&self, connection_id: &str) -> Option<ConnectionQuality> {
        let health = self.health.read().await;
        let h = health.get(connection_id)?;

        let total_queries = h.total_queries;
        let avg_latency_ms = if total_queries > 0 {
            h.total_latency_ms / total_queries as f64
        } else {
            0.0
        };

        // Latency score: lower is better, 0-100
        let latency_score = if total_queries == 0 {
            100.0
        } else if avg_latency_ms <= 10.0 {
            100.0
        } else if avg_latency_ms <= 50.0 {
            80.0 + (50.0 - avg_latency_ms) / 40.0 * 20.0
        } else if avg_latency_ms <= 200.0 {
            50.0 + (200.0 - avg_latency_ms) / 150.0 * 30.0
        } else if avg_latency_ms <= 1000.0 {
            20.0 + (1000.0 - avg_latency_ms) / 800.0 * 30.0
        } else {
            0.0
        };

        // Reliability based on error rate
        let reliability = if total_queries == 0 {
            100.0
        } else {
            let error_rate = h.error_count as f64 / total_queries as f64;
            if error_rate <= 0.01 {
                100.0
            } else if error_rate <= 0.05 {
                80.0 + (0.05 - error_rate) / 0.04 * 20.0
            } else if error_rate <= 0.20 {
                50.0 + (0.20 - error_rate) / 0.15 * 30.0
            } else {
                0.0
            }
        };

        let score = latency_score * 0.3 + reliability * 0.7;
        let uptime_pct = if total_queries > 0 {
            ((total_queries.saturating_sub(h.error_count as u64)) as f64 / total_queries as f64)
                * 100.0
        } else {
            100.0
        };

        Some(ConnectionQuality {
            connection_id: connection_id.to_string(),
            state: h.state,
            error_count: h.error_count,
            total_queries,
            avg_latency_ms,
            uptime_pct,
            score,
        })
    }

    // ── internal ──────────────────────────────────────────────────────

    fn emit_state_change(&self, connection_id: &str, state: HealthState, error: Option<&str>) {
        let event = ConnectionStateEvent {
            connection_id: connection_id.to_string(),
            state,
            error: error.map(str::to_string),
        };
        if let Some(handle) = APP_HANDLE.get() {
            let _ = handle.emit("connection-state-changed", &event);
        }
    }

    async fn run_loop(&self) {
        let mut interval = tokio::time::interval(Duration::from_secs(GUARDIAN_INTERVAL_SECS));
        interval.set_missed_tick_behavior(MissedTickBehavior::Delay);

        loop {
            interval.tick().await;
            self.tick().await;
        }
    }

    async fn tick(&self) {
        let now = Instant::now();

        // Evict idle connections
        self.evict_idle(now).await;

        // Collect connections that need attention
        let candidates: Vec<(String, bool)> = {
            let health_map = self.health.read().await;
            health_map
                .iter()
                .filter(|(_, h)| match h.state {
                    HealthState::Dead | HealthState::Degraded | HealthState::Reconnecting => true,
                    _ => false,
                })
                .map(|(id, h)| (id.clone(), h.state == HealthState::Dead))
                .collect()
        };

        for (connection_id, is_dead) in candidates {
            if is_dead {
                self.try_reconnect(&connection_id, now).await;
            } else {
                self.ping(&connection_id).await;
            }
        }
    }

    async fn evict_idle(&self, now: Instant) {
        let idle_threshold = Duration::from_secs(self.idle_eviction_secs);
        let to_evict: Vec<String> = {
            let health_map = self.health.read().await;
            health_map
                .iter()
                .filter(|(_, h)| now.duration_since(h.last_access) > idle_threshold)
                .map(|(id, _)| id.clone())
                .collect()
        };

        for conn_id in to_evict {
            let state = APP_HANDLE
                .get()
                .expect("APP_HANDLE not initialized")
                .state::<AppState>();
            let gate = state.cache.reconnect_gate(&conn_id).await;
            let _guard = gate.lock().await;
            let still_idle = self
                .health
                .read()
                .await
                .get(&conn_id)
                .is_some_and(|health| health.last_access.elapsed() > idle_threshold);
            if !still_idle {
                continue;
            }
            let connection = state.connections.write().await.remove(&conn_id);
            state.cache.remove_all(&conn_id).await;
            if let Some(connection) = connection {
                let timeout = state
                    .configs
                    .read()
                    .await
                    .get(&conn_id)
                    .map(|config| config.connect_timeout_secs)
                    .unwrap_or(10);
                let _ = tokio::time::timeout(Duration::from_secs(timeout), connection.disconnect())
                    .await;
            }
            self.health.write().await.remove(&conn_id);
            self.emit_state_change(&conn_id, HealthState::Dead, Some("Idle eviction"));
        }
    }

    async fn try_reconnect(&self, connection_id: &str, now: Instant) {
        let state = APP_HANDLE
            .get()
            .expect("APP_HANDLE not initialized")
            .state::<AppState>();
        let gate = state.cache.reconnect_gate(connection_id).await;
        let guard = gate.lock().await;
        let (should_attempt, attempt_num) = {
            let health_map = self.health.read().await;
            let h = match health_map.get(connection_id) {
                Some(h) => h,
                None => return,
            };
            if h.state != HealthState::Dead {
                return;
            }
            let should = match h.next_reconnect_at {
                Some(t) => now >= t,
                None => true,
            };
            (should, h.reconnect_attempt)
        };

        if !should_attempt {
            return;
        }
        let failed = state.connections.read().await.get(connection_id).cloned();

        {
            let mut health_map = self.health.write().await;
            let Some(h) = health_map.get_mut(connection_id) else {
                return;
            };
            if h.state != HealthState::Dead {
                return;
            }
            h.state = HealthState::Reconnecting;
            h.reconnect_attempt = h.reconnect_attempt.saturating_add(1);
            self.emit_state_change(connection_id, HealthState::Reconnecting, None);
        }
        drop(guard);

        // Try to reconnect by recreating the adapter from the stored config.
        // Pinging the stale adapter is not enough — the underlying session may
        // be irrecoverable (server restart, dropped connection).
        let success = match failed {
            Some(connection) => state
                .reconnect_connection(connection_id, &connection)
                .await
                .is_ok(),
            None => state.ensure_connection(connection_id).await.is_ok(),
        };

        if !success {
            let _guard = gate.lock().await;
            // Exponential backoff
            let delay = (RECONNECT_BASE_DELAY_SECS
                .saturating_mul(2u64.saturating_pow(attempt_num)))
            .min(RECONNECT_MAX_DELAY_SECS);
            let next = Instant::now() + Duration::from_secs(delay);
            let mut health_map = self.health.write().await;
            if let Some(h) = health_map
                .get_mut(connection_id)
                .filter(|h| matches!(h.state, HealthState::Dead | HealthState::Reconnecting))
            {
                h.next_reconnect_at = Some(next);
                h.state = HealthState::Dead;
                self.emit_state_change(
                    connection_id,
                    HealthState::Dead,
                    Some(&format!(
                        "Reconnect attempt {} failed, retrying in {}s",
                        attempt_num, delay
                    )),
                );
            }
        }
    }

    async fn ping(&self, connection_id: &str) {
        let state = APP_HANDLE
            .get()
            .expect("APP_HANDLE not initialized")
            .state::<AppState>();
        let connection = state.connections.read().await.get(connection_id).cloned();
        let Some(connection) = connection else {
            return;
        };
        let success = match connection.check_connection().await {
            Ok(status) => status.is_connected,
            Err(DbError::QueryExecution(_) | DbError::InvalidQuery(_)) => true,
            Err(_) => false,
        };
        let gate = state.cache.reconnect_gate(connection_id).await;
        let _guard = gate.lock().await;
        if !state
            .connections
            .read()
            .await
            .get(connection_id)
            .is_some_and(|current| current.same_session(&connection))
        {
            return;
        }
        if success {
            self.mark_healthy(connection_id, None).await;
        } else {
            self.mark_error(connection_id, "Health check ping failed", None)
                .await;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn successful_reconnect_restores_health_and_clears_failures() {
        let guardian = ConnectionGuardian::new();
        guardian.mark_healthy("test", None).await;
        guardian.mark_dead("test", "connection closed").await;
        assert_eq!(guardian.get_state("test").await, HealthState::Dead);
        guardian.mark_reconnecting("test").await;
        assert_eq!(guardian.get_state("test").await, HealthState::Reconnecting);
        guardian.mark_healthy("test", Some(12.0)).await;
        let quality = guardian.quality_score("test").await.unwrap();
        assert_eq!(quality.state, HealthState::Healthy);
        assert_eq!(quality.error_count, 0);
        assert_eq!(quality.total_queries, 1);
    }

    #[tokio::test]
    async fn browse_access_updates_idle_activity_and_disconnect_forgets_health() {
        let guardian = ConnectionGuardian::new();
        guardian.mark_healthy("test", None).await;
        guardian
            .health
            .write()
            .await
            .get_mut("test")
            .unwrap()
            .last_access = Instant::now() - Duration::from_secs(3600);
        guardian.touch("test").await;
        assert!(
            guardian.health.read().await["test"].last_access.elapsed() < Duration::from_secs(1)
        );
        guardian.forget("test").await;
        assert!(guardian.quality_score("test").await.is_none());
    }
}
