<script setup lang="ts">
import type { SslMode } from '@/types/connection'
import { useI18n } from 'vue-i18n'
import { Label } from '@/components/ui/label'
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select'
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
</script>

<template>
  <div class="space-y-1">
    <div class="flex gap-2 items-center">
      <Label for="ssl-mode" class="min-w-24">
        {{ t('ssl.mode.label') }}
      </Label>
      <Select v-model="modelValue">
        <SelectTrigger id="ssl-mode" class="flex-1" :class="{ 'border-destructive': error }">
          <SelectValue :placeholder="t('ssl.mode.placeholder')" />
        </SelectTrigger>
        <SelectContent>
          <SelectItem
            v-for="mode in SSL_MODES"
            :key="mode"
            :value="mode"
          >
            {{ getLabel(mode) }}
          </SelectItem>
        </SelectContent>
      </Select>
    </div>
    <p v-if="error" class="text-sm text-destructive ml-28">
      {{ error }}
    </p>
  </div>
</template>
