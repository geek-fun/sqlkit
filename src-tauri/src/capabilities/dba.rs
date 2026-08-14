//! DBA SQL tools for the MCP bridge.
//!
//! Dedicated capabilities for session inspection/termination, slow-query
//! discovery, and privilege management. These call dedicated
//! `DatabaseAdapter` methods that bypass `classify_sql`, so
//! `Statement::Grant/Revoke/Kill` never reaches the write/DDL gate.

use std::sync::Arc;

use async_trait::async_trait;
use serde_json::{json, Value};

use data_studio_agent::capabilities::registry::CapabilityRegistry;
use data_studio_agent::capabilities::types::{
    Capability, CapabilityHandler, RiskLevel, SourceKind,
};

use super::sql::{
    get_connection_id, get_slow_queries_on_adapter, grant_privilege_on_adapter,
    kill_session_on_adapter, list_sessions_on_adapter, resolve_adapter,
    revoke_privilege_on_adapter,
};

// ---------------------------------------------------------------------------
// Handler structs
// ---------------------------------------------------------------------------

struct ListSessionsHandler;
struct KillSessionHandler;
struct GetSlowQueriesHandler;
struct GrantPrivilegeHandler;
struct RevokePrivilegeHandler;

// ---------------------------------------------------------------------------
// Handler implementations
// ---------------------------------------------------------------------------

#[async_trait]
impl CapabilityHandler for ListSessionsHandler {
    async fn handle(
        &self,
        args: &Value,
        connection_config: Option<&Value>,
    ) -> Result<String, String> {
        let conn_id = get_connection_id(connection_config)?;
        let database = args.get("database").and_then(|v| v.as_str());
        let adapter = resolve_adapter(&conn_id).await?;
        let sessions = list_sessions_on_adapter(&adapter, database).await?;
        serde_json::to_string(&sessions).map_err(|e| e.to_string())
    }
}

#[async_trait]
impl CapabilityHandler for KillSessionHandler {
    async fn handle(
        &self,
        args: &Value,
        connection_config: Option<&Value>,
    ) -> Result<String, String> {
        let conn_id = get_connection_id(connection_config)?;
        let session_id = args
            .get("session_id")
            .and_then(|v| v.as_str())
            .ok_or_else(|| "Missing 'session_id' argument".to_string())?;
        let adapter = resolve_adapter(&conn_id).await?;
        kill_session_on_adapter(&adapter, session_id).await?;
        serde_json::to_string(&json!({ "status": "ok", "session_id": session_id }))
            .map_err(|e| e.to_string())
    }
}

#[async_trait]
impl CapabilityHandler for GetSlowQueriesHandler {
    async fn handle(
        &self,
        args: &Value,
        connection_config: Option<&Value>,
    ) -> Result<String, String> {
        let conn_id = get_connection_id(connection_config)?;
        let database = args.get("database").and_then(|v| v.as_str());
        let limit = args.get("limit").and_then(|v| v.as_u64()).map(|n| n as u32);
        let adapter = resolve_adapter(&conn_id).await?;
        let slow = get_slow_queries_on_adapter(&adapter, database, limit).await?;
        serde_json::to_string(&slow).map_err(|e| e.to_string())
    }
}

#[async_trait]
impl CapabilityHandler for GrantPrivilegeHandler {
    async fn handle(
        &self,
        args: &Value,
        connection_config: Option<&Value>,
    ) -> Result<String, String> {
        let conn_id = get_connection_id(connection_config)?;
        let privilege = args
            .get("privilege")
            .and_then(|v| v.as_str())
            .ok_or_else(|| "Missing 'privilege' argument".to_string())?;
        let object = args
            .get("object")
            .and_then(|v| v.as_str())
            .ok_or_else(|| "Missing 'object' argument".to_string())?;
        let grantee = args
            .get("grantee")
            .and_then(|v| v.as_str())
            .ok_or_else(|| "Missing 'grantee' argument".to_string())?;
        let adapter = resolve_adapter(&conn_id).await?;
        grant_privilege_on_adapter(&adapter, privilege, object, grantee).await?;
        serde_json::to_string(&json!({ "status": "ok" })).map_err(|e| e.to_string())
    }
}

#[async_trait]
impl CapabilityHandler for RevokePrivilegeHandler {
    async fn handle(
        &self,
        args: &Value,
        connection_config: Option<&Value>,
    ) -> Result<String, String> {
        let conn_id = get_connection_id(connection_config)?;
        let privilege = args
            .get("privilege")
            .and_then(|v| v.as_str())
            .ok_or_else(|| "Missing 'privilege' argument".to_string())?;
        let object = args
            .get("object")
            .and_then(|v| v.as_str())
            .ok_or_else(|| "Missing 'object' argument".to_string())?;
        let grantee = args
            .get("grantee")
            .and_then(|v| v.as_str())
            .ok_or_else(|| "Missing 'grantee' argument".to_string())?;
        let adapter = resolve_adapter(&conn_id).await?;
        revoke_privilege_on_adapter(&adapter, privilege, object, grantee).await?;
        serde_json::to_string(&json!({ "status": "ok" })).map_err(|e| e.to_string())
    }
}

