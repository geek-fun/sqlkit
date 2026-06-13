package sqlkit.bridge;

import java.sql.*;
import java.util.*;

/**
 * Provides database metadata: databases, schemas, tables, columns.
 */
public class MetadataProvider {

    /**
     * List all databases (catalogs) on the server.
     */
    public static List<String> listDatabases(Connection conn) throws Exception {
        List<String> databases = new ArrayList<>();
        try (ResultSet rs = conn.getMetaData().getCatalogs()) {
            while (rs.next()) {
                databases.add(rs.getString("TABLE_CAT"));
            }
        }
        return databases;
    }

    /**
     * List all schemas in the given database (catalog).
     */
    public static List<String> listSchemas(Connection conn, String database) throws Exception {
        List<String> schemas = new ArrayList<>();
        String catalog = (database != null && !database.isEmpty()) ? database : null;
        try (ResultSet rs = conn.getMetaData().getSchemas(catalog, null)) {
            while (rs.next()) {
                schemas.add(rs.getString("TABLE_SCHEM"));
            }
        }
        return schemas;
    }

    /**
     * List all tables in the given catalog/schema.
     */
    public static List<Map<String, Object>> listTables(Connection conn,
                                                       String database,
                                                       String schema) throws Exception {
        List<Map<String, Object>> tables = new ArrayList<>();
        String catalog = (database != null && !database.isEmpty()) ? database : null;
        String schemaPattern = (schema != null && !schema.isEmpty()) ? schema : null;

        try (ResultSet rs = conn.getMetaData().getTables(catalog, schemaPattern, null,
                new String[]{"TABLE", "VIEW", "SYSTEM TABLE", "ALIAS", "SYNONYM"})) {
            while (rs.next()) {
                Map<String, Object> t = new LinkedHashMap<>();
                t.put("name", rs.getString("TABLE_NAME"));
                t.put("schema", rs.getString("TABLE_SCHEM"));
                t.put("table_type", rs.getString("TABLE_TYPE"));
                t.put("row_count", null);
                tables.add(t);
            }
        }
        return tables;
    }

    /**
     * List all columns for a given table.
     */
    public static List<Map<String, Object>> listColumns(Connection conn,
                                                        String database,
                                                        String schema,
                                                        String table) throws Exception {
        List<Map<String, Object>> columns = new ArrayList<>();
        String catalog = (database != null && !database.isEmpty()) ? database : null;
        String schemaPattern = (schema != null && !schema.isEmpty()) ? schema : null;

        try (ResultSet rs = conn.getMetaData().getColumns(catalog, schemaPattern, table, null)) {
            while (rs.next()) {
                Map<String, Object> c = new LinkedHashMap<>();
                c.put("name", rs.getString("COLUMN_NAME"));
                c.put("data_type", rs.getString("TYPE_NAME"));
                c.put("nullable", rs.getInt("NULLABLE") == DatabaseMetaData.columnNullable);
                c.put("default_value", rs.getString("COLUMN_DEF"));
                c.put("is_primary_key", false); // filled below
                c.put("is_auto_increment", "YES".equalsIgnoreCase(rs.getString("IS_AUTOINCREMENT")));
                c.put("max_length", rs.getInt("COLUMN_SIZE"));
                c.put("precision", rs.getInt("DECIMAL_DIGITS"));
                c.put("scale", rs.getInt("DECIMAL_DIGITS"));
                columns.add(c);
            }
        }

        // Fetch primary keys for this table to set is_primary_key
        Set<String> pkColumns = new HashSet<>();
        try (ResultSet rs = conn.getMetaData().getPrimaryKeys(catalog, schemaPattern, table)) {
            while (rs.next()) {
                pkColumns.add(rs.getString("COLUMN_NAME"));
            }
        }
        for (Map<String, Object> c : columns) {
            if (pkColumns.contains(c.get("name"))) {
                c.put("is_primary_key", true);
            }
        }

        return columns;
    }
}
