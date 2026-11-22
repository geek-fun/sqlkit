import type { EntitlementView, SessionPhase } from '../common'
import { invoke } from '@tauri-apps/api/core'
import { defineStore } from 'pinia'
import {
  isEntitlementError,
  isSessionRejected,
  sessionPhaseFromTokens,
} from '../common'
import { useAccountStore } from './accountStore'
import { useDeviceStore } from './deviceStore'

export type PlanState = 'ultimate' | 'community' | 'unknown' | 'checking'

const REFRESH_RETRY_DELAYS_MS = [1500, 3000, 6000]

const sleep = (ms: number) => new Promise(resolve => setTimeout(resolve, ms))

// Terminal failure without a cached view — a synthetic answer so the UI
// lands on Unknown (with its retry CTA) instead of an endless Checking…
function unknownView(
  lastError: string,
  sessionPhase: SessionPhase,
): EntitlementView {
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
    // Last definite session loss; App.vue toasts from it.
    sessionLostNotice: number
    // Consecutive deliberate rejections before the session is torn down.
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
    async clearSessionState(): Promise<void> {
      const accountStore = useAccountStore()
      const lease = accountStore.refreshToken || null
      accountStore.clearAuth()
      this.sessionRejections = 0
      useDeviceStore().$reset()
      void invoke('revoke_session', { refreshToken: lease }).catch(() => {})
      await this.clearCachedEntitlement()
    },
    async handleSessionLoss(): Promise<void> {
      const accountStore = useAccountStore()
      const hadIdentity = accountStore.isLoggedIn
      const clearing = this.clearSessionState()
      if (hadIdentity) {
        this.sessionLostNotice = Date.now()
      }
      await clearing
    },
    async resolveSessionRejection(): Promise<void> {
      this.sessionRejections += 1
      this.view = unknownView('session expired', 'expired')
      if (this.sessionRejections < 2) {
        return
      }
      await this.handleSessionLoss()
    },
    async attemptSessionRecovery(): Promise<'recovered' | 'lost'> {
      const accountStore = useAccountStore()
      const revision = accountStore.sessionRevision
      const lease = accountStore.refreshToken || ''
      if (lease.trim()) {
        try {
          const refreshed = await invoke<{
            access_token: string
            refresh_token: string
          }>('rotate_session_now', { refreshToken: lease })
          if (accountStore.sessionRevision !== revision)
            return 'recovered'
          accountStore.applySessionRefresh(
            lease,
            refreshed.access_token,
            refreshed.refresh_token,
          )
          this.sessionRejections = 0
        }
        catch (e) {
          if (accountStore.sessionRevision !== revision)
            return 'recovered'
          if (accountStore.refreshToken !== lease)
            return 'recovered'
          if (isSessionRejected(e)) {
            await this.resolveSessionRejection()
            return 'lost'
          }
        }
      }
      return (await this.refreshEntitlement(true)) === 'lost'
        ? 'lost'
        : 'recovered'
    },
    async seedFromHandoff(payload: {
      ultimateExpiresAt?: string | null
      versionLockHorizon?: string | null
      cancelScheduledAt?: string | null
    }): Promise<void> {
      const revision = useAccountStore().sessionRevision
      const { ultimateExpiresAt, versionLockHorizon, cancelScheduledAt }
        = payload
      if (!ultimateExpiresAt && !versionLockHorizon)
        return
      try {
        const seeded = await invoke<EntitlementView>('seed_entitlement', {
          ultimateExpiresAt: ultimateExpiresAt ?? null,
          versionLockHorizon: versionLockHorizon ?? null,
          cancelScheduledAt: cancelScheduledAt ?? null,
        })
        if (useAccountStore().sessionRevision === revision)
          this.view = seeded
      }
      catch {
        // best effort — the refresh below still verifies
      }
    },
    // only a real server answer (fetchedAtMs) may surface as a confirmed plan
    async hydrate(): Promise<void> {
      const revision = useAccountStore().sessionRevision
      if (!useAccountStore().isLoggedIn) {
        this.view = null
        return
      }
      const cached = await invoke<EntitlementView>('get_entitlement', {
        tokenPresent: true,
      }).catch(() => null)
      if (useAccountStore().sessionRevision !== revision)
        return
      if (cached?.fetchedAtMs != null)
        this.view = cached
    },
    async refreshEntitlement(force = false): Promise<'ok' | 'lost'> {
      const accountStore = useAccountStore()
      const revision = accountStore.sessionRevision
      if (!accountStore.isLoggedIn) {
        await this.clearCachedEntitlement()
        return 'ok'
      }
      for (let attempt = 0; ; attempt += 1) {
        if (accountStore.sessionRevision !== revision)
          return 'ok'
        const lease = accountStore.refreshToken
        try {
          const view = await invoke<EntitlementView>('refresh_entitlement', {
            token: accountStore.token,
            refreshToken: accountStore.refreshToken || null,
            force,
          })
          if (accountStore.sessionRevision !== revision)
            return 'ok'
          if (view.refreshedSession) {
            const rotated = view.refreshedSession
            accountStore.applySessionRefresh(
              rotated.previous_refresh_token,
              rotated.access_token,
              rotated.refresh_token,
            )
          }
          if (
            view.sessionPhase === 'expired'
            && accountStore.refreshToken !== lease
          ) {
            return 'ok'
          }
          this.view = view
          if (view.sessionPhase === 'expired') {
            await this.handleSessionLoss()
            return 'lost'
          }
          this.sessionRejections = 0
          return 'ok'
        }
        catch (e) {
          if (accountStore.sessionRevision !== revision)
            return 'ok'
          if (isSessionRejected(e)) {
            if (accountStore.refreshToken !== lease)
              return 'ok'
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
                  : typeof e === 'string'
                    ? e
                    : String(e),
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
      this.view = null
      try {
        await invoke<EntitlementView>('clear_entitlement')
      }
      catch {
        // best effort — the local view reset below always applies
      }
    },
  },
})
