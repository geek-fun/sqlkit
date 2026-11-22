package sqlkit.bridge;

import org.junit.jupiter.api.Test;
import java.lang.reflect.Proxy;
import java.sql.*;
import static org.junit.jupiter.api.Assertions.*;

class QueryExecutorTest {
    private ResultSet result(int type, String name, String text, Object value) {
        ResultSetMetaData meta = (ResultSetMetaData) Proxy.newProxyInstance(
            getClass().getClassLoader(), new Class<?>[]{ResultSetMetaData.class},
            (proxy, method, args) -> switch (method.getName()) {
                case "getColumnType" -> type;
                case "getColumnTypeName" -> name;
                default -> throw new UnsupportedOperationException(method.getName());
            });
        return (ResultSet) Proxy.newProxyInstance(getClass().getClassLoader(),
            new Class<?>[]{ResultSet.class}, (proxy, method, args) -> switch (method.getName()) {
                case "getMetaData" -> meta;
                case "getString" -> text;
                case "getObject" -> {
                    if (value instanceof SQLException e) throw e;
                    yield value;
                }
                default -> throw new UnsupportedOperationException(method.getName());
            });
    }

    @Test void nativeJsonUsesTextAccessor() throws Exception {
        assertEquals("{\"city\":\"杭州\"}", QueryExecutor.getValue(
            result(Types.OTHER, "JSON", "{\"city\":\"杭州\"}", new SQLException("getOracleObject not implemented")), 1));
    }
    @Test void vendorTimestampsNeverReachSerializer() throws Exception {
        for (int type : new int[]{Types.TIMESTAMP, Types.TIMESTAMP_WITH_TIMEZONE}) {
            assertEquals("2026-06-03 12:30:00 +08:00", QueryExecutor.getValue(
                result(type, "TIMESTAMP", "2026-06-03 12:30:00 +08:00", new Object()), 1));
        }
    }
    @Test void unknownVendorValuesUseText() throws Exception {
        assertEquals("P1D", QueryExecutor.getValue(result(Types.OTHER, "INTERVAL", "P1D", new Object()), 1));
    }
    @Test void objectTypesDoNotUseVendorGetString() throws Exception {
        Struct money = new Struct() {
            public String getSQLTypeName() { return "SQLKIT.MONEY_AMOUNT"; }
            public Object[] getAttributes() { return new Object[]{300, "CNY"}; }
            public Object[] getAttributes(java.util.Map<String, Class<?>> map) { return getAttributes(); }
        };
        assertEquals("SQLKIT.MONEY_AMOUNT(300, CNY)", QueryExecutor.getValue(
            result(Types.STRUCT, "MONEY_AMOUNT", null, money), 1));
    }
    @Test void primitiveValuesAndNullsSurvive() throws Exception {
        assertEquals(true, QueryExecutor.getValue(result(Types.BOOLEAN, "BOOLEAN", "true", true), 1));
        assertEquals(42L, QueryExecutor.getValue(result(Types.BIGINT, "BIGINT", "42", 42L), 1));
        assertNull(QueryExecutor.getValue(result(Types.TIMESTAMP, "TIMESTAMP", null, null), 1));
    }
}
