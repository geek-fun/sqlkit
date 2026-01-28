<script setup lang="ts">
import type { QueryResult } from '@/store/tabStore'
import type { ApiError } from '@/types/api'
import { computed, ref } from 'vue'
import { useI18n } from 'vue-i18n'
import { Button } from '@/components/ui/button'
import { formatApiError } from '@/types/api'

interface Props {
  results?: QueryResult | null
  error?: ApiError | string | null
  isExecuting?: boolean
  executionTime?: number
  height?: string
  resizable?: boolean
}

const props = withDefaults(defineProps<Props>(), {
  results: null,
  error: null,
  isExecuting: false,
  executionTime: 0,
  height: '300px',
  resizable: true,
})

const emit = defineEmits<{
  (e: 'close'): void
  (e: 'resize', height: number): void
}>()

const { t } = useI18n()

const formattedError = computed(() => {
  if (!props.error)
    return ''
  if (typeof props.error === 'string')
    return props.error
  return formatApiError(props.error, t)
})

const panelHeight = ref(300)
const isResizing = ref(false)
const startY = ref(0)
const startHeight = ref(0)

const formattedTime = computed(() => {
  if (!props.executionTime)
    return ''
  return props.executionTime < 1000
    ? `${props.executionTime}ms`
    : `${(props.executionTime / 1000).toFixed(2)}s`
})

function formatValue(value: unknown): string {
  return value === null || value === undefined
    ? 'NULL'
    : typeof value === 'object'
      ? JSON.stringify(value)
      : String(value)
}

function isNullValue(value: unknown): boolean {
  return value === null || value === undefined
}

function startResize(e: MouseEvent) {
  if (!props.resizable)
    return
  isResizing.value = true
  startY.value = e.clientY
  startHeight.value = panelHeight.value

  document.addEventListener('mousemove', handleResize)
  document.addEventListener('mouseup', stopResize)
}

function handleResize(e: MouseEvent) {
  if (!isResizing.value)
    return
  const diff = startY.value - e.clientY
  const newHeight = Math.max(100, Math.min(800, startHeight.value + diff))
  panelHeight.value = newHeight
  emit('resize', newHeight)
}

function stopResize() {
  isResizing.value = false
  document.removeEventListener('mousemove', handleResize)
  document.removeEventListener('mouseup', stopResize)
}

function close() {
  emit('close')
}
</script>

