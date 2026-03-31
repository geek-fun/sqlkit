<script setup lang="ts">
import { onMounted } from 'vue'
import { RouterView } from 'vue-router'
import AppNotifications from '@/components/ui/notification/AppNotifications.vue'
import { useAppStore } from '@/store/appStore'
import { useAppUpdater } from '@/composables/useAppUpdater'

const appStore = useAppStore()
const { checkForUpdates, downloadAndInstall } = useAppUpdater()

onMounted(() => {
  appStore.setThemeType(appStore.themeType)
  // Auto-check for updates on app launch (silent)
  checkForUpdates(false).then((update) => {
    if (update) {
      // Show update dialog
      const confirmed = window.confirm(
        `New version ${update.version} available!\n\n${update.notes || ''}\n\nDownload and install now? App will restart automatically.`,
      )
      if (confirmed) {
        downloadAndInstall()
      }
    }
  })
})
</script>

<template>
  <RouterView />
  <AppNotifications />
</template>
