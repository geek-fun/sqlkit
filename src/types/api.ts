/**
 * API response types matching the Rust backend structure
 */

export interface ApiSuccessResponse<T> {
  status: 'success'
  data: T
  message?: string
}

export interface ApiErrorResponse {
  status: 'error'
  error: ApiError
}

export type ApiResponse<T> = ApiSuccessResponse<T> | ApiErrorResponse

export interface ApiError {
  code: string
  message: string
  details?: string
  field?: string
  hint?: string
  original_error?: string
}

/**
 * Error codes matching backend
 */
export const ErrorCodes = {
  // Connection errors (1xxx)
  CONNECTION_FAILED: 'CONNECTION_FAILED',
  CONNECTION_TIMEOUT: 'CONNECTION_TIMEOUT',
  CONNECTION_REFUSED: 'CONNECTION_REFUSED',
  AUTHENTICATION_FAILED: 'AUTHENTICATION_FAILED',

  // Query errors (2xxx)
  QUERY_SYNTAX_ERROR: 'QUERY_SYNTAX_ERROR',
  QUERY_EXECUTION_ERROR: 'QUERY_EXECUTION_ERROR',
  QUERY_TIMEOUT: 'QUERY_TIMEOUT',

  // Resource errors (3xxx)
  DATABASE_NOT_FOUND: 'DATABASE_NOT_FOUND',
  TABLE_NOT_FOUND: 'TABLE_NOT_FOUND',
  COLUMN_NOT_FOUND: 'COLUMN_NOT_FOUND',

  // Permission errors (4xxx)
  PERMISSION_DENIED: 'PERMISSION_DENIED',
  INSUFFICIENT_PRIVILEGES: 'INSUFFICIENT_PRIVILEGES',

  // Data errors (5xxx)
  CONSTRAINT_VIOLATION: 'CONSTRAINT_VIOLATION',
  FOREIGN_KEY_VIOLATION: 'FOREIGN_KEY_VIOLATION',
  UNIQUE_VIOLATION: 'UNIQUE_VIOLATION',
  NOT_NULL_VIOLATION: 'NOT_NULL_VIOLATION',
  CHECK_VIOLATION: 'CHECK_VIOLATION',

  // System errors (9xxx)
  INTERNAL_ERROR: 'INTERNAL_ERROR',
  UNSUPPORTED_OPERATION: 'UNSUPPORTED_OPERATION',
  INVALID_CONFIGURATION: 'INVALID_CONFIGURATION',
} as const

/**
 * Type guard to check if response is an error
 */
export function isApiError<T>(response: ApiResponse<T>): response is ApiErrorResponse {
  return response.status === 'error'
}

/**
 * Type guard to check if response is successful
 */
export function isApiSuccess<T>(response: ApiResponse<T>): response is ApiSuccessResponse<T> {
  return response.status === 'success'
}

/**
 * Extract data from API response or throw error
 */
export function unwrapApiResponse<T>(response: ApiResponse<T>): T {
  if (isApiError(response)) {
    throw new ApiErrorException(response.error)
  }
  return response.data
}

/**
 * Custom error class for API errors
 */
export class ApiErrorException extends Error {
  constructor(public apiError: ApiError) {
    super(apiError.message)
    this.name = 'ApiErrorException'
  }

  /**
   * Get formatted error message for display
   * @param t - i18n translation function
   */
  getFormattedMessage(t?: (key: string) => string): string {
    if (t) {
      return formatApiError(this.apiError, t)
    }

    const parts = [this.apiError.message]

    if (this.apiError.details) {
      parts.push(this.apiError.details)
    }

    if (this.apiError.hint) {
      parts.push(`💡 ${this.apiError.hint}`)
    }

    return parts.join('\n\n')
  }

  /**
   * Get error severity level
   */
  getSeverity(): 'error' | 'warning' | 'info' {
    // Permission and authentication errors are warnings
    if (this.apiError.code.includes('PERMISSION') || this.apiError.code.includes('AUTHENTICATION')) {
      return 'warning'
    }

    // Resource not found can be info
    if (this.apiError.code.includes('NOT_FOUND')) {
      return 'info'
    }

    return 'error'
  }

  /**
   * Check if error is retryable
   */
  isRetryable(): boolean {
    const retryableCodes = [
      ErrorCodes.CONNECTION_TIMEOUT,
      ErrorCodes.QUERY_TIMEOUT,
      ErrorCodes.CONNECTION_REFUSED,
    ]
    return retryableCodes.includes(this.apiError.code as any)
  }
}

/**
 * Format API error for display with i18n support
 */
export function formatApiError(error: ApiError, t: (key: string) => string): string {
  const parts: string[] = []

  // Translate error title
  const titleKey = `errors.codes.${error.code}`
  const title = t(titleKey) !== titleKey ? t(titleKey) : error.message
  parts.push(title)

  // Add details if available
  if (error.details) {
    parts.push(error.details)
  }

  // Add translated hint if available
  if (error.hint) {
    const hintKey = `errors.hints.${error.code}`
    const hint = t(hintKey) !== hintKey ? t(hintKey) : error.hint
    parts.push(`💡 ${hint}`)
  }

  return parts.join('\n\n')
}

/**
 * Get user-friendly error title with i18n support
 */
export function getErrorTitle(error: ApiError, t: (key: string) => string): string {
  const titleKey = `errors.titles.${error.code}`
  const translated = t(titleKey)
  return translated !== titleKey ? translated : error.message
}
