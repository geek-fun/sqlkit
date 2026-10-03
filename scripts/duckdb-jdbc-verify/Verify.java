import java.nio.file.Files;
import java.nio.file.Path;
import java.nio.file.Paths;
import java.sql.Connection;
import java.sql.DriverManager;
import java.sql.ResultSet;
import java.sql.SQLException;
import java.sql.Statement;
import java.util.Properties;

/**
 * Verifies the DuckDB JDBC semantics that SQLKit's connection form and JDBC URL
 * building depend on. Run it with the DuckDB JDBC driver on the classpath:
 *
 *   scripts/verify-duckdb-jdbc-urls.sh
 *
 * Expectations encoded here (DuckDB 1.5.x):
 *   - a missing database file is created by the engine on connect
 *   - `;access_mode=READ_ONLY` works as a URL option and blocks writes
 *   - the same file opened twice with the SAME configuration shares one instance
 *   - the same file opened twice with DIFFERENT configurations is rejected
 *   - a NAMED in-memory instance (`memory:<label>`) is shared between connections,
 *     while an unnamed one is private to each connection (this is why SQLKit
 *     stores `memory:<label>` as the host of an in-memory connection)
 *   - a database path containing `;` cannot be expressed in the JDBC URL at all
 */
public class Verify {
    private static int failures = 0;

    private static void check(String name, boolean ok, String detail) {
        if (!ok) {
            failures++;
        }
        System.out.println((ok ? "PASS" : "FAIL") + " | " + name + (detail.isEmpty() ? "" : " | " + detail));
    }

