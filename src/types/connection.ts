/**
 * SSL/TLS configuration types for database connections
 */

export type SslMode = 'disable' | 'prefer' | 'require' | 'verify-ca' | 'verify-full'

export type SslConfig = {
  mode: SslMode
  caCertPath?: string
  clientCertPath?: string
  clientKeyPath?: string
  trustServerCertificate?: boolean
}

export type SslValidationError = {
  field: string
  message: string
}

export const SSL_MODES: SslMode[] = [
  'disable',
  'prefer',
  'require',
  'verify-ca',
  'verify-full',
]

export const SSL_MODE_LABELS: Record<SslMode, string> = {
  'disable': 'Disable SSL',
  'prefer': 'Prefer SSL (try encryption)',
  'require': 'Require SSL (always encrypt)',
  'verify-ca': 'Verify CA (verify certificate)',
  'verify-full': 'Verify Full (verify all)',
}

/**
 * Default SSL configuration for each database type
 */
export const DEFAULT_SSL_MODE = 'prefer'

/**
 * Database types that support SSL configuration (lowercase for case-insensitive matching)
 */
export const SSL_SUPPORTED_DATABASES = ['postgresql', 'mysql', 'mariadb', 'sqlserver']

/**
 * Database types that require certificate fields for verify-ca/verify-full
 */
export const CERT_FIELD_DATABASES = ['postgresql', 'mysql', 'mariadb']

const normalizeDbType = (dbType: string): string => dbType.toLowerCase()

/**
 * Check if database type supports SSL
 */
export function isSslSupported(dbType: string): boolean {
  return SSL_SUPPORTED_DATABASES.includes(normalizeDbType(dbType))
}

/**
 * Check if database type needs certificate fields
 */
export function needsCertFields(dbType: string, sslMode: SslMode): boolean {
  return CERT_FIELD_DATABASES.includes(normalizeDbType(dbType))
    && (sslMode === 'verify-ca' || sslMode === 'verify-full')
}

/**
 * Check if database type needs SQL Server specific options
 */
export function needsSqlServerOptions(dbType: string, sslMode: SslMode): boolean {
  return normalizeDbType(dbType) === 'sqlserver' && sslMode !== 'disable'
}

/**
 * Convert legacy boolean SSL to new SslConfig
 */
export function migrateSslBoolean(ssl: boolean): SslConfig {
  return {
    mode: ssl ? 'prefer' : 'disable',
  }
}

/**
 * Convert SslConfig to backend ssl_mode string
 */
export function sslModeToBackend(sslConfig: SslConfig): string {
  return sslConfig.mode
}

/**
 * Parse backend ssl_mode to SslConfig
 */
export function sslModeFromBackend(sslMode: string | null | undefined): SslConfig {
  const mode = (sslMode as SslMode) || 'disable'
  if (SSL_MODES.includes(mode)) {
    return { mode }
  }
  return { mode: 'disable' }
}

export function isValidSslMode(mode: unknown): mode is SslMode {
  return typeof mode === 'string' && SSL_MODES.includes(mode as SslMode)
}

export function validateSslConfig(sslConfig: SslConfig, dbType: string): SslValidationError[] {
  const errors: SslValidationError[] = []

  if (!isValidSslMode(sslConfig.mode)) {
    errors.push({ field: 'mode', message: 'Invalid SSL mode' })
    return errors
  }

  if (sslConfig.mode === 'disable') {
    return errors
  }

  if (needsCertFields(dbType, sslConfig.mode)) {
    if (!sslConfig.caCertPath?.trim()) {
      errors.push({ field: 'caCertPath', message: 'CA certificate path is required for verify-ca/verify-full modes' })
    }
  }

  if (sslConfig.caCertPath && !isValidCertPath(sslConfig.caCertPath)) {
    errors.push({ field: 'caCertPath', message: 'Invalid CA certificate path' })
  }

  if (sslConfig.clientCertPath && !isValidCertPath(sslConfig.clientCertPath)) {
    errors.push({ field: 'clientCertPath', message: 'Invalid client certificate path' })
  }

  if (sslConfig.clientKeyPath && !isValidCertPath(sslConfig.clientKeyPath)) {
    errors.push({ field: 'clientKeyPath', message: 'Invalid client private key path' })
  }

  return errors
}

function isValidCertPath(path: string): boolean {
  if (!path || !path.trim())
    return false
  if (path.includes('..'))
    return false
  if (path.length > 4096)
    return false
  return true
}

export function hasSslValidationErrors(sslConfig: SslConfig, dbType: string): boolean {
  return validateSslConfig(sslConfig, dbType).length > 0
}

/**
 * Column metadata from database table
 */
export type ColumnInfo = {
  name: string
  data_type: string
  nullable: boolean
  default_value?: string
  is_primary_key: boolean
  is_auto_increment: boolean
  max_length?: number
  precision?: number
  scale?: number
  description?: string
  metadata?: Record<string, string>
}
