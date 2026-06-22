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
    private final Map<String, DriverClassLoader> loaders = new HashMap<>();

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
                        List<String> driverJars,
                        int minPool, int maxPool,
                        boolean credentialsInUrl,
                        String sslMode, String sslCaCert,
                        String sslClientCert, String sslClientKey,
                        boolean trustServerCertificate) throws ClassifiedException, Exception {
        if (pools.containsKey(connId)) {
            throw new Exception("Connection already exists: " + connId);
        }

        if (credentialsInUrl) {
            StringBuilder sb = new StringBuilder(url);
            sb.append(url.contains("?") ? "&" : "?");
            sb.append("user=").append(username);
            if (password != null && !password.isEmpty()) {
                sb.append("&password=").append(password);
            }
            url = sb.toString();
        }
        final String jdbcUrl = url;

        DriverClassLoader loader = new DriverClassLoader(driverJars);
        Class<?> driverCls = Class.forName(driverClass, true, loader);
        if (!java.sql.Driver.class.isAssignableFrom(driverCls)) {
            throw new ClassifiedException("Class " + driverClass + " does not implement java.sql.Driver", null, ErrorClassifier.ErrorType.UNKNOWN);
        }
        final java.sql.Driver driver = (java.sql.Driver) driverCls.getDeclaredConstructor().newInstance();
        loaders.put(connId, loader);

        HikariConfig config = new HikariConfig();
        config.setDataSource(new javax.sql.DataSource() {
            public java.sql.Connection getConnection() throws java.sql.SQLException {
                java.util.Properties info = new java.util.Properties();
                if (username != null) info.setProperty("user", username);
                if (password != null) info.setProperty("password", password);
                SslPropertyMapper.applySslProperties(driverClass, jdbcUrl, sslMode, sslCaCert, sslClientCert, sslClientKey, trustServerCertificate, info);
                return driver.connect(jdbcUrl, info);
            }
            public java.sql.Connection getConnection(String u, String p) throws java.sql.SQLException {
                java.util.Properties info = new java.util.Properties();
                if (u != null) info.setProperty("user", u);
                if (p != null) info.setProperty("password", p);
                SslPropertyMapper.applySslProperties(driverClass, jdbcUrl, sslMode, sslCaCert, sslClientCert, sslClientKey, trustServerCertificate, info);
                return driver.connect(jdbcUrl, info);
            }
            public java.io.PrintWriter getLogWriter() { return null; }
            public void setLogWriter(java.io.PrintWriter out) {}
            public void setLoginTimeout(int seconds) {}
            public int getLoginTimeout() { return 0; }
            public java.util.logging.Logger getParentLogger() { return java.util.logging.Logger.getLogger("sqlkit.bridge"); }
            @SuppressWarnings("unchecked")
            public <T> T unwrap(Class<T> iface) throws java.sql.SQLException { throw new java.sql.SQLException("Not supported"); }
            public boolean isWrapperFor(Class<?> iface) { return false; }
        });
        config.setUsername(username);
        if (password != null && !password.isEmpty()) {
            config.setPassword(password);
        }
        config.setMinimumIdle(minPool);
        config.setMaximumPoolSize(maxPool);
        config.setConnectionTimeout(30000);
        config.setIdleTimeout(600000);
        config.setMaxLifetime(1800000);

        HikariDataSource ds = new HikariDataSource(config);

        // Verify connection works
        try (Connection c = ds.getConnection()) {
            // ok
        } catch (Exception e) {
            ds.close();
            ErrorClassifier.ErrorType errorType = ErrorClassifier.classify(e);
            throw new ClassifiedException("Failed to verify connection: " + e.getMessage(), e, errorType);
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
        DriverClassLoader loader = loaders.remove(connId);
        if (loader != null) {
            try { loader.close(); } catch (Exception ignored) { }
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
        for (DriverClassLoader loader : loaders.values()) {
            try { loader.close(); } catch (Exception ignored) { }
        }
        loaders.clear();
    }
}
