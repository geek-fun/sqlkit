<script setup lang="ts">
import { usePlatform } from '@/composables/usePlatform'
import { useWindowControls } from '@/composables/useWindowControls'

const { isMaximized, minimize, toggleMaximize, close } = useWindowControls()
const { isMac, platformReady } = usePlatform()
</script>

<template>
  <div v-if="platformReady && !isMac" class="mr-2 flex items-stretch">
    <button
      class="inline-flex h-10 w-11 transition-colors items-center justify-center hover:bg-foreground/10"
      @click="minimize"
    >
      <svg width="12" height="12" viewBox="0 0 12 12">
        <rect x="2" y="5.5" width="8" height="1" fill="currentColor" />
      </svg>
    </button>
    <button
      class="inline-flex h-10 w-11 transition-colors items-center justify-center hover:bg-foreground/10"
      @click="toggleMaximize"
    >
      <svg v-if="isMaximized" width="12" height="12" viewBox="0 0 12 12">
        <rect x="1.5" y="3.5" width="7" height="7" rx="0.5" fill="none" stroke="currentColor" stroke-width="1" />
        <rect x="3.5" y="1.5" width="7" height="7" rx="0.5" fill="none" stroke="currentColor" stroke-width="1" />
      </svg>
      <svg v-else width="12" height="12" viewBox="0 0 12 12">
        <rect x="1.5" y="1.5" width="9" height="9" rx="1" fill="none" stroke="currentColor" stroke-width="1" />
      </svg>
    </button>
    <button
      class="inline-flex h-10 w-11 transition-colors items-center justify-center hover:text-white hover:bg-red-500"
      @click="close"
    >
      <svg width="12" height="12" viewBox="0 0 12 12">
        <line x1="2" y1="2" x2="10" y2="10" stroke="currentColor" stroke-width="1.2" />
        <line x1="10" y1="2" x2="2" y2="10" stroke="currentColor" stroke-width="1.2" />
      </svg>
    </button>
  </div>
</template>
