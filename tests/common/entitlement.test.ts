import type { EntitlementView } from '@/common'
import { ENTITLEMENT_ERROR_TYPE, isEntitlementError } from '@/common'

function view(overrides: Partial<EntitlementView> = {}): EntitlementView {
  return {
    ultimateActive: false,
    versionLocked: false,
    localUltimate: false,
    appReleaseDate: '2026-09-12',
    ultimateExpiresAt: null,
    versionLockHorizon: null,
    cancelScheduledAt: null,
    cached: false,
    fetchedAtMs: null,
    lastError: null,
    ...overrides,
  }
}

describe('isEntitlementError', () => {
  it('detects the structured Rust error payload', () => {
    const raw = JSON.stringify({
      status: 403,
      error_type: ENTITLEMENT_ERROR_TYPE,
      message: '\'AI\' requires an Ultimate subscription',
    })
    expect(isEntitlementError(raw)).toBe(true)
  })

  it('detects CustomError-like objects carrying the error type', () => {
    expect(isEntitlementError({ errorType: ENTITLEMENT_ERROR_TYPE })).toBe(true)
  })

  it('detects plain errors mentioning the type', () => {
    expect(isEntitlementError(new Error(ENTITLEMENT_ERROR_TYPE))).toBe(true)
  })

  it('rejects unrelated errors', () => {
    expect(isEntitlementError('DNS_ERROR: cannot resolve')).toBe(false)
    expect(isEntitlementError(null)).toBe(false)
    expect(isEntitlementError(undefined)).toBe(false)
  })
})

describe('entitlement view defaults', () => {
  it('community mode has no entitlements', () => {
    const community = view()
    expect(community.localUltimate).toBe(false)
    expect(community.ultimateActive).toBe(false)
    expect(community.versionLocked).toBe(false)
  })

  it('version lock keeps local features without an active subscription', () => {
    const locked = view({ versionLocked: true, localUltimate: true })
    expect(locked.localUltimate).toBe(true)
    expect(locked.ultimateActive).toBe(false)
  })
})
