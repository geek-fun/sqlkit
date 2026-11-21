<script setup lang="ts">
import { invoke } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'
import { storeToRefs } from 'pinia'
import { onMounted, onUnmounted, watch } from 'vue'
import { RouterView } from 'vue-router'
import {
  cancelAvatarRenewal,
  configureAvatarRefresh,
  createSessionLifecycle,
  ensureFreshAvatar,
  parseJwtStringClaim,
  resolveAvatarExpiresSec,
} from '@/common'
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
import { currentBcp47Locale, lang } from './lang'

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

// Device payloads report the effective UI locale (geek-fun/sqlkit#209) —
// pushed before session start and refreshed whenever the language changes.
const pushUiLocale = () => invoke('set_ui_locale', { locale: currentBcp47Locale() })
watch(() => lang.global.locale.value, pushUiLocale)

// The avatar is a signed, expiring credential — renew it through the same
// single-flight path the `<img>` error handler and the renewal timer use.
configureAvatarRefresh({
  read: () => ({ url: accountStore.avatar, expiresAtSec: accountStore.avatarExpiresAt }),
  apply: (url, expiresAtSec) => accountStore.setAvatar(url, expiresAtSec),
  fetch: async () => {
    // Handoffs from older console builds can omit userId — the JWT carries
    // the stable account id.
    const userId = accountStore.userId || parseJwtStringClaim(accountStore.token, 'id') || ''
    if (!accountStore.token || !userId)
      throw new Error('avatar refresh requires an authenticated session')
    const result = await invoke<{ avatarUrl?: string, expiresAt?: number | null }>(
      'refresh_avatar',
      { token: accountStore.token, userId },
    )
    return { avatarUrl: result.avatarUrl ?? '', expiresAtSec: result.expiresAt ?? null }
  },
})

const sessionLifecycle = createSessionLifecycle({
  read: () => ({ token: accountStore.token, refreshToken: accountStore.refreshToken, revision: accountStore.sessionRevision }),
  authenticate: (payload) => {
    entitlementStore.view = null
    entitlementStore.sessionRejections = 0
    deviceStore.$reset()
    const avatar = payload.avatar ?? ''
    accountStore.setAuth(
      payload.token,
      payload.username ?? '',
      payload.email ?? '',
      payload.userId ?? '',
      avatar,
      resolveAvatarExpiresSec(avatar, payload.avatarExpiresAt ?? 0) ?? 0,
    )
  },
  applyRotation: payload => accountStore.applySessionRefresh(payload.previousRefreshToken, payload.accessToken, payload.refreshToken),
  seed: payload => entitlementStore.seedFromHandoff(payload),
  hydrate: () => entitlementStore.hydrate(),
  refresh: () => entitlementStore.refreshEntitlement(true),
  ensureFreshAvatar: () => ensureFreshAvatar(),
  activate: force => deviceStore.ensureActivated(force),
  clearCache: () => entitlementStore.clearCachedEntitlement(),
  rotate: refreshToken => invoke('rotate_session_now', { refreshToken }),
  listenAuth: handler => listen('sqlkit://auth', event => handler(event.payload as Parameters<typeof handler>[0])),
  listenRotation: handler => listen('session-refreshed', event => handler(event.payload as Parameters<typeof handler>[0])),
  consumeAuth: () => invoke('consume_pending_auth'),
})

onMounted(async () => {
  checkForUpdates(false)

  await pushUiLocale()
  await sessionLifecycle.start()
})

onUnmounted(() => {
  sessionLifecycle.dispose()
  cancelAvatarRenewal()
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
