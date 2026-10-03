package sqlkit.bridge;

import org.junit.jupiter.api.Test;

import java.util.Properties;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertTrue;

class SslPropertyMapperTest {

    private Properties apply(String driverClass, String sslMode) {
        Properties props = new Properties();
        SslPropertyMapper.applySslProperties(
                driverClass, "jdbc:test://host/db", sslMode, null, null, null, false, props);
        return props;
    }

    @Test
    void noSslPropertiesWhenDisabledOrAbsent() {
        assertTrue(apply("org.postgresql.Driver", null).isEmpty());
        assertTrue(apply("org.postgresql.Driver", "disable").isEmpty());
    }

    @Test
    void unknownDriversReceiveNoProperties() {
        // Issue #158: DuckDB rejects unknown options — the blind fallback
        // `ssl=true` broke every connection. Drivers without an explicit
        // case must not be injected at all (H2 and Firebird have no
        // dedicated case either).
        assertTrue(apply("org.duckdb.DuckDBDriver", "prefer").isEmpty());
        assertTrue(apply("org.duckdb.DuckDBDriver", "require").isEmpty());
        assertTrue(apply("org.h2.Driver", "prefer").isEmpty());
        assertTrue(apply("org.firebirdsql.jdbc.FBDriver", "require").isEmpty());
        assertTrue(apply("com.example.UnknownDriver", "require").isEmpty());
    }

    @Test
    void knownDriversKeepTheirSslProperties() {
        assertEquals("require", apply("org.postgresql.Driver", "require").getProperty("sslmode"));
        assertEquals("VERIFY_IDENTITY",
                apply("com.mysql.cj.jdbc.Driver", "verify-full").getProperty("sslMode"));
        assertEquals("on", apply("net.snowflake.client.jdbc.SnowflakeDriver", "require").getProperty("ssl"));
        assertEquals("true", apply("org.apache.hive.jdbc.HiveDriver", "require").getProperty("ssl"));

        Properties sqlServer = apply("com.microsoft.sqlserver.jdbc.SQLServerDriver", "require");
        assertEquals("true", sqlServer.getProperty("encrypt"));
        assertEquals("true", sqlServer.getProperty("trustServerCertificate"));

        Properties oracle = apply("oracle.jdbc.OracleDriver", "verify-full");
        assertEquals("true", oracle.getProperty("oracle.net.ssl"));
        assertEquals("true", oracle.getProperty("oracle.net.ssl_server_dn_match"));

        Properties vertica = apply("com.vertica.jdbc.Driver", "verify-full");
        assertEquals("true", vertica.getProperty("ssl"));
        assertEquals("true", vertica.getProperty("ssl_hostname_verify"));
    }
}
