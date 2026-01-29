//! API response types for consistent error handling.
//!
//! This module provides standardized response structures similar to REST APIs,
//! enabling detailed error reporting and case-by-case handling on the frontend.

use serde::{Deserialize, Serialize};

/// Standard API response wrapper
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "status", rename_all = "lowercase")]
pub enum ApiResponse<T> {
    /// Success response with data
    Success {
        data: T,
        #[serde(skip_serializing_if = "Option::is_none")]
        message: Option<String>,
    },
    /// Error response with details
    Error {
        error: ApiError,
    },
}

/// Detailed error information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiError {
    /// Error code for programmatic handling
    pub code: String,
    /// Human-readable error message
    pub message: String,
    /// Detailed error description
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<String>,
    /// Specific field that caused the error (for validation errors)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub field: Option<String>,
    /// Additional context or suggestions
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hint: Option<String>,
    /// Original error from database/system
    #[serde(skip_serializing_if = "Option::is_none")]
    pub original_error: Option<String>,
}

/// Error codes for different error types
pub mod error_codes {
    // Connection errors (1xxx)
    pub const CONNECTION_FAILED: &str = "CONNECTION_FAILED";
    pub const CONNECTION_TIMEOUT: &str = "CONNECTION_TIMEOUT";
    pub const CONNECTION_REFUSED: &str = "CONNECTION_REFUSED";
    pub const AUTHENTICATION_FAILED: &str = "AUTHENTICATION_FAILED";
    
    // Query errors (2xxx)
    pub const QUERY_SYNTAX_ERROR: &str = "QUERY_SYNTAX_ERROR";
    pub const QUERY_EXECUTION_ERROR: &str = "QUERY_EXECUTION_ERROR";
    pub const QUERY_TIMEOUT: &str = "QUERY_TIMEOUT";
    
    // Resource errors (3xxx)
    pub const DATABASE_NOT_FOUND: &str = "DATABASE_NOT_FOUND";
    pub const TABLE_NOT_FOUND: &str = "TABLE_NOT_FOUND";
    pub const COLUMN_NOT_FOUND: &str = "COLUMN_NOT_FOUND";
    
    // Permission errors (4xxx)
    pub const PERMISSION_DENIED: &str = "PERMISSION_DENIED";
    pub const INSUFFICIENT_PRIVILEGES: &str = "INSUFFICIENT_PRIVILEGES";
    
    // Data errors (5xxx)
    pub const CONSTRAINT_VIOLATION: &str = "CONSTRAINT_VIOLATION";
    pub const FOREIGN_KEY_VIOLATION: &str = "FOREIGN_KEY_VIOLATION";
    pub const UNIQUE_VIOLATION: &str = "UNIQUE_VIOLATION";
    pub const NOT_NULL_VIOLATION: &str = "NOT_NULL_VIOLATION";
    pub const CHECK_VIOLATION: &str = "CHECK_VIOLATION";
    
    // System errors (9xxx)
    pub const INTERNAL_ERROR: &str = "INTERNAL_ERROR";
    pub const UNSUPPORTED_OPERATION: &str = "UNSUPPORTED_OPERATION";
    pub const INVALID_CONFIGURATION: &str = "INVALID_CONFIGURATION";
}

impl ApiError {
    /// Create a new API error
    pub fn new(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            code: code.into(),
            message: message.into(),
            details: None,
            field: None,
            hint: None,
            original_error: None,
        }
    }

    /// Add detailed description
    pub fn with_details(mut self, details: impl Into<String>) -> Self {
        self.details = Some(details.into());
        self
    }

    /// Add field name for validation errors
    pub fn with_field(mut self, field: impl Into<String>) -> Self {
        self.field = Some(field.into());
        self
    }

    /// Add hint or suggestion
    pub fn with_hint(mut self, hint: impl Into<String>) -> Self {
        self.hint = Some(hint.into());
        self
    }

    /// Add original error message
    pub fn with_original_error(mut self, error: impl Into<String>) -> Self {
        self.original_error = Some(error.into());
        self
    }
}

impl<T> ApiResponse<T> {
    /// Create a success response
    pub fn success(data: T) -> Self {
        Self::Success {
            data,
            message: None,
        }
    }

