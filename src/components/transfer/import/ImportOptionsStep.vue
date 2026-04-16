<script setup lang="ts">
import type { ConflictStrategy } from '@/types/transfer'

import { ref, watch } from 'vue'
import { Card, CardContent } from '@/components/ui/card'
import { Checkbox } from '@/components/ui/checkbox'
import { Input } from '@/components/ui/input'
import { Label } from '@/components/ui/label'

import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from '@/components/ui/select'
import { previewImport } from '@/datasources/transferApi'

import { useTransferStore } from '@/store/transferStore'

const transferStore = useTransferStore()

const conflictStrategy = ref<ConflictStrategy>('skip')
const batchSize = ref(5000)
const truncateBefore = ref(false)
const dryRun = ref(false)

const previewData = ref<string[][]>([])
const previewColumns = ref<string[]>([])
const isLoading = ref(false)

const conflictOptions: { value: ConflictStrategy, label: string }[] = [
  { value: 'skip', label: 'Skip duplicates' },
  { value: 'replace', label: 'Replace existing' },
  { value: 'upsert', label: 'Update existing (upsert)' },
  { value: 'abort', label: 'Abort on error' },
]

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
  <div class="space-y-6">
    <Card>
      <CardContent class="pt-4 space-y-4">
        <div class="space-y-2">
          <Label>On Conflict</Label>
          <Select v-model="conflictStrategy">
            <SelectTrigger>
              <SelectValue />
            </SelectTrigger>
            <SelectContent>
              <SelectItem
                v-for="opt in conflictOptions"
                :key="opt.value"
                :value="opt.value"
              >
                {{ opt.label }}
              </SelectItem>
            </SelectContent>
          </Select>
        </div>

        <div class="space-y-2">
          <Label>Batch Size</Label>
          <Input
            v-model.number="batchSize"
            type="number"
            min="1"
            max="100000"
          />
        </div>

        <div class="flex items-center space-x-2">
          <Checkbox v-model:checked="truncateBefore" />
          <Label class="cursor-pointer">Truncate table before import</Label>
        </div>

        <div class="flex items-center space-x-2">
          <Checkbox v-model:checked="dryRun" />
          <Label class="cursor-pointer">Dry run (validate without inserting)</Label>
        </div>
      </CardContent>
    </Card>

    <Card>
      <CardContent class="pt-4">
        <Label class="mb-2">Data Preview (first 10 rows)</Label>
        <div v-if="previewData.length > 0" class="mt-2 max-h-200px overflow-auto">
          <table class="text-xs w-full">
            <thead>
              <tr>
                <th v-for="col in previewColumns" :key="col" class="p-2 text-left border-b">
                  {{ col }}
                </th>
              </tr>
            </thead>
            <tbody>
              <tr v-for="(row, i) in previewData" :key="i">
                <td v-for="(val, j) in row" :key="j" class="p-2 border-b">
                  {{ val }}
                </td>
              </tr>
            </tbody>
          </table>
        </div>
        <div v-else class="text-sm mt-2 p-4 text-center rounded bg-muted">
          {{ isLoading ? 'Loading preview...' : 'No preview data' }}
        </div>
      </CardContent>
    </Card>
  </div>
</template>
