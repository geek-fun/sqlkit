import type { EntitlementView } from '@/common'
import {
  ENTITLEMENT_ERROR_TYPE,
  isEntitlementError,
  isSessionRejected,
  isSshTunnelStrippedError,
  markSshTunnelStripped,
  SESSION_REJECTED_ERROR_TYPE,
  shouldStripSshTunnel,
} from '@/common'

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
    sessionPhase: null,
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

describe('isSessionRejected', () => {
  it('detects the structured Rust rejection payload', () => {
    const raw = JSON.stringify({
      error_type: SESSION_REJECTED_ERROR_TYPE,
      message: 'session refresh rejected',
    })
    expect(isSessionRejected(raw)).toBe(true)
    expect(isEntitlementError(raw)).toBe(false)
  })

  it('rejects unrelated errors', () => {
    expect(isSessionRejected('network error: timeout')).toBe(false)
    expect(isSessionRejected(undefined)).toBe(false)
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

describe('ssh tunnel entitlement fallback', () => {
  it('strips an enabled tunnel only for non-Ultimate plans', () => {
    expect(shouldStripSshTunnel(true, false)).toBe(true)
    expect(shouldStripSshTunnel(true, true)).toBe(false)
    expect(shouldStripSshTunnel(false, false)).toBe(false)
    expect(shouldStripSshTunnel(undefined, false)).toBe(false)
  })

  it('marks a failure as tunnel-stripped while keeping the original message', () => {
    const failure = markSshTunnelStripped(new Error('Failed to connect: timeout'))
    expect(isSshTunnelStrippedError(failure)).toBe(true)
    expect(failure.message).toBe('Failed to connect: timeout')
  })

  it('never conflates a stripped tunnel with an entitlement rejection', () => {
    const failure = markSshTunnelStripped(new Error('Failed to connect'))
    expect(isEntitlementError(failure)).toBe(false)
    expect(isSshTunnelStrippedError(new Error('Failed to connect'))).toBe(false)
    expect(isSshTunnelStrippedError(undefined)).toBe(false)
  })
})
