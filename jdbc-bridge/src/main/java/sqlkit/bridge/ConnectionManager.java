package sqlkit.bridge;

import com.zaxxer.hikari.HikariConfig;
import com.zaxxer.hikari.HikariDataSource;

import java.sql.*;
import java.util.*;

/**
 * Manages HikariCP connection pools for JDBC bridge connections.
 * Each connection is identified by a unique conn_id string.
 */
public class ConnectionManager {

    private final Map<String, HikariDataSource> pools = new HashMap<>();

    /**
     * Create a new JDBC connection pool.
     *
     * @param connId      unique identifier for this connection
     * @param url         JDBC URL
     * @param username    database username
     * @param password    database password
     * @param driverClass JDBC driver class name
     * @param minPool     minimum pool size
     * @param maxPool     maximum pool size
     */
    public void connect(String connId, String url, String username,
                        String password, String driverClass,
                        int minPool, int maxPool) throws Exception {
        if (pools.containsKey(connId)) {
            throw new Exception("Connection already exists: " + connId);
        }

        // Load the JDBC driver class
        Class.forName(driverClass);

        HikariConfig config = new HikariConfig();
        config.setJdbcUrl(url);
        config.setUsername(username);
        if (password != null && !password.isEmpty()) {
            config.setPassword(password);
        }
        config.setMinimumIdle(minPool);
        config.setMaximumPoolSize(maxPool);
        config.setConnectionTimeout(30000);
        config.setIdleTimeout(600000);
        config.setMaxLifetime(1800000);
        config.addDataSourceProperty("cachePrepStmts", "true");
        config.addDataSourceProperty("prepStmtCacheSize", "250");
        config.addDataSourceProperty("prepStmtCacheSqlLimit", "2048");

        HikariDataSource ds = new HikariDataSource(config);

        // Verify connection works
        try (Connection c = ds.getConnection()) {
            // ok
        } catch (Exception e) {
            ds.close();
            throw new Exception("Failed to verify connection: " + e.getMessage());
        }

        pools.put(connId, ds);
    }

    /**
     * Close and remove a connection pool.
     */
    public void disconnect(String connId) {
        HikariDataSource ds = pools.remove(connId);
        if (ds != null) {
            ds.close();
        }
    }

    /**
     * Get a connection from the pool for the given connId.
     */
    public Connection getConnection(String connId) throws Exception {
        HikariDataSource ds = pools.get(connId);
        if (ds == null) {
            throw new Exception("Connection not found: " + connId);
        }
        return ds.getConnection();
    }

    /**
     * Test a connection — return status metadata as a Map.
     */
    public Map<String, Object> testConnection(String connId) throws Exception {
        try (Connection c = getConnection(connId)) {
            Map<String, Object> status = new LinkedHashMap<>();
            status.put("is_connected", true);

            DatabaseMetaData meta = c.getMetaData();
            status.put("server_version", meta.getDatabaseProductVersion());
            status.put("current_database", c.getCatalog());
            try {
                status.put("current_user", meta.getUserName());
            } catch (Exception e) {
                status.put("current_user", null);
            }

            return status;
        }
    }

    /**
     * Close all connection pools.
     */
    public void closeAll() {
        for (HikariDataSource ds : pools.values()) {
            ds.close();
        }
        pools.clear();
    }
}
