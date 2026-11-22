# Connection recovery

All database actions acquire their session through `AppState::ensure_connection`.
This includes metadata browsing, SQL execution, row edits, DDL, EXPLAIN, transfers,
connection status, cached handles, and AI/MCP tools. Adapter-specific commands
retain their existing feature support.

The shared acquisition path validates an existing session using its adapter's
connection check before starting the requested operation. Successful checks are
reused for one second to coalesce bursts of metadata requests. Checks wait for
an adapter already in use before applying the configured connection timeout.
They do not mistake a busy adapter for a broken connection.

A failed connectivity check replaces the session once using the original saved
configuration, including database, TLS and SSH settings. The replacement must
pass its own connection check before it is published. A per-connection mutex
coordinates foreground actions, background reconnects, explicit disconnects,
and idle eviction. Cached handles and validation records are invalidated when a
session is replaced or removed. Background health checks do not refresh the
user-activity timestamp.

Authentication, configuration, driver-version and SQL errors are distinguished
from connection failures. A SQL error during a probe proves that the server
answered, so recovery SQL such as ROLLBACK remains possible. It does not cause
a session replacement. Connection checks cover every adapter route:

- PostgreSQL and compatible wire-protocol engines.
- MySQL and compatible wire-protocol engines.
- SQL Server, SQLite and SQLCipher.
- ClickHouse, Trino, Presto, RQLite and Turso HTTP adapters.
- All engines served by the JDBC bridge.

The JDBC bridge classifies SQL exceptions by type and SQLSTATE; Rust also
recognizes closed-session responses from older bridge builds. HTTP transport
errors and unavailable gateways are classified separately from query failures.

The requested action starts only after session acquisition succeeds. Arbitrary
SQL, edits, DDL and transfers are never automatically replayed after execution
starts: a lost response could otherwise repeat a committed write. Table reads
have one additional bounded retry after a transport failure during execution.
Failures leave the connection recoverable by a later action or the background
guardian. Explicit disconnection removes the saved runtime configuration and
prevents background reconnection.
