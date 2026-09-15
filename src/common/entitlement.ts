export const ENTITLEMENT_ERROR_TYPE = 'ENTITLEMENT_REQUIRED'

export const UPGRADE_URL = 'https://www.geekfun.club/pricing'

export type PaidFeature = 'ai' | 'er_diagram' | 'transfer' | 'ssh_tunnel' | 'mcp_bridge'

export type EntitlementView = {
  ultimateActive: boolean
  versionLocked: boolean
  localUltimate: boolean
  appReleaseDate: string
  ultimateExpiresAt: string | null
  versionLockHorizon: string | null
  cancelScheduledAt: string | null
  cached: boolean
  fetchedAtMs: number | null
  lastError: string | null
}

export function isEntitlementError(error: unknown): boolean {
  if (!error)
    return false
  if (typeof error === 'object' && 'errorType' in error) {
    return (error as { errorType?: string }).errorType === ENTITLEMENT_ERROR_TYPE
  }
  const raw = typeof error === 'string' ? error : String(error)
  try {
    const parsed = JSON.parse(raw) as { error_type?: string }
    return parsed.error_type === ENTITLEMENT_ERROR_TYPE
  }
  catch {
    return raw.includes(ENTITLEMENT_ERROR_TYPE)
  }
}
