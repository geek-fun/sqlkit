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

    public static ErrorType classify(String errorMessage) {
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
            lower.contains("abstractmethoderror"))
            return ErrorType.VERSION_INCOMPATIBLE;

        // Check auth patterns
        if (lower.contains("ora-01017") || lower.contains("ora-01045") ||
            lower.contains("invalid username/password") || lower.contains("login denied") ||
            lower.contains("sql30082n") || lower.contains("wrong user name or password") ||
            lower.contains("access denied") || lower.contains("authentication failed") ||
            lower.contains("password incorrect") || lower.contains("password does not match"))
            return ErrorType.AUTHENTICATION_FAILED;

        // Check network patterns
        if (lower.contains("connection refused") || lower.contains("connection reset") ||
            lower.contains("unknown host") || lower.contains("unreachable") ||
            lower.contains("network adapter") || lower.contains("connection closed") ||
            lower.contains("communications link"))
            return ErrorType.NETWORK_ERROR;

        // Check timeout
        if (lower.contains("timed out") || lower.contains("timeout"))
            return ErrorType.TIMEOUT;

        return ErrorType.UNKNOWN;
    }
}
