<script setup lang="ts">
import type { UnlistenFn } from '@tauri-apps/api/event'
import { listen } from '@tauri-apps/api/event'
import { storeToRefs } from 'pinia'
import { onMounted, onUnmounted, watch } from 'vue'
import { RouterView } from 'vue-router'
import DeviceReplaceDialog from '@/components/DeviceReplaceDialog.vue'
import AppNotifications from '@/components/ui/notification/AppNotifications.vue'
import UpdateNotification from '@/components/UpdateNotification.vue'
import UpgradeDialog from '@/components/upgrade/UpgradeDialog.vue'
import { useAppUpdater } from '@/composables/useAppUpdater'
import { useAccountStore } from '@/store/accountStore'
import { useAppStore } from '@/store/appStore'
import { useDeviceStore } from '@/store/deviceStore'
import { useEntitlementStore } from '@/store/entitlementStore'

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

onMounted(async () => {
  checkForUpdates(false)

  if (accountStore.isLoggedIn) {
    entitlementStore.refreshEntitlement(true)
    // geekfun#59: entitlement-activation point — register/verify this device.
    deviceStore.ensureActivated()
  }

  unlistenAuth = await listen<{
    token: string
    username: string
    email: string
  }>('sqlkit://auth', ({ payload }) => {
    accountStore.setAuth(payload.token, payload.username, payload.email)
    entitlementStore.refreshEntitlement(true)
    // The deep-linked token comes from a web login with no device attached —
    // register/verify this machine right away.
    deviceStore.ensureActivated(true)
  })

  // Transparent session refresh (Rust rotates the lease): keep the frontend
  // copy of the access token in sync.
  unlistenSessionRefresh = await listen<string>('session-refreshed', ({ payload }) => {
    accountStore.setToken(payload)
  })
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
