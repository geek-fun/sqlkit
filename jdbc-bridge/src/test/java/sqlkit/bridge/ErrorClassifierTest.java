package sqlkit.bridge;

import org.junit.jupiter.api.Test;
import java.sql.SQLException;
import java.sql.SQLRecoverableException;
import java.sql.SQLSyntaxErrorException;
import java.sql.SQLTimeoutException;
import static org.junit.jupiter.api.Assertions.assertEquals;

class ErrorClassifierTest {
    @Test
    void classifiesConnectionFailuresFromAllJdbcDriversByTypeAndSqlState() {
        assertEquals(ErrorClassifier.ErrorType.NETWORK_ERROR,
                ErrorClassifier.classifyRuntime(new SQLException("session lost", "08006")));
        assertEquals(ErrorClassifier.ErrorType.NETWORK_ERROR,
                ErrorClassifier.classifyRuntime(new SQLRecoverableException("recoverable session")));
        assertEquals(ErrorClassifier.ErrorType.AUTHENTICATION_FAILED,
                ErrorClassifier.classifyRuntime(new SQLException("denied", "28000")));
    }

    @Test
    void preservesSqlErrorsAndTimeoutsInsteadOfTreatingThemAsLostConnections() {
        assertEquals(ErrorClassifier.ErrorType.QUERY_ERROR,
                ErrorClassifier.classifyRuntime(new SQLSyntaxErrorException("column 'connection closed' not found", "42000")));
        assertEquals(ErrorClassifier.ErrorType.TIMEOUT,
                ErrorClassifier.classifyRuntime(new SQLTimeoutException("statement took too long")));
        assertEquals(ErrorClassifier.ErrorType.NETWORK_ERROR,
                ErrorClassifier.classifyRuntime(new SQLException("driver error", "HY000", new java.io.IOException("closed socket"))));
    }
}
