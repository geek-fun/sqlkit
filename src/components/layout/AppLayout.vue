<script setup lang="ts">
import ActivityDrawer from '@/components/transfer/ActivityDrawer.vue'
import AppSidebar from './AppSidebar.vue'
</script>

<template>
  <div class="flex h-screen">
    <AppSidebar />
    <div class="flex-1 relative overflow-hidden">
      <main class="h-full overflow-y-auto">
        <slot />
      </main>
      <ActivityDrawer />
    </div>
  </div>
</template>
