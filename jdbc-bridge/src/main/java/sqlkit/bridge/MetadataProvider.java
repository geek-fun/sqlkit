package sqlkit.bridge;

import java.sql.*;
import java.util.*;

/**
 * Provides database metadata: databases, schemas, tables, columns.
 */
public class MetadataProvider {

    /**
     * List all databases (catalogs) on the server.
     *
     * For Oracle, getCatalogs() returns empty because Oracle doesn't use
     * JDBC catalogs in the traditional sense. Fall back to querying the
     * current container/PDB name via SYS_CONTEXT, then try listing all
     * PDBs if connected to a CDB.
     */
    public static List<String> listDatabases(Connection conn) throws Exception {
        List<String> databases = new ArrayList<>();
        try (ResultSet rs = conn.getMetaData().getCatalogs()) {
            while (rs.next()) {
                String cat = rs.getString("TABLE_CAT");
                if (cat != null && !cat.isEmpty()) {
                    databases.add(cat);
                }
            }
        }

        // Oracle fallback: getCatalogs() returns empty for Oracle JDBC.
        // Try SYS_CONTEXT first (works in any Oracle container),
        // then v$pdbs (only works in CDB$ROOT).
        if (databases.isEmpty()) {
            try (Statement stmt = conn.createStatement();
                 ResultSet rs = stmt.executeQuery(
                     "SELECT SYS_CONTEXT('USERENV', 'CON_NAME') FROM DUAL")) {
                if (rs.next()) {
                    String conName = rs.getString(1);
                    if (conName != null && !conName.isEmpty()) {
                        databases.add(conName);
                    }
                }
            } catch (SQLException e) {
                // Fall through to v$pdbs
            }
        }

        if (databases.isEmpty()) {
            try (Statement stmt = conn.createStatement();
                 ResultSet rs = stmt.executeQuery("SELECT name FROM v$pdbs")) {
                while (rs.next()) {
                    String name = rs.getString(1);
                    if (name != null && !name.isEmpty()) {
                        databases.add(name);
                    }
                }
            } catch (SQLException e) {
                // Driver does not support Oracle-specific queries;
                // leave databases empty (frontend will fall back to
                // the configured connection database).
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

    /**
     * List all views in the given catalog/schema.
     */
    public static List<Map<String, Object>> listViews(Connection conn,
                                                      String database,
                                                      String schema) throws Exception {
        List<Map<String, Object>> views = new ArrayList<>();
        String catalog = (database != null && !database.isEmpty()) ? database : null;
        String schemaPattern = (schema != null && !schema.isEmpty()) ? schema : null;

        try (ResultSet rs = conn.getMetaData().getTables(catalog, schemaPattern, null,
                new String[]{"VIEW"})) {
            while (rs.next()) {
                Map<String, Object> v = new LinkedHashMap<>();
                v.put("name", rs.getString("TABLE_NAME"));
                v.put("object_type", "VIEW");
                v.put("schema", rs.getString("TABLE_SCHEM"));
                v.put("detail", rs.getString("REMARKS"));
                views.add(v);
            }
        }
        return views;
    }

    /**
     * List stored procedures in the given catalog/schema. Engines without
     * stored procedures (e.g. DuckDB) simply report an empty list.
     */
    public static List<Map<String, Object>> listProcedures(Connection conn,
                                                           String database,
                                                           String schema) throws Exception {
        List<Map<String, Object>> procedures = new ArrayList<>();
        String catalog = (database != null && !database.isEmpty()) ? database : null;
        String schemaPattern = (schema != null && !schema.isEmpty()) ? schema : null;

        try (ResultSet rs = conn.getMetaData().getProcedures(catalog, schemaPattern, null)) {
            while (rs.next()) {
                Map<String, Object> p = new LinkedHashMap<>();
                p.put("name", rs.getString("PROCEDURE_NAME"));
                p.put("object_type", "PROCEDURE");
                p.put("schema", rs.getString("PROCEDURE_SCHEM"));
                p.put("detail", rs.getString("REMARKS"));
                procedures.add(p);
            }
        } catch (SQLException e) {
            // Driver does not support procedure metadata — degrade to empty
        }
        return procedures;
    }

    /**
     * List functions in the given catalog/schema.
     */
    public static List<Map<String, Object>> listFunctions(Connection conn,
                                                          String database,
                                                          String schema) throws Exception {
        List<Map<String, Object>> functions = new ArrayList<>();
        String catalog = (database != null && !database.isEmpty()) ? database : null;
        String schemaPattern = (schema != null && !schema.isEmpty()) ? schema : null;

        try (ResultSet rs = conn.getMetaData().getFunctions(catalog, schemaPattern, null)) {
            while (rs.next()) {
                Map<String, Object> f = new LinkedHashMap<>();
                f.put("name", rs.getString("FUNCTION_NAME"));
                f.put("object_type", "FUNCTION");
                f.put("schema", rs.getString("FUNCTION_SCHEM"));
                f.put("detail", rs.getString("REMARKS"));
                functions.add(f);
            }
        } catch (SQLException e) {
            // Driver does not support function metadata — degrade to empty
        }
        return functions;
    }

    /**
     * List triggers on the given table (or all tables when {@code table} is null).
     *
     * JDBC has no trigger metadata API, so this uses engine SQL with graceful
     * degradation: information_schema.triggers first (DuckDB, SQL Server, ...),
     * then Oracle's all_triggers, else an empty list.
     */
    public static List<Map<String, Object>> listTriggers(Connection conn,
                                                         String database,
                                                         String schema,
                                                         String table) throws Exception {
        List<Map<String, Object>> triggers = new ArrayList<>();

        String sql = "SELECT trigger_name, action_timing, event_manipulation, action_statement, "
                + "trigger_schema, event_object_table FROM information_schema.triggers";
        try (Statement stmt = conn.createStatement();
             ResultSet rs = stmt.executeQuery(sql)) {
            while (rs.next()) {
                String triggerSchema = rs.getString("trigger_schema");
                String triggerTable = rs.getString("event_object_table");
                if (schema != null && !schema.isEmpty() && triggerSchema != null
                        && !triggerSchema.equalsIgnoreCase(schema)) {
                    continue;
                }
                if (table != null && !table.isEmpty() && triggerTable != null
                        && !triggerTable.equalsIgnoreCase(table)) {
                    continue;
                }
                Map<String, Object> t = new LinkedHashMap<>();
                t.put("name", rs.getString("trigger_name"));
                t.put("action_timing", rs.getString("action_timing"));
                t.put("event", rs.getString("event_manipulation"));
                t.put("ddl", rs.getString("action_statement"));
                triggers.add(t);
            }
        } catch (SQLException e) {
            // information_schema.triggers not available on this engine
            if (isOracle(conn)) {
                try (Statement stmt = conn.createStatement();
                     ResultSet rs = stmt.executeQuery(
                         "SELECT trigger_name, trigger_type, triggering_event, trigger_body, "
                             + "triggering_type, base_object_type, table_name FROM all_triggers")) {
                    while (rs.next()) {
                        String triggerTable = rs.getString("table_name");
                        if (table != null && !table.isEmpty() && triggerTable != null
                                && !triggerTable.equalsIgnoreCase(table)) {
                            continue;
                        }
                        Map<String, Object> t = new LinkedHashMap<>();
                        t.put("name", rs.getString("trigger_name"));
                        t.put("action_timing", rs.getString("trigger_type"));
                        t.put("event", rs.getString("triggering_event"));
                        t.put("ddl", readClob(rs.getObject("trigger_body")));
                        triggers.add(t);
                    }
                } catch (SQLException ignored) {
                    // Oracle metadata also unavailable — leave the list empty
                }
            }
        }
        return triggers;
    }

    /**
     * List indexes on a table via {@link DatabaseMetaData#getIndexInfo}.
     * Drivers whose metadata endpoint does not surface engine indexes fall
     * back to engine catalogs (DuckDB: {@code duckdb_indexes()}).
     */
    public static List<Map<String, Object>> listIndexes(Connection conn,
                                                        String database,
                                                        String schema,
                                                        String table) throws Exception {
        List<Map<String, Object>> result = indexesViaMetadata(conn, database, schema, table);
        if (result.isEmpty() && isDuckDB(conn)) {
            result = listIndexesViaDuckDB(conn, database, schema, table);
        }
        return result;
    }

    private static List<Map<String, Object>> indexesViaMetadata(Connection conn,
                                                                String database,
                                                                String schema,
                                                                String table) throws Exception {
        String catalog = (database != null && !database.isEmpty()) ? database : null;
        String schemaPattern = (schema != null && !schema.isEmpty()) ? schema : null;

        Set<String> pkColumns = new HashSet<>();
        try (ResultSet rs = conn.getMetaData().getPrimaryKeys(catalog, schemaPattern, table)) {
            while (rs.next()) {
                pkColumns.add(rs.getString("COLUMN_NAME"));
            }
        }

        Map<String, Map<String, Object>> indexes = new LinkedHashMap<>();
        Map<String, List<String>> indexColumns = new LinkedHashMap<>();
        try (ResultSet rs = conn.getMetaData().getIndexInfo(catalog, schemaPattern, table, false, true)) {
            while (rs.next()) {
                String indexName = rs.getString("INDEX_NAME");
                if (indexName == null) {
                    // tableIndexStatistic rows carry no index name
                    continue;
                }
                String column = rs.getString("COLUMN_NAME");
                if (column != null) {
                    indexColumns.computeIfAbsent(indexName, k -> new ArrayList<>()).add(column);
                }
                indexes.computeIfAbsent(indexName, k -> {
                    Map<String, Object> idx = new LinkedHashMap<>();
                    idx.put("name", k);
                    idx.put("is_unique", false);
                    // TYPE: tableIndexClustered=1, tableIndexHashed=2, other=3
                    int type;
                    try {
                        type = rs.getInt("TYPE");
                    } catch (SQLException e) {
                        type = 3;
                    }
                    String indexType;
                    if (type == 1) {
                        indexType = "CLUSTERED";
                    } else if (type == 2) {
                        indexType = "HASH";
                    } else {
                        indexType = "BTREE";
                    }
                    idx.put("index_type", indexType);
                    return idx;
                });
                try {
                    boolean nonUnique = rs.getBoolean("NON_UNIQUE");
                    if (!rs.wasNull()) {
                        indexes.get(indexName).put("is_unique", !nonUnique);
                    }
                } catch (SQLException ignored) {
                    // driver without NON_UNIQUE column — leave false
                }
            }
        }

        List<Map<String, Object>> result = new ArrayList<>();
        for (Map.Entry<String, List<String>> entry : indexColumns.entrySet()) {
            Map<String, Object> idx = indexes.get(entry.getKey());
            List<String> cols = entry.getValue();
            idx.put("columns", cols);
            // Mark as primary when the index spans exactly the table's PK columns
            idx.put("is_primary", !pkColumns.isEmpty() && pkColumns.equals(new HashSet<>(cols)));
            result.add(idx);
        }
        return result;
    }

    /**
     * DuckDB-specific index listing: the DuckDB JDBC credits an empty result
     * from getIndexInfo, so read {@code duckdb_indexes()} directly.
     */
    private static List<Map<String, Object>> listIndexesViaDuckDB(Connection conn,
                                                                  String database,
                                                                  String schema,
                                                                  String table) throws Exception {
        List<Map<String, Object>> result = new ArrayList<>();
        String sql = "SELECT index_name, schema_name, table_name, is_unique, is_primary, "
                + "expressions FROM duckdb_indexes()";
        try (Statement stmt = conn.createStatement();
             ResultSet rs = stmt.executeQuery(sql)) {
            while (rs.next()) {
                String indexSchema = safeString(rs, "schema_name");
                String indexTable = safeString(rs, "table_name");
                if (table != null && indexTable != null && !indexTable.equalsIgnoreCase(table)) {
                    continue;
                }
                if (schema != null && !schema.isEmpty() && indexSchema != null
                        && !indexSchema.equalsIgnoreCase(schema)) {
                    continue;
                }
                List<String> columns = readColumnNames(rs.getObject("expressions"));
                Map<String, Object> idx = new LinkedHashMap<>();
                idx.put("name", rs.getString("index_name"));
                idx.put("columns", columns);
                idx.put("index_type", "BTREE");
                Boolean isUnique = maybeBoolean(rs, "is_unique");
                idx.put("is_unique", isUnique != null && isUnique);
                Boolean isPrimary = maybeBoolean(rs, "is_primary");
                idx.put("is_primary", isPrimary != null && isPrimary);
                result.add(idx);
            }
        }
        return result;
    }

    /** Parse a bracketed name list (e.g. DuckDB's `expressions` = `[a, b]`). */
    private static List<String> readColumnNames(Object value) throws SQLException {
        List<String> columns = new ArrayList<>();
        if (value instanceof java.sql.Array) {
            Object raw = ((java.sql.Array) value).getArray();
            Object[] elements = raw instanceof Object[] ? (Object[]) raw : new Object[]{raw};
            for (Object element : elements) {
                columns.add(element == null ? "" : element.toString());
            }
            return columns;
        }
        if (value instanceof String) {
            String joined = (String) value;
            joined = joined.replaceFirst("^\\[", "").replaceFirst("\\]$", "");
            for (String part : joined.split(",")) {
                columns.add(part.trim().replace("\"", "").replace("'", "").trim());
            }
        }
        return columns;
    }

    private static Boolean maybeBoolean(ResultSet rs, String column) {
        try {
            boolean value = rs.getBoolean(column);
            if (rs.wasNull()) {
                return null;
            }
            return value;
        } catch (SQLException e) {
            return null;
        }
    }

    /**
     * List foreign keys pointing from a table to other tables via
     * {@link DatabaseMetaData#getImportedKeys}.
     */
    public static List<Map<String, Object>> listForeignKeys(Connection conn,
                                                            String database,
                                                            String schema,
                                                            String table) throws Exception {
        String catalog = (database != null && !database.isEmpty()) ? database : null;
        String schemaPattern = (schema != null && !schema.isEmpty()) ? schema : null;

        Map<String, Map<String, Object>> fks = new LinkedHashMap<>();
        Map<String, List<String>> fkColumns = new LinkedHashMap<>();
        Map<String, List<String>> fkReferencedColumns = new LinkedHashMap<>();
        try (ResultSet rs = conn.getMetaData().getImportedKeys(catalog, schemaPattern, table)) {
            while (rs.next()) {
                String constraintName = rs.getString("FK_NAME");
                if (constraintName == null) {
                    constraintName = "fk_" + rs.getString("FKTABLE_NAME");
                }
                fkColumns.computeIfAbsent(constraintName, k -> new ArrayList<>())
                        .add(rs.getString("FKCOLUMN_NAME"));
                fkReferencedColumns.computeIfAbsent(constraintName, k -> new ArrayList<>())
                        .add(rs.getString("PKCOLUMN_NAME"));
                fks.computeIfAbsent(constraintName, k -> {
                    Map<String, Object> fk = new LinkedHashMap<>();
                    fk.put("constraint_name", k);
                    fk.put("source_table", table);
                    fk.put("referenced_schema", safeString(rs, "PKTABLE_SCHEM"));
                    fk.put("referenced_table", safeString(rs, "PKTABLE_NAME"));
                    fk.put("on_update", mapRefentialAction(rs, "UPDATE_RULE"));
                    fk.put("on_delete", mapRefentialAction(rs, "DELETE_RULE"));
                    return fk;
                });
            }
        }

        List<Map<String, Object>> result = new ArrayList<>();
        for (String name : fks.keySet()) {
            Map<String, Object> fk = fks.get(name);
            fk.put("columns", fkColumns.get(name));
            fk.put("referenced_columns", fkReferencedColumns.get(name));
            result.add(fk);
        }
        return result;
    }

    /**
     * List all foreign keys in a schema/database via
     * {@link DatabaseMetaData#getExportedKeys} with a "%" table pattern.
     * Drivers that reject the pattern degrade to an empty list.
     */
    public static List<Map<String, Object>> listAllForeignKeys(Connection conn,
                                                               String database,
                                                               String schema) throws Exception {
        String catalog = (database != null && !database.isEmpty()) ? database : null;
        String schemaPattern = (schema != null && !schema.isEmpty()) ? schema : null;

        Map<String, Map<String, Object>> fks = new LinkedHashMap<>();
        Map<String, List<String>> fkColumns = new LinkedHashMap<>();
        Map<String, List<String>> fkReferencedColumns = new LinkedHashMap<>();
        try (ResultSet rs = conn.getMetaData().getExportedKeys(catalog, schemaPattern, "%")) {
            collectForeignKeys(rs, fks, fkColumns, fkReferencedColumns);
        } catch (SQLException e) {
            // Pattern-based exported keys are not portable — degrade to empty
        }

        List<Map<String, Object>> result = new ArrayList<>();
        for (String name : fks.keySet()) {
            Map<String, Object> fk = fks.get(name);
            fk.put("columns", fkColumns.get(name));
            fk.put("referenced_columns", fkReferencedColumns.get(name));
            result.add(fk);
        }
        return result;
    }

    private static void collectForeignKeys(ResultSet rs,
                                           Map<String, Map<String, Object>> fks,
                                           Map<String, List<String>> fkColumns,
                                           Map<String, List<String>> fkReferencedColumns) throws SQLException {
        while (rs.next()) {
            String constraintName = rs.getString("FK_NAME");
            String sourceTable = rs.getString("FKTABLE_NAME");
            if (constraintName == null) {
                constraintName = "fk_" + sourceTable;
            }
            fkColumns.computeIfAbsent(constraintName, k -> new ArrayList<>())
                    .add(rs.getString("FKCOLUMN_NAME"));
            fkReferencedColumns.computeIfAbsent(constraintName, k -> new ArrayList<>())
                    .add(rs.getString("PKCOLUMN_NAME"));
            fks.computeIfAbsent(constraintName, k -> {
                Map<String, Object> fk = new LinkedHashMap<>();
                fk.put("constraint_name", k);
                fk.put("source_table", sourceTable);
                fk.put("referenced_schema", safeString(rs, "PKTABLE_SCHEM"));
                fk.put("referenced_table", safeString(rs, "PKTABLE_NAME"));
                fk.put("on_update", mapRefentialAction(rs, "UPDATE_RULE"));
                fk.put("on_delete", mapRefentialAction(rs, "DELETE_RULE"));
                return fk;
            });
        }
    }

    private static String mapRefentialAction(ResultSet rs, String column) {
        try {
            int rule = rs.getInt(column);
            if (rs.wasNull()) {
                return null;
            }
            switch (rule) {
                case DatabaseMetaData.importedKeyCascade:
                    return "CASCADE";
                case DatabaseMetaData.importedKeyRestrict:
                    return "RESTRICT";
                case DatabaseMetaData.importedKeySetNull:
                    return "SET NULL";
                case DatabaseMetaData.importedKeySetDefault:
                    return "SET DEFAULT";
                case DatabaseMetaData.importedKeyNoAction:
                default:
                    return "NO ACTION";
            }
        } catch (SQLException e) {
            return null;
        }
    }

    private static String safeString(ResultSet rs, String column) {
        try {
            return rs.getString(column);
        } catch (SQLException e) {
            return null;
        }
    }

    private static boolean isOracle(Connection conn) {
        try {
            return conn.getMetaData().getDatabaseProductName().contains("Oracle");
        } catch (SQLException e) {
            return false;
        }
    }

    private static String readClob(Object value) {
        if (value instanceof Clob) {
            try {
                Clob clob = (Clob) value;
                return clob.getSubString(1, (int) clob.length());
            } catch (SQLException e) {
                return null;
            }
        }
        return value instanceof String ? (String) value : null;
    }

    /**
     * Get the DDL source for a view/trigger/routine. DDL is not exposed through
     * JDBC metadata, so this uses engine SQL with a fallback chain and degrades
     * to a {@code null} ddl value when no strategy applies.
     */
    public static Map<String, Object> getObjectDdl(Connection conn,
                                                   String database,
                                                   String schema,
                                                   String objectName,
                                                   String objectType) throws Exception {
        String schemaPattern = (schema != null && !schema.isEmpty()) ? schema : "";

        String ddl = null;
        if ("VIEW".equalsIgnoreCase(objectType)) {
            ddl = firstNonEmpty(conn,
                () -> String.format(
                    "SELECT view_definition FROM information_schema.views "
                        + "WHERE table_schema = '%s' AND table_name = '%s'",
                    escapeLiteral(schemaPattern), escapeLiteral(objectName)),
                () -> String.format(
                    "SELECT sql FROM duckdb_views() WHERE NOT internal "
                        + "AND schema_name = '%s' AND view_name = '%s'",
                    escapeLiteral(schemaPattern), escapeLiteral(objectName)),
                () -> String.format(
                    "SELECT text FROM all_views WHERE view_name = '%s'",
                    escapeLiteral(objectName)));
        } else if ("TRIGGER".equalsIgnoreCase(objectType)) {
            ddl = firstNonEmpty(conn,
                () -> String.format(
                    "SELECT action_statement FROM information_schema.triggers "
                        + "WHERE trigger_name = '%s'",
                    escapeLiteral(objectName)),
                () -> String.format(
                    "SELECT trigger_body FROM all_triggers WHERE trigger_name = '%s'",
                    escapeLiteral(objectName)));
        } else if ("PROCEDURE".equalsIgnoreCase(objectType)
                || "FUNCTION".equalsIgnoreCase(objectType)) {
            ddl = firstNonEmpty(conn,
                () -> String.format(
                    "SELECT routine_definition FROM information_schema.routines "
                        + "WHERE routine_type = '%s' AND routine_name = '%s'",
                    "PROCEDURE".equalsIgnoreCase(objectType) ? "PROCEDURE" : "FUNCTION",
                    escapeLiteral(objectName)),
                () -> String.format(
                    "SELECT text FROM all_source WHERE name = '%s' AND type = '%s'",
                    escapeLiteral(objectName),
                    "PROCEDURE".equalsIgnoreCase(objectType) ? "PROCEDURE" : "FUNCTION"));
        }

        Map<String, Object> result = new LinkedHashMap<>();
        result.put("ddl", ddl);
        return result;
    }

    /**
     * Drop a view/table/routine with standard SQL. Engine differences are
     * handled here: Oracle/Firebird reject IF EXISTS, so it is only added for
     * engines whose DROP supports it.
     */
    public static void dropObject(Connection conn,
                                  String database,
                                  String schema,
                                  String objectName,
                                  String objectType) throws Exception {
        String type = objectType.toUpperCase(Locale.ROOT);
        if (!type.equals("VIEW") && !type.equals("TABLE") && !type.equals("PROCEDURE")
                && !type.equals("FUNCTION") && !type.equals("MATERIALIZED VIEW")) {
            throw new Exception("drop_object not supported for type: " + objectType);
        }

        boolean supportsIfExists = !isOracle(conn) && !isFirebird(conn);

        String qualified = quoteQualified(schema, objectName);
        String sql = "DROP " + type + (supportsIfExists ? " IF EXISTS " : " ") + qualified;

        try (Statement stmt = conn.createStatement()) {
            stmt.execute(sql);
        }
    }

    /**
     * Rename a table/view. Standard SQL is {@code ALTER <type> <name> RENAME TO <new>};
     * Firebird uses {@code ALTER TABLE <name> TO <new>}.
     */
    public static void renameObject(Connection conn,
                                    String database,
                                    String schema,
                                    String objectName,
                                    String objectType,
                                    String newName) throws Exception {
        String type = objectType.toUpperCase(Locale.ROOT);
        if (!type.equals("TABLE") && !type.equals("VIEW")) {
            throw new Exception("rename_object not supported for type: " + objectType);
        }

        String qualified = quoteQualified(schema, objectName);
        String sql;
        if (isFirebird(conn)) {
            sql = String.format("ALTER %s ALTER %s TO %s", type, qualified, quoteIdentifier(newName));
        } else {
            sql = String.format("ALTER %s %s RENAME TO %s", type, qualified, quoteIdentifier(newName));
        }

        try (Statement stmt = conn.createStatement()) {
            stmt.execute(sql);
        }
    }

    // ── SQL helpers ──

    private static boolean isFirebird(Connection conn) {
        try {
            String product = conn.getMetaData().getDatabaseProductName();
            return product != null && product.toLowerCase(Locale.ROOT).contains("firebird");
        } catch (SQLException e) {
            return false;
        }
    }

    private static boolean isDuckDB(Connection conn) {
        try {
            String product = conn.getMetaData().getDatabaseProductName();
            return product != null && product.toLowerCase(Locale.ROOT).contains("duckdb");
        } catch (SQLException e) {
            return false;
        }
    }

    private static String quoteIdentifier(String identifier) {
        return "\"" + identifier.replace("\"", "\"\"") + "\"";
    }

    private static String quoteQualified(String schema, String name) {
        if (schema != null && !schema.isEmpty()) {
            return quoteIdentifier(schema) + "." + quoteIdentifier(name);
        }
        return quoteIdentifier(name);
    }

    private static String escapeLiteral(String value) {
        return value == null ? "" : value.replace("'", "''");
    }

    private interface SqlCandidate {
        String build();
    }

    /**
     * Run candidate DDL queries in order and return the first non-empty,
     * non-null single value. Engines lacking the queried catalog simply raise
     * a SQLException, which advances to the next candidate.
     */
    private static String firstNonEmpty(Connection conn, SqlCandidate... candidates) throws SQLException {
        for (SqlCandidate candidate : candidates) {
            String sql = candidate.build();
            try (Statement stmt = conn.createStatement();
                 ResultSet rs = stmt.executeQuery(sql)) {
                if (rs.next()) {
                    String value = rs.getString(1);
                    if (value != null && !value.trim().isEmpty()) {
                        return value;
                    }
                }
            } catch (SQLException e) {
                // try the next fallback
            }
        }
        return null;
    }
}
