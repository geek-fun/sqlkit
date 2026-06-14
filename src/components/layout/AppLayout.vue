<script setup lang="ts">
import AppSidebar from './AppSidebar.vue'
</script>

<template>
  <div class="flex h-screen">
    <AppSidebar />
    <main class="flex-1 overflow-y-auto">
      <slot />
    </main>
  </div>
</template>
