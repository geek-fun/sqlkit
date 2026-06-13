package sqlkit.bridge;

import java.sql.*;
import java.util.*;

/**
 * Executes SQL queries against a JDBC connection and serializes results to JSON-compatible Maps.
 */
public class QueryExecutor {

    /**
     * Execute a SQL query and return the result as a Map.
     * <p>
     * For SELECT queries, returns {columns: [...], rows: [[...], ...]}.
     * For UPDATE/INSERT/DELETE, returns {rows_affected: N}.
     */
    public static Map<String, Object> execute(Connection conn, String sql) throws Exception {
        sql = sql.trim();

        boolean isQuery;
        String upper = sql.toUpperCase().trim();
        isQuery = upper.startsWith("SELECT")
                || upper.startsWith("WITH")
                || upper.startsWith("EXPLAIN")
                || upper.startsWith("SHOW")
                || upper.startsWith("DESCRIBE")
                || upper.startsWith("PRAGMA");

        if (isQuery) {
            return executeQuery(conn, sql);
        } else {
            return executeUpdate(conn, sql);
        }
    }

    private static Map<String, Object> executeQuery(Connection conn, String sql) throws Exception {
        try (Statement stmt = conn.createStatement();
             ResultSet rs = stmt.executeQuery(sql)) {

            ResultSetMetaData meta = rs.getMetaData();
            int columnCount = meta.getColumnCount();

            List<String> columns = new ArrayList<>();
            for (int i = 1; i <= columnCount; i++) {
                columns.add(meta.getColumnLabel(i));
            }

            List<List<Object>> rows = new ArrayList<>();
            while (rs.next()) {
                List<Object> row = new ArrayList<>();
                for (int i = 1; i <= columnCount; i++) {
                    row.add(getValue(rs, i));
                }
                rows.add(row);
            }

            Map<String, Object> result = new LinkedHashMap<>();
            result.put("columns", columns);
            result.put("rows", rows);
            return result;
        }
    }

    private static Map<String, Object> executeUpdate(Connection conn, String sql) throws Exception {
        try (Statement stmt = conn.createStatement()) {
            int affected = stmt.executeUpdate(sql);
            Map<String, Object> result = new LinkedHashMap<>();
            result.put("rows_affected", (long) affected);
            result.put("columns", Collections.emptyList());
            result.put("rows", Collections.emptyList());
            return result;
        }
    }

    /**
     * Extract a value from a ResultSet at the given column index, converting to
     * a JSON-friendly Java type.
     */
    private static Object getValue(ResultSet rs, int index) throws SQLException {
        Object val = rs.getObject(index);
        if (val == null) {
            return null;
        }
        // Convert specific JDBC types to plain Java types
        if (val instanceof Blob) {
            Blob blob = (Blob) val;
            byte[] bytes = blob.getBytes(1, (int) blob.length());
            return Base64.getEncoder().encodeToString(bytes);
        }
        if (val instanceof Clob) {
            Clob clob = (Clob) val;
            return clob.getSubString(1, (int) clob.length());
        }
        if (val instanceof java.sql.Date) {
            return val.toString();
        }
        if (val instanceof java.sql.Time) {
            return val.toString();
        }
        if (val instanceof java.sql.Timestamp) {
            return val.toString();
        }
        if (val instanceof java.util.Date) {
            return val.toString();
        }
        if (val instanceof byte[]) {
            return Base64.getEncoder().encodeToString((byte[]) val);
        }
        if (val instanceof java.math.BigDecimal) {
            return ((java.math.BigDecimal) val).toPlainString();
        }
        // For arrays, convert to list of strings
        if (val instanceof java.sql.Array) {
            java.sql.Array arr = (java.sql.Array) val;
            Object[] arrElements = (Object[]) arr.getArray();
            List<String> elements = new ArrayList<>();
            for (Object elem : arrElements) {
                elements.add(elem == null ? null : elem.toString());
            }
            return String.join(",", elements);
        }
        // Return as-is for simple types (String, Integer, Long, Double, Boolean)
        return val;
    }
}
