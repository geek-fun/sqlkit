<script setup lang="ts">
import type { ResultPageSize } from '@/composables/useDataGridPagination'
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

const props = defineProps<{
  currentPage: number
  pageCount: number
  pageSize: ResultPageSize
  disabled: boolean
}>()
const emit = defineEmits<{
  pageChange: [page: number]
  pageSizeChange: [size: ResultPageSize]
}>()
const { t } = useI18n()
const pageInput = ref(String(props.currentPage))
const customSize = ref('')
const sizeMenuOpen = ref(false)
const presets = [50, 100, 200, 500, 1000]
const pageSizeLabel = computed(() => props.pageSize === 'all'
  ? t('components.dataGrid.pagination.allRows')
  : t('components.dataGrid.pagination.pageSize', { count: props.pageSize }))
const validCustomSize = computed(() => Number.isSafeInteger(Number(customSize.value)) && Number(customSize.value) > 0)
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

const selectSize = (size: ResultPageSize) => {
  if (props.disabled)
    return
  emit('pageSizeChange', size)
  sizeMenuOpen.value = false
}

const applyCustomSize = () => {
  if (validCustomSize.value)
    selectSize(Number(customSize.value))
}
</script>

<template>
  <nav class="result-pagination flex shrink-0 gap-1 items-center" :aria-label="t('components.dataGrid.pagination.label')">
    <DropdownMenu v-model:open="sizeMenuOpen">
      <DropdownMenuTrigger as-child>
        <Button
          variant="ghost"
          size="sm"
          class="text-xs text-foreground px-2 h-7"
          :disabled="disabled"
          :aria-label="`${t('components.dataGrid.pagination.rowsPerPage')}: ${pageSizeLabel}`"
        >
          {{ pageSizeLabel }}
          <span class="i-lucide-chevron-down ml-1 h-3 w-3" aria-hidden="true" />
        </Button>
      </DropdownMenuTrigger>
      <DropdownMenuContent align="end" class="w-56">
        <DropdownMenuItem v-for="size in presets" :key="size" :aria-current="pageSize === size ? 'true' : undefined" @select="selectSize(size)">
          <span class="flex-1">{{ t('components.dataGrid.pagination.pageSize', { count: size }) }}</span>
          <span v-if="pageSize === size" class="i-lucide-check h-3.5 w-3.5" aria-hidden="true" />
        </DropdownMenuItem>
        <DropdownMenuItem :aria-current="pageSize === 'all' ? 'true' : undefined" @select="selectSize('all')">
          <span class="flex-1">{{ t('components.dataGrid.pagination.showAll') }}</span>
          <span v-if="pageSize === 'all'" class="i-lucide-check h-3.5 w-3.5" aria-hidden="true" />
        </DropdownMenuItem>
        <DropdownMenuSeparator />
        <form class="p-2 flex gap-2" @submit.prevent="applyCustomSize" @keydown="event => { if (event.key !== 'Escape') event.stopPropagation() }" @pointerdown.stop>
          <Input
            v-model="customSize"
            type="number"
            inputmode="numeric"
            min="1"
            :max="Number.MAX_SAFE_INTEGER"
            step="1"
            class="text-xs px-2 border-muted-foreground/80 h-7 min-w-0 tabular-nums"
            :placeholder="t('components.dataGrid.pagination.customSize')"
            :aria-label="t('components.dataGrid.pagination.customSize')"
            :disabled="disabled"
          />
          <Button type="submit" variant="outline" size="sm" class="text-xs px-2 h-7" :disabled="disabled || !validCustomSize">
            {{ t('components.dataGrid.pagination.apply') }}
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
          :aria-label="t('components.dataGrid.pagination.goToPage')"
          @keydown.enter.prevent.stop="applyPage"
          @keydown.esc.stop="pageInput = String(currentPage)"
          @blur="applyPage"
        />
        <span class="mr-1 whitespace-nowrap tabular-nums">{{ t('components.dataGrid.pagination.pageCount', { count: pageCount }) }}</span>
      </div>
      <Button
        variant="ghost"
        size="icon"
        class="text-foreground h-7 w-7"
        :disabled="disabled || action.disabled"
        :aria-label="t(`components.dataGrid.pagination.${action.label}`)"
        :title="t(`components.dataGrid.pagination.${action.label}`)"
        @click="emit('pageChange', action.page)"
      >
        <span :class="action.icon" class="h-3.5 w-3.5" aria-hidden="true" />
      </Button>
    </template>
  </nav>
</template>
