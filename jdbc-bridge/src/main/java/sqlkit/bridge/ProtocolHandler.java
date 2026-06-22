package sqlkit.bridge;

import com.fasterxml.jackson.databind.JsonNode;
import com.fasterxml.jackson.databind.ObjectMapper;
import com.fasterxml.jackson.databind.node.ArrayNode;
import com.fasterxml.jackson.databind.node.ObjectNode;

import java.sql.Connection;
import java.util.ArrayList;
import java.util.List;
import java.util.Map;
import java.util.UUID;

/**
 * Dispatches JSON-RPC requests to the appropriate handler.
 */
public class ProtocolHandler {

    private static final ObjectMapper MAPPER = new ObjectMapper();

    private final ConnectionManager connectionManager;

    public ProtocolHandler(ConnectionManager connectionManager) {
        this.connectionManager = connectionManager;
    }

    /**
     * Handle a single JSON-RPC request and return a response.
     */
    public ObjectNode handle(JsonNode request) {
        long id = request.has("id") ? request.get("id").asLong(-1) : -1;
        String method = request.has("method") ? request.get("method").asText("") : "";
        JsonNode params = request.has("params") ? request.get("params") : MAPPER.missingNode();

        try {
            ObjectNode response = MAPPER.createObjectNode();
            response.put("id", id);

            switch (method) {
                case "ping":
                    response.put("result", "pong");
                    break;

                case "connect":
                    handleConnect(params, response);
                    break;

                case "disconnect":
                    handleDisconnect(params, response);
                    break;

                case "execute_query":
                    handleExecuteQuery(params, response);
                    break;

                case "list_databases":
                    handleListDatabases(params, response);
                    break;

                case "list_schemas":
                    handleListSchemas(params, response);
                    break;

                case "list_tables":
                    handleListTables(params, response);
                    break;

                case "list_columns":
                    handleListColumns(params, response);
                    break;

                case "resolve_driver":
                    handleResolveDriver(params, response);
                    break;

                case "test_connection":
                    handleTestConnection(params, response);
                    break;

                default:
                    response.put("error", "Unknown method: " + method);
                    break;
            }

            return response;
        } catch (ClassifiedException e) {
            ObjectNode errorResp = MAPPER.createObjectNode();
            errorResp.put("id", id);
            errorResp.put("error", buildErrorMessage(e));
            errorResp.put("error_type", e.getErrorType().name().toLowerCase());
            return errorResp;
        } catch (Exception e) {
            ObjectNode errorResp = MAPPER.createObjectNode();
            errorResp.put("id", id);
            errorResp.put("error", buildErrorMessage(e));
            return errorResp;
        }
    }

    private static String buildErrorMessage(Throwable e) {
        StringBuilder sb = new StringBuilder();
        Throwable current = e;
        while (current != null) {
            if (sb.length() > 0) {
                sb.append("\nCaused by: ");
            }
            String msg = current.getMessage();
            sb.append(msg != null ? msg : current.getClass().getName());
            current = current.getCause();
        }
        return sb.toString();
    }

    private void handleConnect(JsonNode params, ObjectNode response) throws Exception {
        String connId = requiredString(params, "conn_id",
                UUID.randomUUID().toString());
        String url = requiredString(params, "url", null);
        String username = params.has("username") ? params.get("username").asText("") : "";
        String password = params.has("password") && !params.get("password").isNull()
                ? params.get("password").asText() : null;
        String driverClass = requiredString(params, "driver_class", null);
        int poolMin = params.has("pool_min") ? params.get("pool_min").asInt(1) : 1;
        int poolMax = params.has("pool_max") ? params.get("pool_max").asInt(5) : 5;
        boolean credentialsInUrl = params.has("credentials_in_url") && !params.get("credentials_in_url").isNull()
                && params.get("credentials_in_url").asBoolean(false);

        List<String> driverJars = new ArrayList<>();
        if (params.has("driver_jars") && params.get("driver_jars").isArray()) {
            for (JsonNode jar : params.get("driver_jars")) {
                driverJars.add(jar.asText());
            }
        }

        String sslMode = params.has("ssl_mode") && !params.get("ssl_mode").isNull() ? params.get("ssl_mode").asText() : null;
        String sslCaCert = params.has("ssl_ca_cert") && !params.get("ssl_ca_cert").isNull() ? params.get("ssl_ca_cert").asText() : null;
        String sslClientCert = params.has("ssl_client_cert") && !params.get("ssl_client_cert").isNull() ? params.get("ssl_client_cert").asText() : null;
        String sslClientKey = params.has("ssl_client_key") && !params.get("ssl_client_key").isNull() ? params.get("ssl_client_key").asText() : null;
        boolean trustServerCertificate = params.has("trust_server_certificate") && params.get("trust_server_certificate").asBoolean(false);

        connectionManager.connect(connId, url, username, password, driverClass, driverJars, poolMin, poolMax, credentialsInUrl, sslMode, sslCaCert, sslClientCert, sslClientKey, trustServerCertificate);
        response.put("result", connId);
    }

