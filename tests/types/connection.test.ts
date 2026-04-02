import type { SslConfig } from '@/types/connection'
import {
  hasSslValidationErrors,
  isValidSslMode,
  needsCertFields,
  needsSqlServerOptions,
  sslModeFromBackend,
  sslModeToBackend,
  validateSslConfig,
} from '@/types/connection'

describe('connection types', () => {
  describe('isValidSslMode', () => {
    it('returns true for valid SSL modes', () => {
      expect(isValidSslMode('disable')).toBe(true)
      expect(isValidSslMode('prefer')).toBe(true)
      expect(isValidSslMode('require')).toBe(true)
      expect(isValidSslMode('verify-ca')).toBe(true)
      expect(isValidSslMode('verify-full')).toBe(true)
    })

    it('returns false for invalid SSL modes', () => {
      expect(isValidSslMode('invalid')).toBe(false)
      expect(isValidSslMode('')).toBe(false)
      expect(isValidSslMode(null)).toBe(false)
      expect(isValidSslMode(undefined)).toBe(false)
      expect(isValidSslMode(123)).toBe(false)
    })
  })

  describe('sslModeFromBackend', () => {
    it('returns valid SslConfig for valid modes', () => {
      expect(sslModeFromBackend('prefer')).toEqual({ mode: 'prefer' })
      expect(sslModeFromBackend('require')).toEqual({ mode: 'require' })
      expect(sslModeFromBackend('verify-ca')).toEqual({ mode: 'verify-ca' })
    })

    it('returns disable for null/undefined', () => {
      expect(sslModeFromBackend(null)).toEqual({ mode: 'disable' })
      expect(sslModeFromBackend(undefined)).toEqual({ mode: 'disable' })
    })

    it('returns disable for invalid modes', () => {
      expect(sslModeFromBackend('invalid')).toEqual({ mode: 'disable' })
    })
  })

  describe('sslModeToBackend', () => {
    it('returns mode string from SslConfig', () => {
      expect(sslModeToBackend({ mode: 'prefer' })).toBe('prefer')
      expect(sslModeToBackend({ mode: 'require' })).toBe('require')
      expect(sslModeToBackend({ mode: 'verify-ca', caCertPath: '/path' })).toBe('verify-ca')
    })
  })

  describe('needsCertFields', () => {
    it('returns true for verify-ca with PostgreSQL', () => {
      expect(needsCertFields('PostgreSQL', 'verify-ca')).toBe(true)
    })

    it('returns true for verify-full with MySQL', () => {
      expect(needsCertFields('MySQL', 'verify-full')).toBe(true)
    })

    it('returns false for disable mode', () => {
      expect(needsCertFields('PostgreSQL', 'disable')).toBe(false)
    })

    it('returns false for prefer mode', () => {
      expect(needsCertFields('MySQL', 'prefer')).toBe(false)
    })

    it('returns false for SQLServer', () => {
      expect(needsCertFields('SQLServer', 'verify-ca')).toBe(false)
    })
  })

  describe('needsSqlServerOptions', () => {
    it('returns true for SQLServer with non-disable mode', () => {
      expect(needsSqlServerOptions('SQLServer', 'prefer')).toBe(true)
      expect(needsSqlServerOptions('SQLServer', 'require')).toBe(true)
    })

    it('returns false for SQLServer with disable mode', () => {
      expect(needsSqlServerOptions('SQLServer', 'disable')).toBe(false)
    })

    it('returns false for other databases', () => {
      expect(needsSqlServerOptions('PostgreSQL', 'require')).toBe(false)
      expect(needsSqlServerOptions('MySQL', 'require')).toBe(false)
    })
  })

  describe('validateSslConfig', () => {
    it('returns no errors for disable mode', () => {
      const config: SslConfig = { mode: 'disable' }
      expect(validateSslConfig(config, 'PostgreSQL')).toHaveLength(0)
    })

    it('returns error for invalid SSL mode', () => {
      const config = { mode: 'invalid' as unknown as SslConfig['mode'] }
      const errors = validateSslConfig(config, 'PostgreSQL')
      expect(errors).toHaveLength(1)
      expect(errors[0].field).toBe('mode')
    })

    it('returns error when CA cert missing for verify-ca', () => {
      const config: SslConfig = { mode: 'verify-ca' }
      const errors = validateSslConfig(config, 'PostgreSQL')
      expect(errors.some(e => e.field === 'caCertPath')).toBe(true)
    })

    it('returns no errors when CA cert provided for verify-ca', () => {
      const config: SslConfig = { mode: 'verify-ca', caCertPath: '/path/to/ca.pem' }
      expect(validateSslConfig(config, 'PostgreSQL')).toHaveLength(0)
    })

    it('returns error for path traversal in cert path', () => {
      const config: SslConfig = { mode: 'verify-ca', caCertPath: '/path/../etc/passwd' }
      const errors = validateSslConfig(config, 'PostgreSQL')
      expect(errors.some(e => e.field === 'caCertPath')).toBe(true)
    })

    it('returns error for empty cert path', () => {
      const config: SslConfig = { mode: 'verify-ca', caCertPath: '   ' }
      const errors = validateSslConfig(config, 'PostgreSQL')
      expect(errors.some(e => e.field === 'caCertPath')).toBe(true)
    })

    it('validates client cert path', () => {
      const config: SslConfig = { mode: 'verify-ca', caCertPath: '/ca.pem', clientCertPath: '../bad' }
      const errors = validateSslConfig(config, 'PostgreSQL')
      expect(errors.some(e => e.field === 'clientCertPath')).toBe(true)
    })

    it('validates client key path', () => {
      const config: SslConfig = { mode: 'verify-ca', caCertPath: '/ca.pem', clientKeyPath: '../bad' }
      const errors = validateSslConfig(config, 'PostgreSQL')
      expect(errors.some(e => e.field === 'clientKeyPath')).toBe(true)
    })

    it('does not require cert fields for SQLServer', () => {
      const config: SslConfig = { mode: 'verify-ca' }
      expect(validateSslConfig(config, 'SQLServer')).toHaveLength(0)
    })

    it('returns error for path exceeding max length', () => {
      const longPath = '/'.repeat(5000)
      const config: SslConfig = { mode: 'verify-ca', caCertPath: longPath }
      const errors = validateSslConfig(config, 'PostgreSQL')
      expect(errors.some(e => e.field === 'caCertPath')).toBe(true)
    })
  })

  describe('hasSslValidationErrors', () => {
    it('returns true when errors exist', () => {
      const config: SslConfig = { mode: 'verify-ca' }
      expect(hasSslValidationErrors(config, 'PostgreSQL')).toBe(true)
    })

    it('returns false when no errors', () => {
      const config: SslConfig = { mode: 'disable' }
      expect(hasSslValidationErrors(config, 'PostgreSQL')).toBe(false)
    })
  })
})
