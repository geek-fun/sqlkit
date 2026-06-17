<script setup lang="ts">
import type { ColumnFilter, ColumnHeaderContextMenuEmits, SortDirection } from '@/types/grid'
import { onClickOutside } from '@vueuse/core'
import { ref } from 'vue'
import { useI18n } from 'vue-i18n'
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuSeparator,
  DropdownMenuTrigger,
} from '@/components/ui/dropdown-menu'

const props = defineProps<{
  show: boolean
  x: number
  y: number
  column: string
  columnType?: string
  hasActiveFilter: boolean
  currentSortDirection: SortDirection | null
}>()

const emit = defineEmits<ColumnHeaderContextMenuEmits>()

const { t } = useI18n()

const menuRef = ref<HTMLElement>()

onClickOutside(menuRef, () => emit('close'))

const adjustedX = Math.min(props.x, window.innerWidth - 240)
const adjustedY = Math.min(props.y, window.innerHeight - 420)

function promptFilterValue(label: string): string | null {
  // eslint-disable-next-line no-alert
  return window.prompt(`${label} — ${props.column}`)
}

function handleFilterWithPrompt(operator: ColumnFilter['operator']) {
  if (operator === 'between') {
    const v1 = promptFilterValue(t('components.dataGrid.filter.between'))
    if (v1 === null)
      return
    const v2 = promptFilterValue(t('components.dataGrid.filter.between'))
    if (v2 === null)
      return
    emit('filter', { column: props.column, operator, value: v1, value2: v2 })
  }
  else {
    const v = promptFilterValue(t('components.dataGrid.filter.byValue'))
    if (v === null)
      return
    emit('filter', { column: props.column, operator, value: v })
  }
  emit('close')
}
</script>

<template>
  <Teleport to="body">
    <div
      v-if="show"
      ref="menuRef"
      class="fixed z-50"
      :style="{ left: `${adjustedX}px`, top: `${adjustedY}px` }"
    >
      <DropdownMenu :open="true">
        <DropdownMenuTrigger />
        <DropdownMenuContent align="start" class="min-w-48" @keydown.escape="emit('close')">
          <!-- Sort options -->
          <DropdownMenuItem @click="emit('sort', column, 'ASC'); emit('close')">
            <span class="i-carbon-arrow-up mr-2 h-3.5 w-3.5" />
            {{ t('components.dataGrid.sort.asc') }}
          </DropdownMenuItem>
          <DropdownMenuItem @click="emit('sort', column, 'DESC'); emit('close')">
            <span class="i-carbon-arrow-down mr-2 h-3.5 w-3.5" />
            {{ t('components.dataGrid.sort.desc') }}
          </DropdownMenuItem>
          <DropdownMenuItem
            v-if="currentSortDirection"
            @click="emit('clearSort'); emit('close')"
          >
            <span class="i-carbon-close mr-2 h-3.5 w-3.5" />
            {{ t('components.dataGrid.sort.clear') }}
          </DropdownMenuItem>

          <DropdownMenuSeparator />

          <!-- Filter options -->
          <DropdownMenuItem @click="handleFilterWithPrompt('eq')">
            <span class="i-carbon-filter mr-2 h-3.5 w-3.5" />
            {{ t('components.dataGrid.filter.byValue') }}
          </DropdownMenuItem>
          <DropdownMenuItem @click="handleFilterWithPrompt('like')">
            <span class="i-carbon-filter-edit mr-2 h-3.5 w-3.5" />
            {{ t('components.dataGrid.filter.byLike') }}
          </DropdownMenuItem>
          <DropdownMenuItem @click="handleFilterWithPrompt('gte')">
            <span class="i-carbon-filter-edit mr-2 h-3.5 w-3.5" />
            {{ t('components.dataGrid.filter.greaterThan') }}
          </DropdownMenuItem>
          <DropdownMenuItem @click="handleFilterWithPrompt('lte')">
            <span class="i-carbon-filter-edit mr-2 h-3.5 w-3.5" />
            {{ t('components.dataGrid.filter.lessThan') }}
          </DropdownMenuItem>
          <DropdownMenuItem @click="handleFilterWithPrompt('between')">
            <span class="i-carbon-filter-edit mr-2 h-3.5 w-3.5" />
            {{ t('components.dataGrid.filter.between') }}
          </DropdownMenuItem>
          <DropdownMenuItem @click="handleFilterWithPrompt('neq')">
            <span class="i-carbon-filter-remove mr-2 h-3.5 w-3.5" />
            {{ t('components.dataGrid.filter.excludeValue') }}
          </DropdownMenuItem>

          <DropdownMenuSeparator v-if="hasActiveFilter" />

          <DropdownMenuItem
            v-if="hasActiveFilter"
            @click="emit('clearFilter', column); emit('close')"
          >
            <span class="i-carbon-clean mr-2 h-3.5 w-3.5" />
            {{ t('components.dataGrid.filter.clearFilter') }}
          </DropdownMenuItem>
        </DropdownMenuContent>
      </DropdownMenu>
    </div>
  </Teleport>
</template>
