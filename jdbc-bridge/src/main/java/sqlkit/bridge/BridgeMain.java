package sqlkit.bridge;

import com.fasterxml.jackson.databind.JsonNode;
import com.fasterxml.jackson.databind.ObjectMapper;
import com.fasterxml.jackson.databind.node.ObjectNode;

import java.io.*;

/**
 * BridgeMain — entry point for the SQLKit JDBC bridge.
 *
 * Reads JSON-RPC requests from stdin, dispatches them, and writes responses to stdout.
 * Each line on stdin is a complete JSON request. Each response is a single JSON line on stdout.
 */
public class BridgeMain {

    private static final ObjectMapper MAPPER = new ObjectMapper();
    private static final ProtocolHandler HANDLER = new ProtocolHandler(new ConnectionManager());

    public static void main(String[] args) throws Exception {
        // Disable Jackson's FAIL_ON_EMPTY_BEANS for safety
        MAPPER.disable(com.fasterxml.jackson.databind.DeserializationFeature.FAIL_ON_UNKNOWN_PROPERTIES);

        BufferedReader reader = new BufferedReader(new InputStreamReader(System.in));
        PrintWriter writer = new PrintWriter(new OutputStreamWriter(System.out));

        String line;
        while ((line = reader.readLine()) != null) {
            if (line.trim().isEmpty()) {
                continue;
            }
            try {
                JsonNode request = MAPPER.readTree(line);
                JsonNode response = HANDLER.handle(request);
                writer.println(MAPPER.writeValueAsString(response));
                writer.flush();
            } catch (Exception e) {
                // Send error response
                ObjectNode errorResp = MAPPER.createObjectNode();
                errorResp.put("id", -1);
                errorResp.put("error", "Internal error: " + e.getMessage());
                writer.println(MAPPER.writeValueAsString(errorResp));
                writer.flush();
            }
        }
    }
}
