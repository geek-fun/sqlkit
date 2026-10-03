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
     * Driver classes loaded once per JAR set.
     *
     * A fresh class loader per connection re-runs the driver's static
     * initialisation, which for DuckDB maps a ~107 MB native library (measured
     * ~1.4 s). Caching the loader keeps that cost to the first connection of the
     * bridge process; entries are released together with the process.
     */
    private final Map<String, DriverClassLoader> sharedLoaders = new HashMap<>();
    private final Map<String, Driver> sharedDrivers = new HashMap<>();

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

        final java.sql.Driver driver = sharedDriver(driverClass, driverJars);
        DriverClassLoader loader = loaderFor(driverJars);
        loaders.put(connId, loader);

        HikariConfig config = new HikariConfig();
        // Keep HikariCP's read-only flag aligned with the connection the driver
        // already opened: HikariCP calls `connection.setReadOnly(config.isReadOnly())`
        // whenever the two disagree, and DuckDB rejects changing the mode at
        // connection level ("Can't change read-only status on connection level").
        config.setReadOnly(isReadOnlyUrl(jdbcUrl));
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

    /** Reuse one {@link DriverClassLoader} per set of driver JARs. */
    private DriverClassLoader loaderFor(List<String> driverJars) throws Exception {
        String key = String.join("|", driverJars);
        DriverClassLoader loader = sharedLoaders.get(key);
        if (loader == null) {
            loader = new DriverClassLoader(driverJars);
            sharedLoaders.put(key, loader);
        }
        return loader;
    }

    /** Reuse the instantiated JDBC driver for a driver class + JAR set. */
    private Driver sharedDriver(String driverClass, List<String> driverJars) throws Exception {
        String key = driverClass + "|" + String.join("|", driverJars);
        Driver driver = sharedDrivers.get(key);
        if (driver != null) {
            return driver;
        }

        DriverClassLoader loader = loaderFor(driverJars);
        Class<?> driverCls = Class.forName(driverClass, true, loader);
        if (!java.sql.Driver.class.isAssignableFrom(driverCls)) {
            throw new ClassifiedException("Class " + driverClass + " does not implement java.sql.Driver", null, ErrorClassifier.ErrorType.UNKNOWN);
        }
        driver = (java.sql.Driver) driverCls.getDeclaredConstructor().newInstance();
        sharedDrivers.put(key, driver);
        return driver;
    }

    /**
     * Whether a JDBC URL asks for a read-only database.
     *
     * DuckDB expresses this in the URL (`access_mode=READ_ONLY` from SQLKit, or
     * the driver property `duckdb.read_only`), and the driver refuses to change
     * the mode through {@link java.sql.Connection#setReadOnly(boolean)}.
     */
    static boolean isReadOnlyUrl(String jdbcUrl) {
        if (jdbcUrl == null) {
            return false;
        }
        String normalized = jdbcUrl.toLowerCase(Locale.ROOT);
        return normalized.contains("access_mode=read_only")
                || normalized.contains("duckdb.read_only=true");
    }

    /**
     * Close and remove a connection pool.
     */
    public void disconnect(String connId) {
        HikariDataSource ds = pools.remove(connId);
        if (ds != null) {
            ds.close();
        }
        // The loader is shared with other connections (see sharedLoaders), so it is
        // only closed when the whole bridge shuts down.
        loaders.remove(connId);
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
        for (DriverClassLoader loader : sharedLoaders.values()) {
            try { loader.close(); } catch (Exception ignored) { }
        }
        sharedLoaders.clear();
        sharedDrivers.clear();
    }
}
