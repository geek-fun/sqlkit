<script setup lang="ts">
import { onMounted } from 'vue'
import { RouterView } from 'vue-router'
import { useAppStore } from '@/store/appStore'

const appStore = useAppStore()

onMounted(() => {
  appStore.setThemeType(appStore.themeType)
})
</script>

<template>
  <RouterView />
</template>
