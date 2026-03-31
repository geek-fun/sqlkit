<script setup lang="ts">
import { onMounted } from 'vue'
import { RouterView } from 'vue-router'
import AppNotifications from '@/components/ui/notification/AppNotifications.vue'
import { useAppUpdater } from '@/composables/useAppUpdater'
import { useAppStore } from '@/store/appStore'

const appStore = useAppStore()
const { checkForUpdates } = useAppUpdater()

onMounted(() => {
  appStore.setThemeType(appStore.themeType)
  // Auto-check for updates on app launch (silent)
  checkForUpdates(false).then((update) => {
    if (update) {
      // Update info is now available via updateInfo ref
      // The composable handles showing notifications
    }
  })
})
</script>

<template>
  <RouterView />
  <AppNotifications />
</template>
