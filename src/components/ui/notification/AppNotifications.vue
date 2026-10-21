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
          'bg-background border-success-border text-foreground': item.type === 'success',
          'bg-background border-destructive-border text-foreground': item.type === 'error',
          'bg-background border-warning-border text-foreground': item.type === 'warning',
        }"
      >
        <!-- Icon -->
        <div class="mt-0.5 flex-shrink-0">
          <!-- Success -->
          <span v-if="item.type === 'success'" class="i-carbon-checkmark text-success h-4 w-4" />
          <!-- Error -->
          <span v-else-if="item.type === 'error'" class="i-carbon-error text-destructive-text h-4 w-4" />
          <!-- Warning -->
          <span v-else-if="item.type === 'warning'" class="i-carbon-warning text-warning h-4 w-4" />
          <!-- Info -->
          <span v-else class="i-carbon-information text-muted-foreground h-4 w-4" />
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
          <span class="i-carbon-close h-3.5 w-3.5" />
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
