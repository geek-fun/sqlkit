<script setup lang="ts">
import type { SslMode } from '@/types/connection'
import { useI18n } from 'vue-i18n'
import { Label } from '@/components/ui/label'
import { SSL_MODE_LABELS, SSL_MODES } from '@/types/connection'

defineProps<{
  error?: string
}>()

const modelValue = defineModel<SslMode>({ required: true })

const { t } = useI18n()

function getLabel(mode: SslMode): string {
  const key = `ssl.mode.${mode.replace('-', '')}`
  const translated = t(key)
  return translated !== key ? translated : SSL_MODE_LABELS[mode]
}

function getDescription(mode: SslMode): string {
  const key = `ssl.modeDesc.${mode.replace('-', '')}`
  const translated = t(key)
  return translated !== key ? translated : ''
}

function selectMode(mode: SslMode) {
  modelValue.value = mode
}
</script>

<template>
  <div class="space-y-1">
    <div class="flex flex-wrap gap-3 items-center">
      <Label class="text-sm min-w-fit whitespace-nowrap">{{ t('ssl.mode.label') }}</Label>
      <div class="p-0.5 border border-border rounded-md bg-muted/40 inline-flex gap-px">
        <button
          v-for="mode in SSL_MODES"
          :key="mode"
          type="button"
          :title="getDescription(mode)"
          class="text-xs leading-none px-2.5 py-1.5 rounded-sm cursor-pointer transition-colors" :class="[
            modelValue === mode
              ? 'bg-background text-primary shadow-sm font-medium'
              : 'text-muted-foreground hover:text-foreground',
          ]"
          @click="selectMode(mode)"
        >
          {{ getLabel(mode) }}
        </button>
      </div>
    </div>
    <p v-if="error" class="text-sm text-destructive">
      {{ error }}
    </p>
  </div>
</template>
