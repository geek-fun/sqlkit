<script setup lang="ts">
import type { CopyFormat } from '@/types/grid'
import { useI18n } from 'vue-i18n'
import { Button } from '@/components/ui/button'
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuTrigger,
} from '@/components/ui/dropdown-menu'

defineProps<{
  selectedCount: number
  selectedRows: Record<string, unknown>[]
  columns: string[]
  tableName?: string
}>()

const emit = defineEmits<{
  (e: 'deleteSelected'): void
  (e: 'exportSelected', format: CopyFormat): void
}>()

const { t } = useI18n()
</script>

<template>
  <Transition name="slide-up">
    <div
      v-if="selectedCount > 0"
      class="px-4 py-2 border-t border-primary/20 bg-primary/10 flex gap-3 items-center bottom-0 sticky z-30 backdrop-blur-sm"
    >
      <span class="text-sm font-medium tabular-nums">
        {{ t('dataGrid.selection.selected', { count: selectedCount }) }}
      </span>
      <div class="ml-auto flex gap-2">
        <DropdownMenu>
          <DropdownMenuTrigger as-child>
            <Button variant="outline" size="sm">
              {{ t('dataGrid.selection.exportSelected') }}
              <span class="i-carbon-chevron-down ml-1 h-3 w-3" />
            </Button>
          </DropdownMenuTrigger>
          <DropdownMenuContent align="end">
            <DropdownMenuItem @click="emit('exportSelected', 'csv')">
              <span class="i-carbon-table-split mr-2 h-3.5 w-3.5" /> CSV
            </DropdownMenuItem>
            <DropdownMenuItem @click="emit('exportSelected', 'json')">
              <span class="i-carbon-code mr-2 h-3.5 w-3.5" /> JSON
            </DropdownMenuItem>
            <DropdownMenuItem @click="emit('exportSelected', 'insert')">
              <span class="i-carbon-sql mr-2 h-3.5 w-3.5" /> INSERT
            </DropdownMenuItem>
          </DropdownMenuContent>
        </DropdownMenu>
        <Button
          variant="destructive"
          size="sm"
          @click="emit('deleteSelected')"
        >
          <span class="i-carbon-trash-can mr-1 h-3.5 w-3.5" />
          {{ t('dataGrid.selection.deleteSelected') }}
        </Button>
      </div>
    </div>
  </Transition>
</template>

<style scoped>
.slide-up-enter-active {
  transition: all 0.2s ease-out;
}
.slide-up-leave-active {
  transition: all 0.15s ease-in;
}
.slide-up-enter-from,
.slide-up-leave-to {
  transform: translateY(100%);
  opacity: 0;
}
</style>
