package sqlkit.bridge;

import java.util.Arrays;
import java.util.Collections;
import java.util.List;
import java.util.Map;
import java.util.regex.Pattern;

public class ErrorClassifier {
    public enum ErrorType {
        VERSION_INCOMPATIBLE,
        AUTHENTICATION_FAILED,
        NETWORK_ERROR,
        TIMEOUT,
        UNKNOWN
    }

    public static ErrorType classify(Throwable throwable) {
        String errorMessage = buildFullErrorMessage(throwable);
        if (errorMessage == null || errorMessage.isEmpty()) return ErrorType.UNKNOWN;
        String lower = errorMessage.toLowerCase();

        // Check version patterns (all common DB version errors)
        if (lower.contains("ora-28040") || lower.contains("ora-03134") ||
            lower.contains("ora-28002") || lower.contains("ora-00439") ||
            lower.contains("the driver does not support this version") ||
            lower.contains("sql1402n") ||
            lower.contains("driver version is not compatible") ||
            lower.contains("unsupported protocol version") ||
            lower.contains("no matching authentication protocol") ||
            lower.contains("abstractmethoderror") ||
            lower.contains("the driver has not received any packets") ||
            lower.contains("database not found") ||
            lower.contains("catalog error") ||
            lower.contains("the driver could not establish a secure connection"))
            return ErrorType.VERSION_INCOMPATIBLE;

        // Check auth patterns
        if (lower.contains("ora-01017") || lower.contains("ora-01045") ||
            lower.contains("invalid username/password") || lower.contains("login denied") ||
            lower.contains("sql30082n") || lower.contains("wrong user name or password") ||
            lower.contains("access denied") || lower.contains("authentication failed") ||
            lower.contains("password incorrect") || lower.contains("password does not match") ||
            lower.contains("login failed for user") ||
            lower.contains("password authentication failed") ||
            lower.contains("no pg_hba.conf entry") ||
            lower.contains("incorrect username or password") ||
            lower.contains("18456") ||
            lower.contains("18470") ||
            lower.contains("cannot open database") ||
            lower.contains("using password") ||
            lower.contains("login failed") ||
            lower.contains("authentication violation"))
            return ErrorType.AUTHENTICATION_FAILED;

        // Check network patterns
        if (lower.contains("connection refused") || lower.contains("connection reset") ||
            lower.contains("unknown host") || lower.contains("unreachable") ||
            lower.contains("network adapter") || lower.contains("connection closed") ||
            lower.contains("communications link") ||
            lower.contains("communicationsexception") ||
            lower.contains("unable to connect") ||
            lower.contains("could not create connection") ||
            lower.contains("connection rejected") ||
            lower.contains("connection error") ||
            lower.contains("unable to connect to snowflake"))
            return ErrorType.NETWORK_ERROR;

        // Check timeout
        if (lower.contains("timed out") || lower.contains("timeout") ||
            lower.contains("connection timed out") ||
            lower.contains("connect timed out"))
            return ErrorType.TIMEOUT;

        return ErrorType.UNKNOWN;
    }

    private static String buildFullErrorMessage(Throwable throwable) {
        if (throwable == null) return "";
        StringBuilder sb = new StringBuilder();
        Throwable current = throwable;
        while (current != null) {
            if (current.getMessage() != null) {
                if (sb.length() > 0) sb.append(" | ");
                sb.append(current.getMessage());
            }
            current = current.getCause();
        }
        return sb.toString();
    }
}