<template>
  <div
    class="query-result-panel border-t bg-background flex flex-col"
    :style="{ height: `${panelHeight}px` }"
  >
    <!-- Resize handle -->
    <div
      v-if="resizable"
      class="resize-handle bg-transparent h-1 cursor-ns-resize transition-colors hover:bg-primary/20"
      @mousedown="startResize"
    />

    <!-- Header -->
    <div class="px-3 py-1.5 border-b bg-muted/30 flex items-center justify-between">
      <div class="flex gap-4 items-center">
        <span class="text-sm font-medium">
          {{ t('components.queryResult.title') }}
        </span>
        <div v-if="results" class="text-xs text-muted-foreground flex gap-2 items-center">
          <span v-if="results.columns.length > 0">{{ t('components.queryResult.rows', { count: results.rowCount }) }}</span>
          <span v-else-if="results.rowsAffected !== undefined">{{ t('components.queryResult.rowsAffected', { count: results.rowsAffected }) }}</span>
          <span v-if="formattedTime">• {{ t('components.queryResult.time', { time: formattedTime }) }}</span>
        </div>
      </div>
      <Button
        variant="ghost"
        size="icon"
        class="h-6 w-6"
        :title="t('components.queryResult.close')"
        @click="close"
      >
        <svg xmlns="http://www.w3.org/2000/svg" width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
          <path d="M18 6 6 18" />
          <path d="m6 6 12 12" />
        </svg>
      </Button>
    </div>

    <!-- Content -->
    <div class="flex-1 overflow-auto">
      <!-- Loading state -->
      <div v-if="isExecuting" class="flex h-full items-center justify-center">
        <div class="text-center">
          <svg
            class="text-primary mx-auto mb-2 h-8 w-8 animate-spin"
            xmlns="http://www.w3.org/2000/svg"
            fill="none"
            viewBox="0 0 24 24"
          >
            <circle class="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" stroke-width="4" />
            <path class="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z" />
          </svg>
          <p class="text-sm text-muted-foreground">
            {{ t('components.queryResult.executing') }}
          </p>
        </div>
      </div>

      <!-- Error state -->
      <div v-else-if="error" class="p-4">
        <div class="p-4 border border-red-200 rounded-md bg-red-50 dark:border-red-800 dark:bg-red-900/20">
          <div class="flex gap-3 items-start">
            <svg xmlns="http://www.w3.org/2000/svg" width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" class="text-red-600 mt-0.5 flex-shrink-0 dark:text-red-400">
              <circle cx="12" cy="12" r="10" />
              <line x1="12" x2="12" y1="8" y2="12" />
              <line x1="12" x2="12.01" y1="16" y2="16" />
            </svg>
            <div class="flex-1 space-y-2">
              <div class="space-y-1">
                <p
                  v-for="(line, index) in formattedError.split('\n\n')"
                  :key="index"
                  class="text-sm dark:text-red-200"
                  :class="line.startsWith('💡') ? 'text-amber-700 dark:text-amber-300' : 'text-red-800 dark:text-red-200'"
                >
                  {{ line }}
                </p>
              </div>
            </div>
          </div>
        </div>
      </div>

      <!-- Results table -->
      <div v-else-if="results && results.columns.length > 0" class="h-full overflow-auto">
        <table class="results-table w-full">
          <thead>
            <tr class="border-b">
              <th class="results-table-header text-center w-12">
                #
              </th>
              <th
                v-for="column in results.columns"
                :key="column"
                class="results-table-header text-left whitespace-nowrap"
              >
                {{ column }}
              </th>
            </tr>
          </thead>
          <tbody>
            <tr v-for="(row, index) in results.rows" :key="index" class="border-b hover:bg-muted/50">
              <td class="text-xs text-muted-foreground px-4 py-2 text-center w-12">
                {{ index + 1 }}
              </td>
              <td
                v-for="column in results.columns"
                :key="`${index}-${column}`"
                class="text-sm px-4 py-2 max-w-xs truncate"
                :class="{ 'italic text-muted-foreground': isNullValue(row[column]) }"
                :title="formatValue(row[column])"
              >
                {{ formatValue(row[column]) }}
              </td>
            </tr>
          </tbody>
        </table>
      </div>

      <!-- Empty state (DDL/DML success) -->
      <div v-else-if="results && results.columns.length === 0" class="flex h-full items-center justify-center">
        <div class="text-muted-foreground text-center">
          <svg xmlns="http://www.w3.org/2000/svg" width="32" height="32" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" class="mx-auto mb-2 opacity-50">
            <circle cx="12" cy="12" r="10" />
            <path d="m9 12 2 2 4-4" />
          </svg>
          <p class="text-sm font-medium">
            {{ t('components.queryResult.success') }}
          </p>
          <p v-if="results.rowsAffected !== undefined" class="text-xs mt-1">
            {{ t('components.queryResult.rowsAffected', { count: results.rowsAffected }) }}
          </p>
          <p v-else class="text-xs mt-1">
            {{ t('components.queryResult.commandCompleted') }}
          </p>
        </div>
      </div>

      <!-- No results -->
      <div v-else class="flex h-full items-center justify-center">
        <div class="text-muted-foreground text-center">
          <svg xmlns="http://www.w3.org/2000/svg" width="32" height="32" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" class="mx-auto mb-2 opacity-50">
            <path d="M14.5 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V7.5L14.5 2z" />
            <polyline points="14 2 14 8 20 8" />
          </svg>
          <p class="text-sm">
            {{ t('components.queryResult.noResults') }}
          </p>
        </div>
      </div>
    </div>
  </div>
</template>

<style scoped>
.results-table {
  border-collapse: collapse;
}

.results-table-header {
  position: sticky;
  top: 0;
  z-index: 10;
  background-color: hsl(var(--muted));
  padding: 0.5rem 1rem;
  font-size: 0.875rem;
  font-weight: 500;
}

.resize-handle {
  position: relative;
}

.resize-handle::before {
  content: '';
  position: absolute;
  left: 50%;
  top: 50%;
  transform: translate(-50%, -50%);
  width: 30px;
  height: 3px;
  background-color: currentColor;
  opacity: 0.3;
  border-radius: 2px;
}

.resize-handle:hover::before {
  opacity: 0.6;
}
</style>
