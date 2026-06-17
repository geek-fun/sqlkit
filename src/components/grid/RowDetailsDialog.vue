<script setup lang="ts">
import { useI18n } from 'vue-i18n'
import { Dialog, DialogContent, DialogTitle } from '@/components/ui/dialog'

type Props = {
  open: boolean
  row: Record<string, unknown> | null
  columns: string[]
  columnTypes?: Record<string, string>
}

defineProps<Props>()

const emit = defineEmits<{
  (e: 'update:open', open: boolean): void
  (e: 'close'): void
}>()

const { t } = useI18n()

function isJsonType(type?: string): boolean {
  return !!(type && /^JSONB?/.test(type.toUpperCase()))
}

function formatValue(v: unknown, type?: string): string {
  if (v === null || v === undefined)
    return ''
  if (isJsonType(type)) {
    try {
      return JSON.stringify(typeof v === 'string' ? JSON.parse(v) : v, null, 2)
    }
    catch {
      return typeof v === 'object' ? JSON.stringify(v) : String(v)
    }
  }
  return typeof v === 'object' ? JSON.stringify(v) : String(v)
}

const isNull = (v: unknown): boolean => v === null || v === undefined
</script>

<template>
  <Dialog :open="open" @update:open="emit('update:open', $event)">
    <DialogContent class="flex flex-col max-h-[80vh] max-w-2xl overflow-hidden">
      <DialogTitle>{{ t('components.dataGrid.details.title') }}</DialogTitle>
      <div class="mt-4 flex-1 overflow-y-auto space-y-0 divide-border/30 divide-y">
        <div
          v-for="col in columns"
          :key="col"
          class="px-1 py-2.5 flex gap-4"
        >
          <span class="text-sm text-muted-foreground font-medium flex-shrink-0 w-48">
            {{ col }}
            <span v-if="columnTypes?.[col]" class="text-[10px] font-mono ml-1">({{ columnTypes[col] }})</span>
          </span>
          <span
            class="text-sm font-mono flex-1 whitespace-pre-wrap break-all"
            :class="{ 'italic text-muted-foreground': isNull(row?.[col]) }"
          >
            <template v-if="isNull(row?.[col])">(NULL)</template>
            <template v-else>{{ formatValue(row?.[col], columnTypes?.[col]) }}</template>
          </span>
        </div>
      </div>
    </DialogContent>
  </Dialog>
</template>
