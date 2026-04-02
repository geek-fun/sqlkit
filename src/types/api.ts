export type ApiSuccessResponse<T> = {
  status: 'success'
  data: T
  message?: string
}

export type ApiErrorResponse = {
  status: 'error'
  error: ApiError
}

export type ApiResponse<T> = ApiSuccessResponse<T> | ApiErrorResponse

export type ApiError = {
  code: string
  message: string
  details?: string
  field?: string
  hint?: string
  original_error?: string
}

export const ErrorCodes = {
  CONNECTION_FAILED: 'CONNECTION_FAILED',
  CONNECTION_TIMEOUT: 'CONNECTION_TIMEOUT',
  CONNECTION_REFUSED: 'CONNECTION_REFUSED',
  AUTHENTICATION_FAILED: 'AUTHENTICATION_FAILED',
  QUERY_SYNTAX_ERROR: 'QUERY_SYNTAX_ERROR',
  QUERY_EXECUTION_ERROR: 'QUERY_EXECUTION_ERROR',
  QUERY_TIMEOUT: 'QUERY_TIMEOUT',
  DATABASE_NOT_FOUND: 'DATABASE_NOT_FOUND',
  TABLE_NOT_FOUND: 'TABLE_NOT_FOUND',
  COLUMN_NOT_FOUND: 'COLUMN_NOT_FOUND',
  PERMISSION_DENIED: 'PERMISSION_DENIED',
  INSUFFICIENT_PRIVILEGES: 'INSUFFICIENT_PRIVILEGES',
  CONSTRAINT_VIOLATION: 'CONSTRAINT_VIOLATION',
  FOREIGN_KEY_VIOLATION: 'FOREIGN_KEY_VIOLATION',
  UNIQUE_VIOLATION: 'UNIQUE_VIOLATION',
  NOT_NULL_VIOLATION: 'NOT_NULL_VIOLATION',
  CHECK_VIOLATION: 'CHECK_VIOLATION',
  INTERNAL_ERROR: 'INTERNAL_ERROR',
  UNSUPPORTED_OPERATION: 'UNSUPPORTED_OPERATION',
  INVALID_CONFIGURATION: 'INVALID_CONFIGURATION',
} as const

export function isApiError<T>(response: ApiResponse<T>): response is ApiErrorResponse {
  return response.status === 'error'
}

export function isApiSuccess<T>(response: ApiResponse<T>): response is ApiSuccessResponse<T> {
  return response.status === 'success'
}

export function unwrapApiResponse<T>(response: ApiResponse<T>): T {
  if (isApiError(response)) {
    throw new ApiErrorException(response.error)
  }
  return response.data
}

export class ApiErrorException extends Error {
  constructor(public apiError: ApiError) {
    super(apiError.message)
    this.name = 'ApiErrorException'
  }

  getFormattedMessage(t?: (key: string) => string): string {
    if (t) {
      return formatApiError(this.apiError, t)
    }

    const parts: string[] = [this.apiError.message]

    if (this.apiError.details) {
      parts.push(this.apiError.details)
    }

    if (this.apiError.hint) {
      parts.push(`💡 ${this.apiError.hint}`)
    }

    return parts.join('\n\n')
  }

  getSeverity(): 'error' | 'warning' | 'info' {
    if (this.apiError.code.includes('PERMISSION') || this.apiError.code.includes('AUTHENTICATION')) {
      return 'warning'
    }

    if (this.apiError.code.includes('NOT_FOUND')) {
      return 'info'
    }

    return 'error'
  }

  isRetryable(): boolean {
    const retryableCodes = [
      ErrorCodes.CONNECTION_TIMEOUT,
      ErrorCodes.QUERY_TIMEOUT,
      ErrorCodes.CONNECTION_REFUSED,
    ]
    return retryableCodes.includes(this.apiError.code as any)
  }
}

export function formatApiError(error: ApiError, t: (key: string) => string): string {
  const parts: string[] = []

  const titleKey = `errors.codes.${error.code}`
  const title = t(titleKey) !== titleKey ? t(titleKey) : error.message
  parts.push(title)

  if (error.details) {
    parts.push(error.details)
  }

  if (error.hint) {
    const hintKey = `errors.hints.${error.code}`
    const hint = t(hintKey) !== hintKey ? t(hintKey) : error.hint
    parts.push(`💡 ${hint}`)
  }

  return parts.join('\n\n')
}

export function getErrorTitle(error: ApiError, t: (key: string) => string): string {
  const titleKey = `errors.titles.${error.code}`
  const translated = t(titleKey)
  return translated !== titleKey ? translated : error.message
}
