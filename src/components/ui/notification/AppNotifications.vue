<script setup lang="ts">
import { useI18n } from 'vue-i18n'
import { useNotifications } from '@/composables/useNotifications'

const { t } = useI18n()
const { toasts, dismiss } = useNotifications()
</script>

<template>
  <div class="flex flex-col gap-2 pointer-events-none bottom-4 right-4 fixed z-[9999]" style="max-width: 380px;">
    <transition-group name="toast">
      <div
        v-for="item in toasts"
        :key="item.id"
        class="text-sm px-4 py-3 border rounded-lg flex gap-3 pointer-events-auto shadow-lg items-start"
        :class="{
          'bg-background border-border text-foreground': item.type === 'info',
          'bg-background border-green-500/40 text-foreground': item.type === 'success',
          'bg-background border-destructive/40 text-foreground': item.type === 'error',
        }"
      >
        <!-- Icon -->
        <div class="mt-0.5 flex-shrink-0">
          <!-- Success -->
          <svg v-if="item.type === 'success'" xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" class="text-green-500">
            <path d="M22 11.08V12a10 10 0 1 1-5.93-9.14" />
            <polyline points="22 4 12 14.01 9 11.01" />
          </svg>
          <!-- Error -->
          <svg v-else-if="item.type === 'error'" xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" class="text-destructive">
            <circle cx="12" cy="12" r="10" />
            <line x1="12" x2="12" y1="8" y2="12" />
            <line x1="12" x2="12.01" y1="16" y2="16" />
          </svg>
          <!-- Info -->
          <svg v-else xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" class="text-primary">
            <circle cx="12" cy="12" r="10" />
            <line x1="12" x2="12" y1="8" y2="12" />
            <line x1="12" x2="12.01" y1="16" y2="16" />
          </svg>
        </div>

        <!-- Content -->
        <div class="flex-1 min-w-0">
          <p class="leading-snug font-medium">
            {{ item.title }}
          </p>
          <p v-if="item.description" class="text-xs text-muted-foreground leading-relaxed mt-0.5 break-all">
            {{ item.description }}
          </p>
        </div>

        <!-- Dismiss -->
        <button
          class="text-muted-foreground flex-shrink-0 transition-colors hover:text-foreground"
          :aria-label="t('common.buttons.dismiss')"
          @click="dismiss(item.id)"
        >
          <svg xmlns="http://www.w3.org/2000/svg" width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
            <path d="M18 6 6 18" /><path d="m6 6 12 12" />
          </svg>
        </button>
      </div>
    </transition-group>
  </div>
</template>

<style scoped>
.toast-enter-active,
.toast-leave-active {
  transition: all 0.25s ease;
}
.toast-enter-from {
  opacity: 0;
  transform: translateX(100%);
}
.toast-leave-to {
  opacity: 0;
  transform: translateX(100%);
}
</style>
