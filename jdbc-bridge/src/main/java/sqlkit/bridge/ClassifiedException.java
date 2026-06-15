package sqlkit.bridge;

public class ClassifiedException extends Exception {
    private final ErrorClassifier.ErrorType errorType;

    public ClassifiedException(String message, ErrorClassifier.ErrorType errorType) {
        super(message);
        this.errorType = errorType;
    }

    public ClassifiedException(String message, Throwable cause, ErrorClassifier.ErrorType errorType) {
        super(message, cause);
        this.errorType = errorType;
    }

    public ErrorClassifier.ErrorType getErrorType() { return errorType; }
}