    public static void main(String[] args) throws Exception {
        Path tmp = Files.createTempDirectory("duckdb-verify");
        Path cwd = Paths.get("").toAbsolutePath();
        Class.forName("org.duckdb.DuckDBDriver");

        // 1. A missing file is created by the engine on connect.
        Path db = tmp.resolve("auto.duckdb");
        try (Connection c = DriverManager.getConnection("jdbc:duckdb:" + db);
             Statement s = c.createStatement()) {
            s.execute("CREATE TABLE t AS SELECT 42 AS x");
        }
        check("missing file auto-created on connect", Files.exists(db), "size=" + Files.size(db));

        byte[] header = Files.readAllBytes(db);
        check("DUCK magic at byte offset 8", new String(header, 8, 4, "US-ASCII").equals("DUCK"), "");

        // 2. URL option: read-only connects and rejects writes.
        try (Connection ro = DriverManager.getConnection("jdbc:duckdb:" + db + ";access_mode=READ_ONLY");
             Statement s = ro.createStatement();
             ResultSet rs = s.executeQuery("SELECT count(*) FROM t")) {
            rs.next();
            check("URL option access_mode=READ_ONLY connects", rs.getInt(1) == 1, "count=" + rs.getInt(1));
            boolean rejected = false;
            try (Statement w = ro.createStatement()) {
                w.execute("CREATE TABLE nope AS SELECT 1");
            } catch (SQLException e) {
                rejected = true;
            }
            check("read-only connection rejects writes", rejected, "");
        }

        // 3. The equivalent driver property still works as a fallback path.
        Properties props = new Properties();
        props.setProperty("duckdb.read_only", "true");
        try (Connection ignored = DriverManager.getConnection("jdbc:duckdb:" + db, props)) {
            check("duckdb.read_only property connects", true, "");
        } catch (SQLException e) {
            check("duckdb.read_only property connects", false, e.getMessage());
        }

        // 4. Different configurations for one file in one JVM are rejected; this is
        //    what the connection form blocks before the engine ever sees it.
        try (Connection rw = DriverManager.getConnection("jdbc:duckdb:" + db)) {
            try (Connection ro = DriverManager.getConnection("jdbc:duckdb:" + db + ";access_mode=READ_ONLY")) {
                check("conflicting configuration rejected", false, "second connection unexpectedly succeeded");
            } catch (SQLException e) {
                check("conflicting configuration rejected", true, e.getMessage());
            }
        }

        // 5. Same file, same configuration -> one shared in-process instance.
        try (Connection a = DriverManager.getConnection("jdbc:duckdb:" + db);
             Connection b = DriverManager.getConnection("jdbc:duckdb:" + db);
             Statement s = a.createStatement()) {
            s.execute("CREATE TABLE IF NOT EXISTS shared_rw AS SELECT 1 AS x");
            try (Statement s2 = b.createStatement();
                 ResultSet rs = s2.executeQuery("SELECT count(*) FROM shared_rw")) {
                rs.next();
                check("same file shares one in-process instance", rs.getInt(1) == 1, "");
            }
        }

        // 6. Named in-memory instances are shared across pooled connections.
        try (Connection a = DriverManager.getConnection("jdbc:duckdb:memory:sqlkit_verify");
             Connection b = DriverManager.getConnection("jdbc:duckdb:memory:sqlkit_verify");
             Statement s = a.createStatement()) {
            s.execute("CREATE TABLE shared_mem AS SELECT 1 AS x");
            boolean visible = false;
            try (Statement s2 = b.createStatement();
                 ResultSet rs = s2.executeQuery("SELECT count(*) FROM shared_mem")) {
                rs.next();
                visible = rs.getInt(1) == 1;
            }
            check("named memory instance shared across connections", visible, "");
        }

        // 7. Unnamed in-memory instances are private to a connection: the reason
        //    SQLKit must never store a bare `:memory:` for a pooled DuckDB pool.
        try (Connection a = DriverManager.getConnection("jdbc:duckdb:");
             Connection b = DriverManager.getConnection("jdbc:duckdb:");
             Statement s = a.createStatement()) {
            s.execute("CREATE TABLE private_mem AS SELECT 1 AS x");
            boolean visible = true;
            try (Statement s2 = b.createStatement();
                 ResultSet rs = s2.executeQuery("SELECT count(*) FROM private_mem")) {
                rs.next();
            } catch (SQLException e) {
                visible = false;
            }
            check("unnamed memory instance is connection-private", !visible, "");
        }

        // 8. The legacy `:memory:` database value resolves to an in-memory database
        //    and does not create a literal file.
        try (Connection c = DriverManager.getConnection("jdbc:duckdb::memory:");
             Statement s = c.createStatement();
             ResultSet rs = s.executeQuery("SELECT current_database()")) {
            rs.next();
            check("legacy jdbc:duckdb::memory: connects", true, "current_database=" + rs.getString(1));
        } catch (SQLException e) {
            check("legacy jdbc:duckdb::memory: connects", false, e.getMessage());
        }
        check("no literal ':memory:' file created in cwd", !Files.exists(cwd.resolve(":memory:")), "");

        // 9. A path containing ';' cannot be expressed: the driver splits the URL on
        //    ';' and reports `Invalid URL entry`. SQLKit rejects such paths.
        Path semicolon = tmp.resolve("semi;colon.duckdb");
        boolean semicolonFailed = false;
        try (Connection ignored = DriverManager.getConnection("jdbc:duckdb:" + semicolon)) {
            semicolonFailed = false;
        } catch (SQLException e) {
            semicolonFailed = true;
        }
        check("path containing ';' is unusable in a JDBC URL", semicolonFailed, "");

        Path equals = tmp.resolve("eq=uals.duckdb");
        try (Connection ignored = DriverManager.getConnection("jdbc:duckdb:" + equals)) {
            check("path containing '=' opens as a file", Files.exists(equals), "");
        } catch (SQLException e) {
            check("path containing '=' opens as a file", false, e.getMessage());
        }

        System.out.println(failures == 0 ? "ALL CHECKS PASSED" : failures + " CHECK(S) FAILED");
        if (failures > 0) {
            System.exit(1);
        }
    }
}
