import { parseJwtExpMs } from './sessionExpiry'

/** Session health: a serving token/lease pair, re-login required, or no token. */
export type SessionPhase = 'signed-in' | 'expired' | 'anonymous'

export type SessionPhaseInput = {
  hasAccessToken: boolean
  hasLease: boolean
  /** Last refresh verdict; only `expired` demotes the phase. */
  serverPhase: string | null | undefined
  accessTokenExpMs: number | null
  now?: number
}

export const deriveSessionPhase = ({
  hasAccessToken,
  hasLease,
  serverPhase,
  accessTokenExpMs,
  now = Date.now(),
}: SessionPhaseInput): SessionPhase => {
  if (!hasAccessToken)
    return 'anonymous'
  if (serverPhase === 'expired')
    return 'expired'
  if (accessTokenExpMs !== null && accessTokenExpMs <= now && !hasLease)
    return 'expired'
  return 'signed-in'
}

export const sessionPhaseFromTokens = (
  accessToken: string,
  lease: string,
  serverPhase: string | null | undefined,
  now?: number,
): SessionPhase =>
  deriveSessionPhase({
    hasAccessToken: accessToken.trim() !== '',
    hasLease: lease.trim() !== '',
    serverPhase,
    accessTokenExpMs: accessToken.trim() === '' ? null : parseJwtExpMs(accessToken),
    now,
  })
