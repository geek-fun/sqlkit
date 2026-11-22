package sqlkit.bridge;

import org.junit.jupiter.api.Test;

import static org.junit.jupiter.api.Assertions.assertFalse;
import static org.junit.jupiter.api.Assertions.assertTrue;

class ConnectionManagerTest {

    @Test
    void detectsReadOnlyJdbcUrls() {
        assertTrue(ConnectionManager.isReadOnlyUrl("/tmp/app.duckdb;access_mode=READ_ONLY"));
        assertTrue(ConnectionManager.isReadOnlyUrl("jdbc:duckdb:/data/app.duckdb;ACCESS_MODE=read_only"));
        assertTrue(ConnectionManager.isReadOnlyUrl("jdbc:duckdb:/data/app.duckdb;duckdb.read_only=true"));
    }

    @Test
    void treatsEverythingElseAsReadWrite() {
        assertFalse(ConnectionManager.isReadOnlyUrl("jdbc:duckdb:/data/app.duckdb"));
        assertFalse(ConnectionManager.isReadOnlyUrl("jdbc:duckdb:memory:shared"));
        assertFalse(ConnectionManager.isReadOnlyUrl("jdbc:duckdb:/data/app.duckdb;duckdb.read_only=false"));
        assertFalse(ConnectionManager.isReadOnlyUrl(null));
    }
}
