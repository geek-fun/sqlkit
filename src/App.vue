<script setup lang="ts">
import { invoke } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'
import { storeToRefs } from 'pinia'
import { onMounted, onUnmounted, watch } from 'vue'
import { RouterView } from 'vue-router'
import { createSessionLifecycle } from '@/common'
import DeviceReplaceDialog from '@/components/DeviceReplaceDialog.vue'
import AppNotifications from '@/components/ui/notification/AppNotifications.vue'
import UpdateNotification from '@/components/UpdateNotification.vue'
import UpgradeDialog from '@/components/upgrade/UpgradeDialog.vue'
import { useAppUpdater } from '@/composables/useAppUpdater'
import { toast } from '@/composables/useNotifications'
import { useAccountStore } from '@/store/accountStore'
import { useAppStore } from '@/store/appStore'
import { useDeviceStore } from '@/store/deviceStore'
import { useEntitlementStore } from '@/store/entitlementStore'
import { lang } from './lang'

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

const sessionLifecycle = createSessionLifecycle({
  read: () => ({ token: accountStore.token, refreshToken: accountStore.refreshToken, revision: accountStore.sessionRevision }),
  authenticate: (payload) => {
    entitlementStore.view = null
    entitlementStore.sessionRejections = 0
    deviceStore.$reset()
    accountStore.setAuth(payload.token, payload.username ?? '', payload.email ?? '', payload.userId ?? '', payload.avatar ?? '')
  },
  applyRotation: payload => accountStore.applySessionRefresh(payload.previousRefreshToken, payload.accessToken, payload.refreshToken),
  seed: payload => entitlementStore.seedFromHandoff(payload),
  hydrate: () => entitlementStore.hydrate(),
  refresh: () => entitlementStore.refreshEntitlement(true),
  activate: force => deviceStore.ensureActivated(force),
  clearCache: () => entitlementStore.clearCachedEntitlement(),
  rotate: refreshToken => invoke('rotate_session_now', { refreshToken }),
  listenAuth: handler => listen('sqlkit://auth', event => handler(event.payload as Parameters<typeof handler>[0])),
  listenRotation: handler => listen('session-refreshed', event => handler(event.payload as Parameters<typeof handler>[0])),
  consumeAuth: () => invoke('consume_pending_auth'),
})

onMounted(async () => {
  checkForUpdates(false)

  await sessionLifecycle.start()
})

onUnmounted(() => {
  sessionLifecycle.dispose()
})

// The store resolves the loss; surface the single notice.
watch(
  () => entitlementStore.sessionLostNotice,
  (noticeAt) => {
    if (!noticeAt)
      return
    toast.warning(lang.global.t('plan.section.sessionLostSignedOut'))
  },
)
</script>

<template>
  <RouterView />
  <AppNotifications />
  <UpdateNotification />
  <UpgradeDialog />
  <DeviceReplaceDialog />
</template>
