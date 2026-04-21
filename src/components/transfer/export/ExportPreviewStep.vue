<script setup lang="ts">
import { open } from '@tauri-apps/plugin-dialog'

import { computed, onMounted, ref } from 'vue'
import { Badge } from '@/components/ui/badge'
import { Button } from '@/components/ui/button'
import { Card, CardContent } from '@/components/ui/card'
import { Input } from '@/components/ui/input'
import { Label } from '@/components/ui/label'

import { previewExport } from '@/datasources/transferApi'

import { useTransferStore } from '@/store/transferStore'

const props = defineProps<{
  previewRows?: number
}>()

const emit = defineEmits<{
  execute: []
}>()

const transferStore = useTransferStore()

const outputPath = ref('')
const previewData = ref<string>('')
const estimatedRows = ref<number | undefined>()
const isLoading = ref(false)

const sourceInfo = computed(() => {
  const req = transferStore.exportRequest
  return {
    connection: req.connectionId || 'Not selected',
    database: req.database || 'default',
    table: req.source?.table || 'Not selected',
    format: req.format?.toUpperCase() || 'CSV',
    columns: req.source?.columns?.length || 0,
  }
})

async function loadPreview() {
  if (!transferStore.exportRequest.connectionId || !transferStore.exportRequest.source?.table) {
    return
  }

  isLoading.value = true
  try {
    const result = await previewExport(
      transferStore.exportRequest as any,
      props.previewRows || 10,
    )
    previewData.value = result.formattedPreview
    estimatedRows.value = result.totalRowsEstimate
  }
  catch (error) {
    console.error('Preview failed:', error)
  }
  finally {
    isLoading.value = false
  }
}

async function handleBrowse() {
  const selected = await open({
    multiple: false,
    directory: false,
    save: true,
    filters: [
      { name: 'Export File', extensions: ['csv', 'jsonl'] },
    ],
  })
  if (selected) {
    outputPath.value = selected as string
    transferStore.exportRequest.outputPath = outputPath.value
  }
}

onMounted(() => {
  loadPreview()
})
</script>

<template>
  <div class="space-y-6">
    <div class="text-sm gap-4 grid grid-cols-2 md:grid-cols-4">
      <div>
        <Label>Connection</Label>
        <div class="mt-1">
          {{ sourceInfo.connection }}
        </div>
      </div>
      <div>
        <Label>Database</Label>
        <div class="mt-1">
          {{ sourceInfo.database }}
        </div>
      </div>
      <div>
        <Label>Table</Label>
        <div class="mt-1">
          {{ sourceInfo.table }}
        </div>
      </div>
      <div>
        <Label>Format</Label>
        <Badge class="mt-1">
          {{ sourceInfo.format }}
        </Badge>
      </div>
    </div>

    <Card>
      <CardContent class="pt-4">
        <Label id="preview-label" class="mb-2">Preview (first {{ props.previewRows || 10 }} rows)</Label>
        <div
          role="region"
          aria-labelledby="preview-label"
          tabindex="0"
          class="text-xs font-mono mt-2 p-4 rounded bg-muted max-h-[200px] whitespace-pre overflow-auto focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2"
        >
          {{ previewData || 'Loading preview...' }}
        </div>
        <div class="text-sm text-muted-foreground mt-2">
          <span v-if="estimatedRows">
            Estimated {{ estimatedRows.toLocaleString() }} rows
          </span>
          <span class="ml-2">
            {{ sourceInfo.columns }} columns
          </span>
        </div>
      </CardContent>
    </Card>

    <div class="space-y-2">
      <Label>Output File</Label>
      <div class="flex gap-2">
        <Input
          v-model="outputPath"
          placeholder="/path/to/output.csv"
          class="flex-1"
        />
        <Button variant="outline" @click="handleBrowse">
          Browse
        </Button>
      </div>
    </div>

    <div class="flex justify-end">
      <Button
        :disabled="!outputPath"
        @click="emit('execute')"
      >
        Export
      </Button>
    </div>
  </div>
</template>
