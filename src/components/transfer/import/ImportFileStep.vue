<script setup lang="ts">
import type { FileDetectionResult } from '@/types/transfer'

import { open } from '@tauri-apps/plugin-dialog'
import { computed, ref, watch } from 'vue'
import { Badge } from '@/components/ui/badge'

import { Card, CardContent } from '@/components/ui/card'

import { Label } from '@/components/ui/label'
import { detectFile } from '@/datasources/transferApi'

import { useTransferStore } from '@/store/transferStore'

import FileDropZone from '../shared/FileDropZone.vue'

const transferStore = useTransferStore()

const filePath = ref('')
const detectionResult = ref<FileDetectionResult | null>(null)
const isDetecting = ref(false)

const formatLabel = computed(() => {
  if (!detectionResult.value)
    return ''
  return detectionResult.value.format.toUpperCase()
})

async function handleFileDrop(_file: File) {
  filePath.value = 'uploaded-file'
  await detectFileInfo()
}

async function handleBrowse() {
  const selected = await open({
    multiple: false,
    filters: [
      { name: 'Import File', extensions: ['csv', 'jsonl', 'sql', 'xlsx'] },
    ],
  })
  if (selected) {
    filePath.value = selected as string
    await detectFileInfo()
  }
}

async function detectFileInfo() {
  if (!filePath.value)
    return

  isDetecting.value = true
  try {
    const result = await detectFile(filePath.value)
    detectionResult.value = result

    transferStore.importRequest = {
      ...transferStore.importRequest,
      filePath: filePath.value,
      format: result.format,
      columnMappings: result.columns.map(col => ({
        sourceColumn: col,
        targetColumn: col,
      })),
    }
  }
  catch (error) {
    console.error('Detection failed:', error)
  }
  finally {
    isDetecting.value = false
  }
}

watch(filePath, () => {
  if (filePath.value) {
    detectFileInfo()
  }
})
</script>

<template>
  <div class="space-y-2">
    <FileDropZone @file-selected="handleFileDrop" />

    <div class="flex justify-end">
      <Button variant="outline" size="sm" @click="handleBrowse">
        <div class="i-carbon-folder-open mr-2" />
        Browse Files
      </Button>
    </div>

    <Card v-if="detectionResult" class="border-border/40 shadow-none">
      <CardContent class="p-3 space-y-3">
        <div class="text-xs tracking-wide font-semibold mb-2 flex gap-2 items-center">
          <div class="i-carbon-document" />
          FILE DETAILS
        </div>

        <div class="gap-2.5 grid grid-cols-2">
          <div>
            <Label class="text-[11px] text-muted-foreground tracking-wide mb-1 block uppercase">Detected Format</Label>
            <Badge class="text-[10px] font-mono px-1 py-0.5 uppercase" variant="secondary">
              {{ formatLabel }}
            </Badge>
          </div>
          <div>
            <Label class="text-[11px] text-muted-foreground tracking-wide mb-1 block uppercase">Encoding</Label>
            <Badge class="text-[10px] font-mono px-1 py-0.5 uppercase" variant="outline">
              {{ detectionResult.encoding }}
            </Badge>
          </div>
          <div>
            <Label class="text-[11px] text-muted-foreground tracking-wide mb-1 block uppercase">Estimated Rows</Label>
            <div class="text-xs font-mono tabular-nums">
              {{ detectionResult.estimatedRows?.toLocaleString() || 'Unknown' }}
            </div>
          </div>
          <div>
            <Label class="text-[11px] text-muted-foreground tracking-wide mb-1 block uppercase">File Size</Label>
            <div class="text-xs font-mono tabular-nums">
              {{ (detectionResult.fileSizeBytes / 1024).toFixed(1) }} KB
            </div>
          </div>
        </div>

        <div class="pt-2 border-t border-border/40">
          <Label class="text-[11px] text-muted-foreground tracking-wide mb-2 block uppercase">Detected Columns</Label>
          <div class="flex flex-wrap gap-1.5">
            <Badge
              v-for="col in detectionResult.columns.slice(0, 10)"
              :key="col"
              variant="outline"
              class="text-[10px] font-mono px-1.5 py-0.5 border-border/40"
            >
              {{ col }}
            </Badge>
            <Badge v-if="detectionResult.columns.length > 10" variant="outline" class="text-[10px] font-mono px-1.5 py-0.5 border-dashed">
              +{{ detectionResult.columns.length - 10 }} more
            </Badge>
          </div>
        </div>
      </CardContent>
    </Card>
  </div>
</template>
