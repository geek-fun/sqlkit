<script setup lang="ts">
import type { LauncherAction, LauncherFormat, LauncherOptions } from './types'
import { open, save } from '@tauri-apps/plugin-dialog'
import { computed } from 'vue'
import { useI18n } from 'vue-i18n'

import { Button } from '@/components/ui/button'
import { Checkbox } from '@/components/ui/checkbox'
import { Input } from '@/components/ui/input'
import { Label } from '@/components/ui/label'
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from '@/components/ui/select'

const props = defineProps<{
  modelValue: LauncherOptions
  action: LauncherAction
}>()

const emit = defineEmits<{
  (e: 'update:modelValue', value: LauncherOptions): void
}>()

const { t } = useI18n()

const options = computed({
  get: () => props.modelValue,
  set: val => emit('update:modelValue', val),
})

function updateOption<K extends keyof LauncherOptions>(key: K, value: LauncherOptions[K]) {
  options.value = { ...options.value, [key]: value }
}

async function handlePickDestination() {
  const result = await save({
    filters: [{ name: 'Backup', extensions: ['sql', 'csv', 'xlsx'] }],
  })
  if (result)
    updateOption('destination', result)
}

function inferFormat(path: string): LauncherFormat | undefined {
  const lower = path.toLowerCase()
  if (lower.endsWith('.sql'))
    return 'sql'
  if (lower.endsWith('.csv'))
    return 'csv'
  if (lower.endsWith('.xlsx'))
    return 'excel'
  return undefined
}

async function handlePickSourceFile() {
  const result = await open({
    multiple: false,
    filters: [{ name: 'Backup', extensions: ['sql', 'csv', 'xlsx'] }],
  })
  const path = Array.isArray(result) ? result[0] : result
  if (path) {
    updateOption('filePath', path)
    const fmt = inferFormat(path)
    if (fmt)
      updateOption('fileFormat', fmt)
  }
}
</script>

<template>
  <div class="transfer-card transfer-console-section">
    <div class="transfer-console-section-header">
      <span class="i-carbon-settings text-xs" />
      {{ t('transfer.launcher.options') }}
    </div>
    <div class="transfer-console-section-body">
      <!-- Backup / Export -->
      <div v-if="action === 'backup' || action === 'export'" class="space-y-3">
        <div class="space-y-1.5">
          <Label class="transfer-mono-label text-muted-foreground">{{ t('transfer.launcher.format') }}</Label>
          <Select :model-value="options.format || 'sql'" @update:model-value="(v) => updateOption('format', v as LauncherFormat)">
            <SelectTrigger class="w-[200px]">
              <SelectValue />
            </SelectTrigger>
            <SelectContent>
              <SelectItem value="sql">
                SQL Dump (.sql)
              </SelectItem>
              <SelectItem value="csv">
                CSV (.csv)
              </SelectItem>
              <SelectItem value="excel">
                Excel (.xlsx)
              </SelectItem>
            </SelectContent>
          </Select>
        </div>

        <div class="space-y-1.5">
          <Label class="transfer-mono-label text-muted-foreground">{{ t('transfer.launcher.destination') }}</Label>
          <div class="flex gap-2 items-center">
            <Input
              :model-value="options.destination || ''"
              readonly
              class="flex-1"
              placeholder="/path/to/backup.sql"
            />
            <Button variant="outline" size="sm" @click="handlePickDestination">
              {{ t('common.buttons.browse') }}
            </Button>
          </div>
        </div>

        <div class="space-y-1.5">
          <Label class="transfer-mono-label text-muted-foreground">{{ t('transfer.launcher.parallelism') }}</Label>
          <Select :model-value="String(options.parallelism || 4)" @update:model-value="(v) => updateOption('parallelism', Number(v))">
            <SelectTrigger class="w-[100px]">
              <SelectValue />
            </SelectTrigger>
            <SelectContent>
              <SelectItem value="1">
                1 thread
              </SelectItem>
              <SelectItem value="2">
                2 threads
              </SelectItem>
              <SelectItem value="4">
                4 threads
              </SelectItem>
              <SelectItem value="8">
                8 threads
              </SelectItem>
            </SelectContent>
          </Select>
        </div>
      </div>

      <!-- Migrate -->
      <div v-else-if="action === 'migrate'" class="space-y-3">
        <div class="space-y-1.5">
          <label class="flex gap-2 cursor-pointer items-center">
            <Checkbox
              :checked="options.dropTargetFirst ?? false"
              @update:checked="(v) => updateOption('dropTargetFirst', !!v)"
            />
            <span class="text-sm">{{ t('transfer.launcher.dropTargetFirst') }}</span>
          </label>
        </div>

        <div class="space-y-1.5">
          <Label class="transfer-mono-label text-muted-foreground">{{ t('transfer.launcher.parallelism') }}</Label>
          <Select :model-value="String(options.parallelism || 4)" @update:model-value="(v) => updateOption('parallelism', Number(v))">
            <SelectTrigger class="w-[100px]">
              <SelectValue />
            </SelectTrigger>
            <SelectContent>
              <SelectItem value="1">
                1 thread
              </SelectItem>
              <SelectItem value="2">
                2 threads
              </SelectItem>
              <SelectItem value="4">
                4 threads
              </SelectItem>
              <SelectItem value="8">
                8 threads
              </SelectItem>
            </SelectContent>
          </Select>
        </div>
      </div>

      <!-- Restore -->
      <div v-else-if="action === 'restore'" class="space-y-3">
        <div class="space-y-1.5">
          <Label class="transfer-mono-label text-muted-foreground">{{ t('transfer.launcher.fileFormat') }}</Label>
          <Select :model-value="options.fileFormat || 'sql'" @update:model-value="(v) => updateOption('fileFormat', v as LauncherFormat)">
            <SelectTrigger class="w-[200px]">
              <SelectValue />
            </SelectTrigger>
            <SelectContent>
              <SelectItem value="sql">
                SQL Dump (.sql)
              </SelectItem>
              <SelectItem value="csv">
                CSV (.csv)
              </SelectItem>
              <SelectItem value="excel">
                Excel (.xlsx)
              </SelectItem>
            </SelectContent>
          </Select>
        </div>

        <div class="space-y-1.5">
          <Label class="transfer-mono-label text-muted-foreground">{{ t('transfer.launcher.sourceFile') }}</Label>
          <div class="flex gap-2 items-center">
            <Input
              :model-value="options.filePath || ''"
              readonly
              class="flex-1"
              placeholder="/path/to/backup.sql"
            />
            <Button variant="outline" size="sm" @click="handlePickSourceFile">
              {{ t('common.buttons.browse') }}
            </Button>
          </div>
        </div>

        <div v-if="options.fileFormat === 'csv' || options.fileFormat === 'excel'" class="space-y-1.5">
          <Label class="transfer-mono-label text-muted-foreground">{{ t('transfer.launcher.targetTable') }}</Label>
          <Input
            :model-value="options.targetTable || ''"
            class="max-w-md"
            :placeholder="t('transfer.launcher.targetTablePlaceholder')"
            @update:model-value="(v) => updateOption('targetTable', String(v))"
          />
        </div>
      </div>
    </div>
  </div>
</template>

<style scoped>
</style>