// ---------------------------------------------------------------------------
// Registration
// ---------------------------------------------------------------------------

fn connection_id_schema() -> Value {
    json!({
        "type": "string",
        "description": "The connection alias to use (e.g. 'mac-postgresql'). Use sqlkit__list_connections to see available connections."
    })
}

pub(crate) fn register_dba_tools(reg: &mut CapabilityRegistry) {
    reg.register(Capability {
        name: "sqlkit__list_sessions",
        description: "List active database sessions/connections: id, user, database, state, and running query. Use to see who is connected and what they are running.",
        handler: Arc::new(ListSessionsHandler),
        input_schema: json!({"type": "object", "properties": {
            "connection_id": connection_id_schema(),
            "database": {"type": "string", "description": "Database name (optional)"}
        }, "required": ["connection_id"]}),
        risk_level: RiskLevel::Safe,
        required_permission: "read",
        source_kind: SourceKind::SqlDatabase,
        tags: &["agent"],
        parallel_ok: true,
    });

    reg.register(Capability {
        name: "sqlkit__kill_session",
        description: "Terminate a database session by its session id (PostgreSQL PID for postgres). DANGEROUS: kills an in-flight query/connection. Requires Full Access in Settings → MCP Bridge.",
        handler: Arc::new(KillSessionHandler),
        input_schema: json!({"type": "object", "properties": {
            "connection_id": connection_id_schema(),
            "session_id": {"type": "string", "description": "Session id to terminate (numeric; for PostgreSQL this is the backend PID)"}
        }, "required": ["connection_id", "session_id"]}),
        risk_level: RiskLevel::Elevated,
        required_permission: "create",
        source_kind: SourceKind::SqlDatabase,
        tags: &["agent"],
        parallel_ok: false,
    });

    reg.register(Capability {
        name: "sqlkit__get_slow_queries",
        description: "List currently slow-running queries (or cached slow query statistics) on the server: duration, user, and query text. Use when investigating performance issues.",
        handler: Arc::new(GetSlowQueriesHandler),
        input_schema: json!({"type": "object", "properties": {
            "connection_id": connection_id_schema(),
            "database": {"type": "string", "description": "Database name (optional)"},
            "limit": {"type": "integer", "description": "Maximum number of queries to return (optional, default 20)"}
        }, "required": ["connection_id"]}),
        risk_level: RiskLevel::Safe,
        required_permission: "read",
        source_kind: SourceKind::SqlDatabase,
        tags: &["agent"],
        parallel_ok: true,
    });

    reg.register(Capability {
        name: "sqlkit__grant_privilege",
        description: "Grant a privilege (e.g. SELECT, INSERT) on an object (e.g. a table) to a user/role. Requires Full Access in Settings → MCP Bridge.",
        handler: Arc::new(GrantPrivilegeHandler),
        input_schema: json!({"type": "object", "properties": {
            "connection_id": connection_id_schema(),
            "privilege": {"type": "string", "description": "Privilege(s) to grant, e.g. SELECT, INSERT, or a comma-separated list"},
            "object": {"type": "string", "description": "Object to grant on, e.g. public.users or db.table"},
            "grantee": {"type": "string", "description": "User or role to grant to"}
        }, "required": ["connection_id", "privilege", "object", "grantee"]}),
        risk_level: RiskLevel::Elevated,
        required_permission: "create",
        source_kind: SourceKind::SqlDatabase,
        tags: &["agent"],
        parallel_ok: false,
    });

    reg.register(Capability {
        name: "sqlkit__revoke_privilege",
        description: "Revoke a privilege (e.g. SELECT, INSERT) on an object from a user/role. Requires Full Access in Settings → MCP Bridge.",
        handler: Arc::new(RevokePrivilegeHandler),
        input_schema: json!({"type": "object", "properties": {
            "connection_id": connection_id_schema(),
            "privilege": {"type": "string", "description": "Privilege(s) to revoke, e.g. SELECT, INSERT, or a comma-separated list"},
            "object": {"type": "string", "description": "Object to revoke on, e.g. public.users or db.table"},
            "grantee": {"type": "string", "description": "User or role to revoke from"}
        }, "required": ["connection_id", "privilege", "object", "grantee"]}),
        risk_level: RiskLevel::Elevated,
        required_permission: "create",
        source_kind: SourceKind::SqlDatabase,
        tags: &["agent"],
        parallel_ok: false,
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn list_sessions_missing_config() {
        let err = ListSessionsHandler
            .handle(&json!({}), None)
            .await
            .unwrap_err();
        assert!(err.contains("connection_id"), "got: {}", err);
        assert!(err.contains("Settings → MCP Bridge"), "got: {}", err);
    }

    #[tokio::test]
    async fn kill_session_missing_config() {
        let err = KillSessionHandler
            .handle(&json!({ "session_id": "123" }), None)
            .await
            .unwrap_err();
        assert!(err.contains("connection_id"), "got: {}", err);
        assert!(err.contains("Settings → MCP Bridge"), "got: {}", err);
    }

    #[tokio::test]
    async fn kill_session_rejects_missing_session_id() {
        let config = json!({ "connectionId": "conn-1" });
        let err = KillSessionHandler
            .handle(&json!({}), Some(&config))
            .await
            .unwrap_err();
        assert!(err.contains("Missing 'session_id'"), "got: {}", err);
    }

    #[tokio::test]
    async fn get_slow_queries_missing_config() {
        let err = GetSlowQueriesHandler
            .handle(&json!({}), None)
            .await
            .unwrap_err();
        assert!(err.contains("connection_id"), "got: {}", err);
        assert!(err.contains("Settings → MCP Bridge"), "got: {}", err);
    }

    #[tokio::test]
    async fn grant_privilege_missing_config() {
        let err = GrantPrivilegeHandler
            .handle(
                &json!({ "privilege": "SELECT", "object": "users", "grantee": "app" }),
                None,
            )
            .await
            .unwrap_err();
        assert!(err.contains("connection_id"), "got: {}", err);
        assert!(err.contains("Settings → MCP Bridge"), "got: {}", err);
    }

    #[tokio::test]
    async fn grant_privilege_rejects_missing_privilege() {
        let config = json!({ "connectionId": "conn-1" });
        let err = GrantPrivilegeHandler
            .handle(
                &json!({ "object": "users", "grantee": "app" }),
                Some(&config),
            )
            .await
            .unwrap_err();
        assert!(err.contains("Missing 'privilege'"), "got: {}", err);
    }

    #[tokio::test]
    async fn grant_privilege_rejects_missing_object() {
        let config = json!({ "connectionId": "conn-1" });
        let err = GrantPrivilegeHandler
            .handle(
                &json!({ "privilege": "SELECT", "grantee": "app" }),
                Some(&config),
            )
            .await
            .unwrap_err();
        assert!(err.contains("Missing 'object'"), "got: {}", err);
    }

    #[tokio::test]
    async fn grant_privilege_rejects_missing_grantee() {
        let config = json!({ "connectionId": "conn-1" });
        let err = GrantPrivilegeHandler
            .handle(
                &json!({ "privilege": "SELECT", "object": "users" }),
                Some(&config),
            )
            .await
            .unwrap_err();
        assert!(err.contains("Missing 'grantee'"), "got: {}", err);
    }

    #[tokio::test]
    async fn revoke_privilege_missing_config() {
        let err = RevokePrivilegeHandler
            .handle(
                &json!({ "privilege": "SELECT", "object": "users", "grantee": "app" }),
                None,
            )
            .await
            .unwrap_err();
        assert!(err.contains("connection_id"), "got: {}", err);
        assert!(err.contains("Settings → MCP Bridge"), "got: {}", err);
    }

    #[tokio::test]
    async fn revoke_privilege_rejects_missing_privilege() {
        let config = json!({ "connectionId": "conn-1" });
        let err = RevokePrivilegeHandler
            .handle(
                &json!({ "object": "users", "grantee": "app" }),
                Some(&config),
            )
            .await
            .unwrap_err();
        assert!(err.contains("Missing 'privilege'"), "got: {}", err);
    }

    #[tokio::test]
    async fn revoke_privilege_rejects_missing_object() {
        let config = json!({ "connectionId": "conn-1" });
        let err = RevokePrivilegeHandler
            .handle(
                &json!({ "privilege": "SELECT", "grantee": "app" }),
                Some(&config),
            )
            .await
            .unwrap_err();
        assert!(err.contains("Missing 'object'"), "got: {}", err);
    }

    #[tokio::test]
    async fn revoke_privilege_rejects_missing_grantee() {
        let config = json!({ "connectionId": "conn-1" });
        let err = RevokePrivilegeHandler
            .handle(
                &json!({ "privilege": "SELECT", "object": "users" }),
                Some(&config),
            )
            .await
            .unwrap_err();
        assert!(err.contains("Missing 'grantee'"), "got: {}", err);
    }
}
