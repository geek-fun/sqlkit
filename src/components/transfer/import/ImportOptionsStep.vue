<script setup lang="ts">
import type { ConflictStrategy } from '@/types/transfer'

import { computed, ref, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import { Card, CardContent } from '@/components/ui/card'
import { Checkbox } from '@/components/ui/checkbox'
import { Input } from '@/components/ui/input'
import { Label } from '@/components/ui/label'

import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from '@/components/ui/select'
import { previewImport } from '@/datasources/transferApi'

import { useTransferStore } from '@/store/transferStore'

const transferStore = useTransferStore()
const { t } = useI18n()

const conflictStrategy = ref<ConflictStrategy>('skip')
const batchSize = ref(5000)
const truncateBefore = ref(false)
const dryRun = ref(false)

const previewData = ref<string[][]>([])
const previewColumns = ref<string[]>([])
const isLoading = ref(false)

const conflictOptions = computed<{ value: ConflictStrategy, label: string }[]>(() => [
  { value: 'skip', label: t('transfer.import.conflict.skip') },
  { value: 'replace', label: t('transfer.import.conflict.replace') },
  { value: 'upsert', label: t('transfer.import.conflict.upsert') },
  { value: 'abort', label: t('transfer.import.conflict.abort') },
])

async function loadPreview() {
  if (!transferStore.importRequest.filePath || !transferStore.importRequest.format) {
    return
  }

  isLoading.value = true
  try {
    const result = await previewImport(
      transferStore.importRequest.filePath,
      transferStore.importRequest.format,
      10,
    )
    previewColumns.value = result.columns
    previewData.value = result.sampleRows
  }
  catch (error) {
    console.error('Preview failed:', error)
  }
  finally {
    isLoading.value = false
  }
}

watch([conflictStrategy, batchSize, truncateBefore, dryRun], () => {
  transferStore.importRequest = {
    ...transferStore.importRequest,
    conflictStrategy: conflictStrategy.value,
    batchSize: batchSize.value,
    truncateBefore: truncateBefore.value,
    dryRun: dryRun.value,
  }
})

watch(() => transferStore.importRequest.filePath, () => {
  loadPreview()
}, { immediate: true })
</script>

<template>
  <div class="space-y-4">
    <Card class="border-border/40 shadow-none">
      <CardContent class="p-3">
        <div class="text-xs tracking-wide font-semibold mb-3 flex gap-2 items-center">
          <div class="i-carbon-settings-adjust" />
          {{ t('transfer.import.importSettings') }}
        </div>

        <div class="mb-3 gap-4 grid grid-cols-2">
          <div class="space-y-1.5">
            <Label class="text-[11px] text-muted-foreground tracking-wide block uppercase">{{ t('transfer.import.conflict.label') }}</Label>
            <Select v-model="conflictStrategy">
              <SelectTrigger class="text-xs h-8">
                <SelectValue />
              </SelectTrigger>
              <SelectContent>
                <SelectItem
                  v-for="opt in conflictOptions"
                  :key="opt.value"
                  :value="opt.value"
                  class="text-xs"
                >
                  {{ opt.label }}
                </SelectItem>
              </SelectContent>
            </Select>
          </div>

          <div class="space-y-1.5">
            <Label class="text-[11px] text-muted-foreground tracking-wide block uppercase">{{ t('transfer.import.batchSize') }}</Label>
            <Input
              v-model.number="batchSize"
              type="number"
              min="1"
              max="100000"
              class="text-xs font-mono h-8 tabular-nums"
            />
          </div>
        </div>

        <div class="pt-2 border-t border-border/40 space-y-0.5">
          <div class="px-2 py-1.5 rounded-sm flex transition-colors items-center space-x-2 hover:bg-muted/40">
            <Checkbox id="import-opt-truncate" v-model:checked="truncateBefore" class="h-3.5 w-3.5" />
            <Label for="import-opt-truncate" class="text-xs cursor-pointer select-none">{{ t('transfer.import.truncateBefore') }}</Label>
          </div>

          <div class="px-2 py-1.5 rounded-sm flex transition-colors items-center space-x-2 hover:bg-muted/40">
            <Checkbox id="import-opt-dry-run" v-model:checked="dryRun" class="h-3.5 w-3.5" />
            <Label for="import-opt-dry-run" class="text-xs cursor-pointer select-none">{{ t('transfer.import.dryRun') }}</Label>
          </div>
        </div>
      </CardContent>
    </Card>

    <Card class="border-border/40 shadow-none">
      <CardContent class="p-3">
        <div class="mb-2 flex items-center justify-between">
          <div class="text-xs tracking-wide font-semibold flex gap-2 items-center">
            <div class="i-carbon-table" />
            {{ t('transfer.import.dataPreview') }}
          </div>
          <span class="text-[10px] text-muted-foreground tracking-wide font-mono uppercase">{{ t('transfer.import.previewRows') }}</span>
        </div>

        <div class="border border-border/40 rounded-sm overflow-hidden">
          <div v-if="previewData.length > 0" class="max-h-[240px] overflow-auto">
            <table class="text-[10px] text-left w-full whitespace-nowrap border-collapse">
              <thead class="bg-muted/40 shadow-border/40 shadow-sm top-0 sticky z-10">
                <tr>
                  <th v-for="col in previewColumns" :key="col" class="text-muted-foreground tracking-wide font-medium px-2 py-1.5 border-b border-border/40 uppercase">
                    {{ col }}
                  </th>
                </tr>
              </thead>
              <tbody class="font-mono">
                <tr v-for="(row, i) in previewData" :key="i" class="border-b border-border/40 transition-colors last:border-0 hover:bg-muted/40">
                  <td v-for="(val, j) in row" :key="j" class="px-2 py-1.5 max-w-[150px] truncate" :title="val">
                    {{ val }}
                  </td>
                </tr>
              </tbody>
            </table>
          </div>
          <div v-else class="text-[11px] text-muted-foreground py-6 text-center bg-muted/20">
            <div v-if="isLoading" class="flex flex-col gap-2 items-center justify-center">
              <div class="i-carbon-circle-dash opacity-50 animate-spin" />
              {{ t('transfer.import.loadingPreview') }}
            </div>
            <span v-else>{{ t('transfer.import.noPreviewData') }}</span>
          </div>
        </div>
      </CardContent>
    </Card>
  </div>
</template>