    /// Create a success response with message
    pub fn success_with_message(data: T, message: impl Into<String>) -> Self {
        Self::Success {
            data,
            message: Some(message.into()),
        }
    }

    /// Create an error response
    pub fn error(error: ApiError) -> Self {
        Self::Error { error }
    }

    /// Create an error response from code and message
    pub fn error_from(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self::Error {
            error: ApiError::new(code, message),
        }
    }
}

/// Convert database errors to API errors with detailed information
pub fn db_error_to_api_error(error: &crate::database::DbError) -> ApiError {
    use crate::database::DbError;
    
    match error {
        DbError::Connection(msg) => {
            ApiError::new(error_codes::CONNECTION_FAILED, "Failed to connect to database")
                .with_details(msg)
                .with_hint("Check your connection settings and ensure the database server is running")
        }
        DbError::Authentication(msg) => {
            ApiError::new(error_codes::AUTHENTICATION_FAILED, "Authentication failed")
                .with_details(msg)
                .with_hint("Verify your username and password")
        }
        DbError::QueryExecution(msg) => {
            parse_query_execution_error(msg)
        }
        DbError::InvalidQuery(msg) => {
            ApiError::new(error_codes::QUERY_SYNTAX_ERROR, "Invalid SQL query")
                .with_details(msg)
                .with_hint("Check your SQL syntax and try again")
        }
        DbError::Timeout(msg) => {
            ApiError::new(error_codes::QUERY_TIMEOUT, "Query execution timed out")
                .with_details(msg)
                .with_hint("Try optimizing your query or increasing the timeout limit")
        }
        DbError::DatabaseNotFound(msg) => {
            ApiError::new(error_codes::DATABASE_NOT_FOUND, "Database not found")
                .with_details(msg)
                .with_hint("Ensure the database exists and you have access to it")
        }
        DbError::TableNotFound(msg) => {
            ApiError::new(error_codes::TABLE_NOT_FOUND, "Table not found")
                .with_details(msg)
                .with_hint("Check the table name and ensure it exists in the database")
        }
        DbError::ColumnNotFound(msg) => {
            ApiError::new(error_codes::COLUMN_NOT_FOUND, "Column not found")
                .with_details(msg)
                .with_hint("Verify the column name matches the table schema")
        }
        DbError::UnsupportedOperation(msg) => {
            ApiError::new(error_codes::UNSUPPORTED_OPERATION, "Operation not supported")
                .with_details(msg)
        }
        DbError::TypeConversion(msg) => {
            ApiError::new(error_codes::INTERNAL_ERROR, "Data type conversion failed")
                .with_details(msg)
                .with_hint("The database returned a value that couldn't be converted to the expected type")
        }
        DbError::PoolError(msg) => {
            ApiError::new(error_codes::CONNECTION_FAILED, "Connection pool error")
                .with_details(msg)
                .with_hint("Try reconnecting to the database")
        }
        DbError::Serialization(msg) => {
            ApiError::new(error_codes::INTERNAL_ERROR, "Data serialization error")
                .with_details(msg)
        }
        DbError::Configuration(msg) => {
            ApiError::new(error_codes::INVALID_CONFIGURATION, "Configuration error")
                .with_details(msg)
                .with_hint("Check your database connection settings")
        }
        DbError::DatabaseError(msg) => {
            ApiError::new(error_codes::INTERNAL_ERROR, "Database error")
                .with_details(msg)
        }
        DbError::Io(err) => {
            ApiError::new(error_codes::INTERNAL_ERROR, "I/O error")
                .with_details(&err.to_string())
        }
        DbError::Other { message, .. } => {
            ApiError::new(error_codes::INTERNAL_ERROR, "Database error")
                .with_details(message)
        }
    }
}

