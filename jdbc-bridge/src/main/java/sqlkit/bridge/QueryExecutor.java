package sqlkit.bridge;

import java.sql.*;
import java.util.*;

/**
 * Executes SQL statements against a JDBC connection and serializes results to JSON-compatible Maps.
 */
public class QueryExecutor {

    /**
     * Execute a SQL statement and return the result as a Map.
     * <p>
     * Uses {@link Statement#execute(String)} so no keyword sniffing is needed:
     * statements that return rows (SELECT, WITH ... SELECT, INSERT ... RETURNING)
     * yield {columns, rows}; everything else (INSERT/UPDATE/DELETE/MERGE/DDL)
     * yields {rows_affected: N}.
     */
    public static Map<String, Object> execute(Connection conn, String sql) throws Exception {
        try (Statement stmt = conn.createStatement()) {
            boolean isResultSet = stmt.execute(sql);

            List<String> columns = null;
            List<List<Object>> rows = null;
            long rowsAffected = 0;

            while (true) {
                if (isResultSet) {
                    try (ResultSet rs = stmt.getResultSet()) {
                        ResultSetMetaData meta = rs.getMetaData();
                        int columnCount = meta.getColumnCount();

                        List<String> resultColumns = new ArrayList<>();
                        for (int i = 1; i <= columnCount; i++) {
                            resultColumns.add(meta.getColumnLabel(i));
                        }

                        List<List<Object>> resultRows = new ArrayList<>();
                        while (rs.next()) {
                            List<Object> row = new ArrayList<>();
                            for (int i = 1; i <= columnCount; i++) {
                                row.add(getValue(rs, i));
                            }
                            resultRows.add(row);
                        }
                        columns = resultColumns;
                        rows = resultRows;
                    }
                } else {
                    int count = stmt.getUpdateCount();
                    if (count >= 0) {
                        rowsAffected = count;
                    }
                }
                isResultSet = stmt.getMoreResults();
                if (!isResultSet && stmt.getUpdateCount() == -1) {
                    break;
                }
            }

            Map<String, Object> result = new LinkedHashMap<>();
            if (rows != null) {
                result.put("columns", columns);
                result.put("rows", rows);
            } else {
                result.put("rows_affected", rowsAffected);
                result.put("columns", Collections.emptyList());
                result.put("rows", Collections.emptyList());
            }
            return result;
        }
    }

    /**
     * Extract a value from a ResultSet at the given column index, converting to
     * a JSON-friendly Java type.
     */
    static Object getValue(ResultSet rs, int index) throws SQLException {
        ResultSetMetaData metadata = rs.getMetaData();
        int type = metadata.getColumnType(index);
        String typeName = metadata.getColumnTypeName(index);
        // Read through JDBC accessors before getObject can expose vendor objects.
        if (type == Types.DATE || type == Types.TIME || type == Types.TIMESTAMP
                || type == Types.TIME_WITH_TIMEZONE || type == Types.TIMESTAMP_WITH_TIMEZONE
                || "JSON".equalsIgnoreCase(typeName)) {
            return rs.getString(index);
        }
        Object val = rs.getObject(index);
        if (val == null) {
            return null;
        }
        if (val instanceof Struct struct) {
            List<String> attributes = new ArrayList<>();
            for (Object attribute : struct.getAttributes()) {
                attributes.add(attribute == null ? "NULL" : attribute.toString());
            }
            return struct.getSQLTypeName() + "(" + String.join(", ", attributes) + ")";
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
        if (val instanceof String || val instanceof Number || val instanceof Boolean) {
            return val;
        }
        // Oracle intervals, XML and other vendor values must not reach Jackson.
        return rs.getString(index);
    }
}
