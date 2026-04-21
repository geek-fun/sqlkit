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
  <div class="space-y-6">
    <FileDropZone @file-selected="handleFileDrop" />

    <Button variant="outline" class="w-full" @click="handleBrowse">
      Browse Files
    </Button>

    <Card v-if="detectionResult">
      <CardContent class="pt-4 space-y-4">
        <div class="text-sm gap-4 grid grid-cols-1 sm:grid-cols-2">
          <div>
            <Label>Detected Format</Label>
            <Badge class="mt-1">
              {{ formatLabel }}
            </Badge>
          </div>
          <div>
            <Label>Encoding</Label>
            <div class="mt-1">
              {{ detectionResult.encoding }}
            </div>
          </div>
          <div>
            <Label>Estimated Rows</Label>
            <div class="mt-1">
              {{ detectionResult.estimatedRows?.toLocaleString() || 'Unknown' }}
            </div>
          </div>
          <div>
            <Label>File Size</Label>
            <div class="mt-1">
              {{ (detectionResult.fileSizeBytes / 1024).toFixed(1) }} KB
            </div>
          </div>
        </div>

        <div>
          <Label>Detected Columns</Label>
          <div class="mt-2 flex flex-wrap gap-2">
            <Badge
              v-for="col in detectionResult.columns.slice(0, 10)"
              :key="col"
              variant="outline"
            >
              {{ col }}
            </Badge>
            <Badge v-if="detectionResult.columns.length > 10" variant="outline">
              +{{ detectionResult.columns.length - 10 }} more
            </Badge>
          </div>
        </div>
      </CardContent>
    </Card>
  </div>
</template>