/// Parse query execution errors to provide specific error codes
fn parse_query_execution_error(msg: &str) -> ApiError {
    let msg_lower = msg.to_lowercase();
    
    // Extract structured information from the error message
    let (main_msg, details, hint) = extract_error_parts(msg);
    
    // Check for constraint violations
    if msg_lower.contains("foreign key") || msg_lower.contains("violates foreign key constraint") {
        return build_api_error(
            error_codes::FOREIGN_KEY_VIOLATION,
            "Foreign key constraint violation",
            &main_msg,
            details.as_deref(),
            hint.or(Some("Ensure referenced records exist in the parent table"))
        );
    }
    
    if msg_lower.contains("unique") || msg_lower.contains("duplicate key") {
        return build_api_error(
            error_codes::UNIQUE_VIOLATION,
            "Unique constraint violation",
            &main_msg,
            details.as_deref(),
            hint.or(Some("A record with this value already exists"))
        );
    }
    
    if msg_lower.contains("not null") || msg_lower.contains("null value") {
        return build_api_error(
            error_codes::NOT_NULL_VIOLATION,
            "NOT NULL constraint violation",
            &main_msg,
            details.as_deref(),
            hint.or(Some("This field requires a value"))
        );
    }
    
    if msg_lower.contains("check constraint") {
        return build_api_error(
            error_codes::CHECK_VIOLATION,
            "Check constraint violation",
            &main_msg,
            details.as_deref(),
            hint.or(Some("The value does not satisfy the table constraints"))
        );
    }
    
    // Check for permission errors
    if msg_lower.contains("permission denied") || msg_lower.contains("access denied") {
        return build_api_error(
            error_codes::PERMISSION_DENIED,
            "Permission denied",
            &main_msg,
            details.as_deref(),
            hint.or(Some("You don't have sufficient privileges for this operation"))
        );
    }
    
    // Check for syntax errors
    if msg_lower.contains("syntax error") || msg_lower.contains("parse error") {
        return build_api_error(
            error_codes::QUERY_SYNTAX_ERROR,
            "SQL syntax error",
            &main_msg,
            details.as_deref(),
            hint.or(Some("Check your SQL syntax"))
        );
    }
    
    // Check for resource not found
    if msg_lower.contains("does not exist") || msg_lower.contains("not found") {
        if msg_lower.contains("table") || msg_lower.contains("relation") {
            return build_api_error(
                error_codes::TABLE_NOT_FOUND,
                "Table does not exist",
                &main_msg,
                details.as_deref(),
                hint.or(Some("Ensure the table name is correct and exists in the database"))
            );
        }
        if msg_lower.contains("column") {
            return build_api_error(
                error_codes::COLUMN_NOT_FOUND,
                "Column does not exist",
                &main_msg,
                details.as_deref(),
                hint.or(Some("Check the column name in your query"))
            );
        }
    }
    
    // Check for duplicate objects (indexes, tables, etc.)
    if msg_lower.contains("already exists") || msg_lower.contains("duplicate") {
        return build_api_error(
            error_codes::CONSTRAINT_VIOLATION,
            "Object already exists",
            &main_msg,
            details.as_deref(),
            hint.or(Some("Try dropping the existing object first or use IF NOT EXISTS"))
        );
    }
    
    // Default query execution error
    build_api_error(
        error_codes::QUERY_EXECUTION_ERROR,
        "Query execution failed",
        &main_msg,
        details.as_deref(),
        hint
    )
}

/// Extract structured parts from error message
fn extract_error_parts(msg: &str) -> (String, Option<String>, Option<&str>) {
    let lines: Vec<&str> = msg.lines().collect();
    
    if lines.is_empty() {
        return (msg.to_string(), None, None);
    }
    
    // First line is the main message
    let main_msg = lines[0].to_string();
    
    // Extract details and hint from subsequent lines
    let mut details_parts: Vec<&str> = Vec::new();
    let mut hint = None;
    
    for &line in &lines[1..] {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        
        // Extract hint
        if trimmed.starts_with("[Hint]") {
            hint = Some(trimmed.trim_start_matches("[Hint]").trim());
        } else {
            // Everything else goes into details
            details_parts.push(trimmed);
        }
    }
    
    let details = if details_parts.is_empty() {
        None
    } else {
        Some(details_parts.join("\n"))
    };
    
    (main_msg, details, hint)
}

/// Build ApiError with all fields
fn build_api_error(
    code: &str,
    message: &str,
    main_msg: &str,
    details: Option<&str>,
    hint: Option<&str>
) -> ApiError {
    let mut error = ApiError::new(code, message);
    
    // Combine main error message with any additional details
    let full_details = if let Some(det) = details {
        format!("{}\n\n{}", main_msg, det)
    } else {
        main_msg.to_string()
    };
    
    error = error.with_details(full_details);
    
    if let Some(h) = hint {
        error = error.with_hint(h);
    }
    
    error
}
