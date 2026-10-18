<script setup lang="ts">
import { computed } from 'vue'

type Props = {
  state: 'idle' | 'pending' | 'success' | 'error'
  label: string
  detail?: string
  progress?: number | null
}

const props = defineProps<Props>()
const percent = computed(() => props.progress == null || !Number.isFinite(props.progress)
  ? null
  : Math.min(100, Math.max(0, props.progress)))
</script>

<template>
  <Transition name="settings-feedback">
    <div v-if="state !== 'idle'" class="settings-feedback" :class="`is-${state}`" data-testid="settings-feedback">
      <div class="flex gap-2 min-w-0 items-start" :role="state === 'error' ? 'alert' : 'status'" aria-atomic="true">
        <span
          class="mt-0.5 shrink-0 h-4 w-4"
          :class="state === 'pending' ? 'i-lucide-loader-circle settings-spinner' : state === 'error' ? 'i-lucide-circle-alert' : 'i-lucide-circle-check'"
          aria-hidden="true"
        />
        <div class="flex-1 min-w-0">
          <p class="text-xs leading-5 font-medium">
            {{ label }}
          </p>
          <p v-if="detail" class="settings-detail text-xs leading-5 break-words">
            {{ detail }}
          </p>
        </div>
        <span v-if="state === 'pending' && percent != null" class="text-xs leading-5 shrink-0 tabular-nums">{{ Math.floor(percent) }}%</span>
      </div>
      <div
        v-if="state === 'pending'"
        class="settings-track"
        role="progressbar"
        :aria-label="label"
        :aria-valuenow="percent == null ? undefined : Math.floor(percent)"
        :aria-valuemin="0"
        :aria-valuemax="100"
      >
        <div class="settings-meter" :class="{ 'is-indeterminate': percent == null }" :style="percent == null ? undefined : { transform: `scaleX(${percent / 100})` }" />
      </div>
    </div>
  </Transition>
</template>

<style scoped>
.settings-feedback { color: hsl(var(--foreground)); padding: 10px 12px; border-radius: 6px; background: hsl(var(--primary) / 0.06); }
.settings-feedback.is-success { background: hsl(var(--success-muted)); }
.settings-feedback.is-error { color: hsl(var(--foreground)); background: hsl(var(--destructive) / 0.08); }
.settings-spinner { color: hsl(var(--primary)); }
.is-success [aria-hidden] { color: hsl(var(--success)); }
.is-error [aria-hidden] { color: hsl(var(--destructive)); }
.settings-detail { color: hsl(var(--foreground)); opacity: 0.8; overflow-wrap: anywhere; }
.settings-track { margin-top: 8px; height: 3px; border-radius: 2px; overflow: hidden; background: hsl(var(--primary) / 0.15); }
.settings-meter { height: 100%; background: hsl(var(--primary)); transform-origin: left; transition: transform 180ms ease-out; }
.settings-meter.is-indeterminate { width: 35%; animation: settings-progress 1.4s ease-in-out infinite; }
.settings-spinner { animation: settings-spin 900ms linear infinite; }
.settings-feedback-enter-active, .settings-feedback-leave-active { transition: opacity 150ms ease-out; }
.settings-feedback-enter-from, .settings-feedback-leave-to { opacity: 0; }
@keyframes settings-progress { from { transform: translateX(-110%); } to { transform: translateX(390%); } }
@keyframes settings-spin { to { transform: rotate(360deg); } }
@media (prefers-reduced-motion: reduce) {
  .settings-spinner, .settings-meter.is-indeterminate { animation: none; }
  .settings-meter.is-indeterminate { width: 100%; opacity: 0.6; }
  .settings-meter, .settings-feedback-enter-active, .settings-feedback-leave-active { transition: none; }
}
</style>
