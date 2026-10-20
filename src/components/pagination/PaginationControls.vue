<script setup lang="ts">
import type { PageSize } from '@/composables/pagination'
import { computed, ref, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import { Button } from '@/components/ui/button'
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuSeparator,
  DropdownMenuTrigger,
} from '@/components/ui/dropdown-menu'
import { Input } from '@/components/ui/input'
import { isValidPageSize, PAGE_SIZE_OPTIONS } from '@/composables/pagination'

const props = withDefaults(defineProps<{
  currentPage: number
  pageCount: number
  pageSize: PageSize
  disabled?: boolean
  allowAll?: boolean
  maxPageSize?: number
  database?: string
}>(), { disabled: false, allowAll: false, maxPageSize: Number.MAX_SAFE_INTEGER, database: undefined })
const emit = defineEmits<{
  pageChange: [page: number]
  pageSizeChange: [size: PageSize]
}>()
const { t } = useI18n()
const pageInput = ref(String(props.currentPage))
const customSize = ref('')
const sizeMenuOpen = ref(false)
const presets = computed(() => PAGE_SIZE_OPTIONS.filter(size => size <= props.maxPageSize))
const pageSizeLabel = computed(() => props.pageSize === 'all'
  ? t('components.pagination.allRows')
  : t('components.pagination.pageSize', { count: props.pageSize }))
const validCustomSize = computed(() => isValidPageSize(Number(customSize.value), false, props.maxPageSize))
const navigation = computed(() => [
  { label: 'firstPage', icon: 'i-lucide-chevrons-left', page: 1, disabled: props.currentPage <= 1 },
  { label: 'previousPage', icon: 'i-lucide-chevron-left', page: props.currentPage - 1, disabled: props.currentPage <= 1 },
  { label: 'nextPage', icon: 'i-lucide-chevron-right', page: props.currentPage + 1, disabled: props.currentPage >= props.pageCount },
  { label: 'lastPage', icon: 'i-lucide-chevrons-right', page: props.pageCount, disabled: props.currentPage >= props.pageCount },
])

watch(() => [props.currentPage, props.pageCount, props.disabled], () => {
  pageInput.value = String(props.currentPage)
})

const applyPage = () => {
  const page = Number(pageInput.value)
  if (!props.disabled && Number.isSafeInteger(page) && page > 0 && page !== props.currentPage)
    emit('pageChange', Math.min(page, props.pageCount))
  pageInput.value = String(props.currentPage)
}

const selectSize = (size: PageSize) => {
  if (props.disabled || !isValidPageSize(size, props.allowAll, props.maxPageSize))
    return
  if (size !== props.pageSize)
    emit('pageSizeChange', size)
  sizeMenuOpen.value = false
}

const applyCustomSize = () => {
  if (validCustomSize.value)
    selectSize(Number(customSize.value))
}
</script>

<template>
  <div class="pagination-footer text-xs text-muted-foreground px-3 py-1 border-t bg-muted/20 flex shrink-0 flex-wrap gap-x-3 gap-y-1 min-w-0 items-center justify-between">
    <div v-if="database || $slots.default" class="pagination-summary flex flex-wrap gap-y-1 max-w-full min-w-0 items-center">
      <div v-if="database" class="pagination-context mr-3 pr-3 border-r border-border flex gap-1.5 max-w-full min-w-0 items-baseline">
        <span class="shrink-0">{{ t('pages.queries.status.database') }}:</span>
        <span class="min-w-0 [overflow-wrap:anywhere]">{{ database }}</span>
      </div>
      <div v-if="$slots.default" class="pagination-metrics flex flex-wrap gap-x-3 gap-y-1 max-w-full min-w-0 items-center">
        <slot />
      </div>
    </div>
    <nav class="app-pagination flex flex-wrap gap-1 items-center" :aria-label="t('components.pagination.label')" :aria-busy="disabled">
      <DropdownMenu v-model:open="sizeMenuOpen">
        <DropdownMenuTrigger as-child>
          <Button
            variant="ghost"
            size="sm"
            class="text-xs text-foreground px-2 h-7"
            :disabled="disabled"
            :aria-label="`${t('components.pagination.rowsPerPage')}: ${pageSizeLabel}`"
          >
            {{ pageSizeLabel }}
            <span class="i-lucide-chevron-down ml-1 h-3 w-3" aria-hidden="true" />
          </Button>
        </DropdownMenuTrigger>
        <DropdownMenuContent align="end" class="w-56">
          <DropdownMenuItem v-for="size in presets" :key="size" :aria-current="pageSize === size ? 'true' : undefined" @select="selectSize(size)">
            <span class="flex-1">{{ t('components.pagination.pageSize', { count: size }) }}</span>
            <span v-if="pageSize === size" class="i-lucide-check h-3.5 w-3.5" aria-hidden="true" />
          </DropdownMenuItem>
          <DropdownMenuItem v-if="allowAll" :aria-current="pageSize === 'all' ? 'true' : undefined" @select="selectSize('all')">
            <span class="flex-1">{{ t('components.pagination.showAll') }}</span>
            <span v-if="pageSize === 'all'" class="i-lucide-check h-3.5 w-3.5" aria-hidden="true" />
          </DropdownMenuItem>
          <DropdownMenuSeparator />
          <form class="p-2 flex gap-2" @submit.prevent="applyCustomSize" @keydown="event => { if (event.key !== 'Escape') event.stopPropagation() }" @pointerdown.stop>
            <Input
              v-model="customSize"
              type="number"
              inputmode="numeric"
              min="1"
              :max="maxPageSize"
              step="1"
              class="text-xs px-2 border-muted-foreground/80 h-7 min-w-0 tabular-nums"
              :placeholder="t('components.pagination.customSize')"
              :aria-label="t('components.pagination.customSize')"
              :disabled="disabled"
            />
            <Button type="submit" variant="outline" size="sm" class="text-xs px-2 h-7" :disabled="disabled || !validCustomSize">
              {{ t('components.pagination.apply') }}
            </Button>
          </form>
        </DropdownMenuContent>
      </DropdownMenu>

      <template v-for="(action, index) in navigation" :key="action.label">
        <div v-if="index === 2" class="flex gap-1 items-center">
          <Input
            v-model="pageInput"
            type="number"
            inputmode="numeric"
            min="1"
            :max="pageCount"
            step="1"
            class="text-xs text-foreground px-1 text-center h-7 w-14 tabular-nums"
            :disabled="disabled || pageCount <= 1"
            :aria-label="t('components.pagination.goToPage')"
            @keydown.enter.prevent.stop="applyPage"
            @keydown.esc.stop="pageInput = String(currentPage)"
            @blur="applyPage"
          />
          <span class="mr-1 whitespace-nowrap tabular-nums">{{ t('components.pagination.pageCount', { count: pageCount }) }}</span>
        </div>
        <Button
          variant="ghost"
          size="icon"
          class="text-foreground h-7 w-7"
          :disabled="disabled || action.disabled"
          :aria-label="t(`components.pagination.${action.label}`)"
          :title="t(`components.pagination.${action.label}`)"
          @click="emit('pageChange', action.page)"
        >
          <span :class="action.icon" class="h-3.5 w-3.5" aria-hidden="true" />
        </Button>
      </template>
    </nav>
  </div>
</template>
