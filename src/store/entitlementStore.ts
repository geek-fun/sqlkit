import type { EntitlementView, PaidFeature } from '../common'
import { invoke } from '@tauri-apps/api/core'
import { defineStore } from 'pinia'
import {
  ENTITLEMENT_ERROR_TYPE,

  isEntitlementError,

} from '../common'
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
          force,
        })
      }
      catch (e) {
        if (!isEntitlementError(e)) {
          throw e
        }
        this.view = null
      }
    },
    async ensureLocalUltimate(feature: PaidFeature): Promise<boolean> {
      await this.refreshEntitlement(false)
      if (this.isLocalUltimate) {
        return true
      }
      throw Object.assign(new Error(`'${feature}' requires an Ultimate subscription`), {
        status: 403,
        details: feature,
        errorType: ENTITLEMENT_ERROR_TYPE,
      })
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
