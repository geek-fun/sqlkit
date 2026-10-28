export const ENTITLEMENT_ERROR_TYPE = 'ENTITLEMENT_REQUIRED'
export const SESSION_REJECTED_ERROR_TYPE = 'SESSION_REJECTED'
export const SSH_TUNNEL_STRIPPED_ERROR_TYPE = 'SSH_TUNNEL_STRIPPED'

export const UPGRADE_URL = 'https://www.geekfun.club/pricing'

export type PaidFeature
  = | 'ai'
    | 'er_diagram'
    | 'transfer'
    | 'ssh_tunnel'
    | 'mcp_bridge'

export type EntitlementView = {
  refreshedSession?: {
    access_token: string
    refresh_token: string
    previous_refresh_token: string
  }
  ultimateActive: boolean
  versionLocked: boolean
  localUltimate: boolean
  appReleaseDate: string
  ultimateExpiresAt: string | null
  versionLockHorizon: string | null
  cancelScheduledAt: string | null
  cached: boolean
  fetchedAtMs: number | null
  /** Structured session verdict from the last refresh — see common/sessionPhase. */
  sessionPhase: string | null
  lastError: string | null
}

/**
 * Rust commands signal structured outcomes via JSON error strings carrying
 * an `error_type` field.
 */
export function errorCarriesType(error: unknown, type: string): boolean {
  if (!error)
    return false
  if (typeof error === 'object' && 'errorType' in error) {
    return (error as { errorType?: string }).errorType === type
  }
  const raw = typeof error === 'string' ? error : String(error)
  try {
    const parsed = JSON.parse(raw) as { error_type?: string }
    return parsed.error_type === type
  }
  catch {
    return raw.includes(type)
  }
}

export function isEntitlementError(error: unknown): boolean {
  return errorCarriesType(error, ENTITLEMENT_ERROR_TYPE)
}

/**
 * The refresh lease was deliberately rejected server-side (expired /
 * revoked / reuse) — the stored lease is dead and must be dropped.
 */
export function isSessionRejected(error: unknown): boolean {
  return errorCarriesType(error, SESSION_REJECTED_ERROR_TYPE)
}

/**
 * SSH tunnelling is Ultimate-only. A saved tunnel must never dead-end a
 * connection on a free plan: the tunnel is dropped and the database is
 * reached directly instead. The tunnel config itself stays untouched on disk.
 */
export function shouldStripSshTunnel(
  sshTunnelEnabled: boolean | undefined,
  localUltimate: boolean,
): boolean {
  return sshTunnelEnabled === true && !localUltimate
}

/**
 * Tag a connect failure as "the saved SSH tunnel was dropped for this attempt".
 * A fresh Error is returned so the caller's error is left untouched.
 */
export function markSshTunnelStripped(error: Error): Error {
  return Object.assign(new Error(error.message), {
    errorType: SSH_TUNNEL_STRIPPED_ERROR_TYPE,
  })
}

/**
 * The attempt failed only after the saved SSH tunnel was dropped for lack of
 * entitlement — the UI must explain that instead of blaming the database host.
 */
export function isSshTunnelStrippedError(error: unknown): boolean {
  return errorCarriesType(error, SSH_TUNNEL_STRIPPED_ERROR_TYPE)
}
