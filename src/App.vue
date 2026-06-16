<script setup lang="ts">
import type { UnlistenFn } from '@tauri-apps/api/event'
import { listen } from '@tauri-apps/api/event'
import { storeToRefs } from 'pinia'
import { onMounted, onUnmounted, watch } from 'vue'
import { RouterView } from 'vue-router'
import AppNotifications from '@/components/ui/notification/AppNotifications.vue'
import { useAppUpdater } from '@/composables/useAppUpdater'
import { useAccountStore } from '@/store/accountStore'
import { useAppStore } from '@/store/appStore'

const appStore = useAppStore()
const { themeType } = storeToRefs(appStore)
const accountStore = useAccountStore()
const { checkForUpdates } = useAppUpdater()

// Apply theme immediately on store hydration (before first render) and whenever it changes
watch(themeType, (newTheme) => {
  appStore.setThemeType(newTheme)
}, { immediate: true })

let unlistenAuth: UnlistenFn | null = null

onMounted(async () => {
  checkForUpdates(false)

  unlistenAuth = await listen<{
    token: string
    username: string
    email: string
  }>('sqlkit://auth', ({ payload }) => {
    accountStore.setAuth(payload.token, payload.username, payload.email)
  })
})

onUnmounted(() => {
  unlistenAuth?.()
})
</script>

<template>
  <RouterView />
  <AppNotifications />
</template>
