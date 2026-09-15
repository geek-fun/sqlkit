import type { EntitlementView } from '../common'
import { invoke } from '@tauri-apps/api/core'
import { defineStore } from 'pinia'
import { isEntitlementError, isSessionRejected } from '../common'
import { useAccountStore } from './accountStore'

export type PlanState = 'ultimate' | 'community'

export const useEntitlementStore = defineStore('entitlement', {
  state: (): { view: EntitlementView | null } => ({
    view: null,
  }),
  getters: {
    isLocalUltimate: (state): boolean => state.view?.localUltimate ?? false,
    isCloudUltimate: (state): boolean => state.view?.ultimateActive ?? false,
    planState: (state): PlanState => (state.view?.localUltimate ? 'ultimate' : 'community'),
    cancelScheduled: (state): boolean => Boolean(state.view?.cancelScheduledAt),
    hasEntitlementError: (state): boolean => Boolean(state.view?.lastError),
  },
  actions: {
    async refreshEntitlement(force = false): Promise<void> {
      const accountStore = useAccountStore()
      try {
        this.view = await invoke<EntitlementView>('refresh_entitlement', {
          token: accountStore.token,
          refreshToken: accountStore.refreshToken || null,
          force,
        })
      }
      catch (e) {
        if (isSessionRejected(e)) {
          // The lease is dead server-side — drop it so the next login starts
          // clean instead of presenting a revoked token.
          accountStore.setRefreshToken('')
        }
        if (!isEntitlementError(e) && !isSessionRejected(e)) {
          throw e
        }
        this.view = null
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
