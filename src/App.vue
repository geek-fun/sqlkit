<script setup lang="ts">
import type { UnlistenFn } from '@tauri-apps/api/event'
import { invoke } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'
import { storeToRefs } from 'pinia'
import { onMounted, onUnmounted, watch } from 'vue'
import { RouterView } from 'vue-router'
import { shouldRotateToken } from '@/common'
import DeviceReplaceDialog from '@/components/DeviceReplaceDialog.vue'
import AppNotifications from '@/components/ui/notification/AppNotifications.vue'
import UpdateNotification from '@/components/UpdateNotification.vue'
import UpgradeDialog from '@/components/upgrade/UpgradeDialog.vue'
import { useAppUpdater } from '@/composables/useAppUpdater'
import { useAccountStore } from '@/store/accountStore'
import { useAppStore } from '@/store/appStore'
import { useDeviceStore } from '@/store/deviceStore'
import { useEntitlementStore } from '@/store/entitlementStore'

type AuthPayload = {
  token: string
  username: string
  email: string
  userId?: string
  avatar?: string
  // Entitlement snapshot from the web handoff — lets the app show the plan
  // instantly, before the first subscription round-trip completes.
  ultimateExpiresAt?: string | null
  versionLockHorizon?: string | null
  cancelScheduledAt?: string | null
}

const appStore = useAppStore()
const { themeType } = storeToRefs(appStore)
const accountStore = useAccountStore()
const entitlementStore = useEntitlementStore()
const deviceStore = useDeviceStore()
const { checkForUpdates } = useAppUpdater()

// Apply theme immediately on store hydration (before first render) and whenever it changes
watch(themeType, (newTheme) => {
  appStore.setThemeType(newTheme)
}, { immediate: true })

let unlistenAuth: UnlistenFn | null = null
let unlistenSessionRefresh: UnlistenFn | null = null

// Idempotent: events and the cold-start pull may both deliver the same link.
function handleAuth(payload: AuthPayload) {
  accountStore.setAuth(payload.token, payload.username, payload.email, payload.userId, payload.avatar)
  entitlementStore.seedFromHandoff(payload)
  entitlementStore.refreshEntitlement(true)
  // The deep-linked token comes from a web login with no device attached —
  // register/verify this machine right away.
  deviceStore.ensureActivated(true)
}

// Rotate an expired or near-expiry access token before anything uses it:
// one round trip now beats a 401 on every subsequent call. The returned
// pair is stored directly (not only via the event) so ordering with the
// entitlement refresh below is deterministic.
async function rotateStaleSession() {
  if (!accountStore.refreshToken || !shouldRotateToken(accountStore.token, accountStore.refreshToken))
    return
  try {
    const refreshed = await invoke<{ access_token: string, refresh_token: string }>(
      'rotate_session_now',
      { refreshToken: accountStore.refreshToken },
    )
    accountStore.setToken(refreshed.access_token)
    accountStore.setRefreshToken(refreshed.refresh_token)
  }
  catch {
    // rejected or transient — the reactive 401 paths still recover
  }
}

onMounted(async () => {
  checkForUpdates(false)

  if (accountStore.isLoggedIn) {
    // Cached last success first — the UI must not flash Unknown while the
    // network refresh below is in flight. A stale token is rotated before
    // the refresh so it goes out with a valid bearer.
    await entitlementStore.hydrate()
    await rotateStaleSession()
    entitlementStore.refreshEntitlement(true)
    // geekfun#59: entitlement-activation point — register/verify this device.
    deviceStore.ensureActivated()
  }

  // Listeners must exist before the pending-auth pull below.
  unlistenAuth = await listen<AuthPayload>('sqlkit://auth', ({ payload }) => {
    handleAuth(payload)
  })

  // Transparent session refresh (Rust rotates the lease): keep the frontend
  // copy of both tokens in sync.
  unlistenSessionRefresh = await listen<{ accessToken: string, refreshToken: string }>('session-refreshed', ({ payload }) => {
    accountStore.setToken(payload.accessToken)
    accountStore.setRefreshToken(payload.refreshToken)
  })

  // Cold start: a deep link can arrive before these listeners exist — Rust
  // parks it in pending state; consume it now.
  try {
    const pending = await invoke<AuthPayload | null>('consume_pending_auth')
    if (pending) {
      handleAuth(pending)
    }
  }
  catch {
    // no pending auth is the normal case
  }
})

onUnmounted(() => {
  unlistenAuth?.()
  unlistenSessionRefresh?.()
})
</script>

<template>
  <RouterView />
  <AppNotifications />
  <UpdateNotification />
  <UpgradeDialog />
  <DeviceReplaceDialog />
</template>