    private void handleDisconnect(JsonNode params, ObjectNode response) throws Exception {
        String connId = requiredString(params, "conn_id", null);
        connectionManager.disconnect(connId);
        response.put("result", "ok");
    }

    private void handleExecuteQuery(JsonNode params, ObjectNode response) throws Exception {
        String connId = requiredString(params, "conn_id", null);
        String sql = requiredString(params, "sql", null);

        try (Connection conn = connectionManager.getConnection(connId)) {
            Map<String, Object> result = QueryExecutor.execute(conn, sql);
            JsonNode json = MAPPER.valueToTree(result);
            response.set("result", json);
        }
    }

    private void handleListDatabases(JsonNode params, ObjectNode response) throws Exception {
        String connId = requiredString(params, "conn_id", null);
        try (Connection conn = connectionManager.getConnection(connId)) {
            List<String> databases = MetadataProvider.listDatabases(conn);
            ArrayNode arr = MAPPER.valueToTree(databases);
            response.set("result", arr);
        }
    }

    private void handleListSchemas(JsonNode params, ObjectNode response) throws Exception {
        String connId = requiredString(params, "conn_id", null);
        String database = params.has("database") && !params.get("database").isNull()
                ? params.get("database").asText() : null;
        try (Connection conn = connectionManager.getConnection(connId)) {
            List<String> schemas = MetadataProvider.listSchemas(conn, database);
            ArrayNode arr = MAPPER.valueToTree(schemas);
            response.set("result", arr);
        }
    }

    private void handleListTables(JsonNode params, ObjectNode response) throws Exception {
        String connId = requiredString(params, "conn_id", null);
        String database = params.has("database") && !params.get("database").isNull()
                ? params.get("database").asText() : null;
        String schema = params.has("schema") && !params.get("schema").isNull()
                ? params.get("schema").asText() : null;
        try (Connection conn = connectionManager.getConnection(connId)) {
            List<Map<String, Object>> tables = MetadataProvider.listTables(conn, database, schema);
            ArrayNode arr = MAPPER.valueToTree(tables);
            response.set("result", arr);
        }
    }

    private void handleListColumns(JsonNode params, ObjectNode response) throws Exception {
        String connId = requiredString(params, "conn_id", null);
        String database = params.has("database") && !params.get("database").isNull()
                ? params.get("database").asText() : null;
        String schema = params.has("schema") && !params.get("schema").isNull()
                ? params.get("schema").asText() : null;
        String table = requiredString(params, "table", null);
        try (Connection conn = connectionManager.getConnection(connId)) {
            List<Map<String, Object>> columns = MetadataProvider.listColumns(conn, database, schema, table);
            ArrayNode arr = MAPPER.valueToTree(columns);
            response.set("result", arr);
        }
    }

    private void handleTestConnection(JsonNode params, ObjectNode response) throws Exception {
        String connId = requiredString(params, "conn_id", null);
        Map<String, Object> status = connectionManager.testConnection(connId);
        JsonNode json = MAPPER.valueToTree(status);
        response.set("result", json);
    }

    private void handleResolveDriver(JsonNode params, ObjectNode response) throws Exception {
        String mavenGroup = requiredString(params, "maven_group", null);
        String mavenArtifact = requiredString(params, "maven_artifact", null);
        String versionCap = params.has("version_cap") && !params.get("version_cap").isNull()
                ? params.get("version_cap").asText() : null;
        String classifier = params.has("maven_classifier") && !params.get("maven_classifier").isNull()
                ? params.get("maven_classifier").asText() : null;
        String downloadUrl = params.has("download_url") && !params.get("download_url").isNull()
                ? params.get("download_url").asText() : null;

        DriverResolver.DriverResult result = DriverResolver.resolve(mavenGroup, mavenArtifact, versionCap, classifier, downloadUrl);
        
        ObjectNode resultNode = MAPPER.createObjectNode();
        resultNode.put("jar_path", result.getJarPath());
        resultNode.put("resolved_version", result.getResolvedVersion());
        response.set("result", resultNode);
    }

    private String requiredString(JsonNode params, String key, String defaultValue) throws Exception {
        if (params.has(key) && !params.get(key).isNull()) {
            return params.get(key).asText();
        }
        if (defaultValue != null) {
            return defaultValue;
        }
        throw new Exception("Missing required parameter: " + key);
    }
}
