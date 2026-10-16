import type { EntitlementView, SessionPhase } from '../common'
import { invoke } from '@tauri-apps/api/core'
import { defineStore } from 'pinia'
import { isEntitlementError, isSessionRejected, sessionPhaseFromTokens } from '../common'
import { useAccountStore } from './accountStore'
import { useDeviceStore } from './deviceStore'

export type PlanState = 'ultimate' | 'community' | 'unknown' | 'checking'

const REFRESH_RETRY_DELAYS_MS = [1500, 3000, 6000]

const sleep = (ms: number) => new Promise(resolve => setTimeout(resolve, ms))

// Terminal failure without a cached view — a synthetic answer so the UI
// lands on Unknown (with its retry CTA) instead of an endless Checking…
function unknownView(lastError: string, sessionPhase: SessionPhase): EntitlementView {
  return {
    ultimateActive: false,
    versionLocked: false,
    localUltimate: false,
    appReleaseDate: '',
    ultimateExpiresAt: null,
    versionLockHorizon: null,
    cancelScheduledAt: null,
    cached: true,
    fetchedAtMs: null,
    sessionPhase,
    lastError,
  }
}

export const useEntitlementStore = defineStore('entitlement', {
  state: (): {
    view: EntitlementView | null
    // Timestamp of the last definite session loss (set by handleSessionLoss);
    // App.vue surfaces the signed-out toast from it.
    sessionLostNotice: number
    // Consecutive deliberate rotation rejections before a session loss is
    // resolved — one rejection lands degraded, two in a row clear.
    sessionRejections: number
  } => ({
    view: null,
    sessionLostNotice: 0,
    sessionRejections: 0,
  }),
  getters: {
    isLocalUltimate: (state): boolean => state.view?.localUltimate ?? false,
    isCloudUltimate: (state): boolean => state.view?.ultimateActive ?? false,
    // 'community' is only claimed when the server answered; a failed or
    // missing check must never masquerade as a confirmed plan.
    planState: (state): PlanState => {
      const accountStore = useAccountStore()
      if (state.view?.localUltimate)
        return 'ultimate' as PlanState
      if (!accountStore.isLoggedIn)
        return 'unknown' as PlanState
      // no server answer yet — the first refresh is still in flight
      if (state.view === null)
        return 'checking' as PlanState
      if (state.view.lastError)
        return 'unknown' as PlanState
      return 'community' as PlanState
    },
    cancelScheduled: (state): boolean => Boolean(state.view?.cancelScheduledAt),
    hasEntitlementError: (state): boolean => Boolean(state.view?.lastError),
    // Single session-health derivation (token facts + the structured server
    // verdict) — every login-CTA / degraded-plan consumer reads this instead
    // of matching lastError strings.
    sessionPhase: (state): SessionPhase => {
      const accountStore = useAccountStore()
      return sessionPhaseFromTokens(
        accountStore.token,
        accountStore.refreshToken,
        state.view?.sessionPhase ?? null,
      )
    },
    sessionExpired(): boolean {
      return this.sessionPhase === 'expired'
    },
  },
  actions: {
    // Shared teardown for an account session — cached entitlements, profile
    // identity, device registration, and a best-effort server-side lease
    // revocation. Local teardown is instant so the UI flips immediately; the
    // revocation is a deliberate no-op on failure and never blocks it.
    async clearSessionState(): Promise<void> {
      const accountStore = useAccountStore()
      const lease = accountStore.refreshToken || null
      await this.clearCachedEntitlement()
      accountStore.clearAuth()
      useDeviceStore().$reset()
      void invoke('revoke_session', { refreshToken: lease }).catch(() => {})
    },
    // The session is definitively unrenewable (the server rejected the lease,
    // or the bearer is dead with no lease to rotate): wipe local state so the
    // UI returns to the logged-out look; App.vue toasts from the notice.
    async handleSessionLoss(): Promise<void> {
      const accountStore = useAccountStore()
      const hadIdentity = accountStore.isLoggedIn
      await this.clearSessionState()
      this.sessionRejections = 0
      if (hadIdentity) {
        this.sessionLostNotice = Date.now()
      }
    },
    // A deliberate rotation rejection is server policy, not transient — but a
    // single one only lands on the degraded login-CTA state; two in a row
    // resolve the whole session (guards against a brief auth-gateway
    // misclassification wiping a healthy account). Success resets the count.
    async resolveSessionRejection(): Promise<void> {
      this.sessionRejections += 1
      this.view = unknownView('session expired', 'expired')
      if (this.sessionRejections < 2) {
        return
      }
      await this.handleSessionLoss()
    },
    // Active recovery for a degraded session: rotate the lease while one
    // exists, then force an entitlement refresh. A deliberate rotation
    // rejection (or a still-expired phase afterwards) is a definite loss and
    // resolves the whole session; transient errors leave the cached state for
    // the caller's retry CTA.
    async attemptSessionRecovery(): Promise<'recovered' | 'lost'> {
      const accountStore = useAccountStore()
      const lease = accountStore.refreshToken || ''
      if (lease.trim()) {
        try {
          const refreshed = await invoke<{ access_token: string, refresh_token: string }>(
            'rotate_session_now',
            { refreshToken: lease },
          )
          accountStore.setToken(refreshed.access_token)
          accountStore.setRefreshToken(refreshed.refresh_token)
          this.sessionRejections = 0
        }
        catch (e) {
          if (isSessionRejected(e)) {
            await this.resolveSessionRejection()
            return 'lost'
          }
          // transient — the entitlement refresh below still tries the
          // existing bearer before falling back to the reactive rotation
        }
      }
      return (await this.refreshEntitlement(true)) === 'lost' ? 'lost' : 'recovered'
    },
    async seedFromHandoff(payload: {
      ultimateExpiresAt?: string | null
      versionLockHorizon?: string | null
      cancelScheduledAt?: string | null
    }): Promise<void> {
      const { ultimateExpiresAt, versionLockHorizon, cancelScheduledAt } = payload
      if (!ultimateExpiresAt && !versionLockHorizon)
        return
      try {
        this.view = await invoke<EntitlementView>('seed_entitlement', {
          ultimateExpiresAt: ultimateExpiresAt ?? null,
          versionLockHorizon: versionLockHorizon ?? null,
          cancelScheduledAt: cancelScheduledAt ?? null,
        })
      }
      catch {
        // best effort — the refresh below still verifies
      }
    },
    // only a real server answer (fetchedAtMs) may surface as a confirmed plan
    async hydrate(): Promise<void> {
      // Entitlements are account-scoped: without a session there is nothing to
      // restore, so a leftover device cache must not be surfaced as a plan.
      if (!useAccountStore().isLoggedIn) {
        this.view = null
        return
      }
      // `tokenPresent` is the session claim: it is this app run's earliest
      // honest "someone is logged in", which is what lets the Rust side
      // (re)start the paid MCP bridge from a cache it may already hold.
      const cached = await invoke<EntitlementView>('get_entitlement', { tokenPresent: true }).catch(() => null)
      if (cached?.fetchedAtMs != null)
        this.view = cached
    },
    async refreshEntitlement(force = false): Promise<'ok' | 'lost'> {
      const accountStore = useAccountStore()
      // No account session ⇒ no entitlements. The Rust cache is offline
      // tolerance for a logged-in session, never a substitute for one —
      // otherwise a cache left behind by a previous session would keep local
      // Ultimate features (Data Studio, SSH tunnel, ER diagram…) unlocked.
      if (!accountStore.isLoggedIn) {
        await this.clearCachedEntitlement()
        return 'ok'
      }
      for (let attempt = 0; ; attempt += 1) {
        try {
          const view = await invoke<EntitlementView>('refresh_entitlement', {
            token: accountStore.token,
            refreshToken: accountStore.refreshToken || null,
            force,
          })
          this.view = view
          if (view.sessionPhase === 'expired') {
            // A dead lease the Rust side could not rotate — never recoverable
            // client-side; resolve the session to the logged-out look.
            await this.handleSessionLoss()
            return 'lost'
          }
          this.sessionRejections = 0
          return 'ok'
        }
        catch (e) {
          if (isSessionRejected(e)) {
            // dead lease — the next login starts clean, and a second
            // consecutive rejection resolves the whole session
            await this.resolveSessionRejection()
            return 'lost'
          }
          if (attempt >= REFRESH_RETRY_DELAYS_MS.length) {
            // terminal: a cached view degrades to itself (offline contract);
            // without one, surface Unknown rather than an endless Checking…
            if (!this.view) {
              this.view = unknownView(
                isEntitlementError(e)
                  ? 'entitlement check failed'
                  : typeof e === 'string' ? e : String(e),
                'signed-in',
              )
            }
            return 'ok'
          }
          await sleep(REFRESH_RETRY_DELAYS_MS[attempt])
        }
      }
    },
    async clearCachedEntitlement(): Promise<void> {
      try {
        await invoke<EntitlementView>('clear_entitlement')
      }
      catch {
        // best effort — the local view reset below always applies
      }
      this.view = null
    },
  },
})
