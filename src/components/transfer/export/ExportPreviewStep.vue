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
  <div class="space-y-4">
    <div class="text-xs gap-2.5 grid grid-cols-2">
      <div class="space-y-1">
        <Label class="text-[11px] text-muted-foreground tracking-wide uppercase">Connection</Label>
        <div class="text-foreground font-mono tabular-nums">
          {{ sourceInfo.connection }}
        </div>
      </div>
      <div class="space-y-1">
        <Label class="text-[11px] text-muted-foreground tracking-wide uppercase">Database</Label>
        <div class="text-foreground font-mono tabular-nums">
          {{ sourceInfo.database }}
        </div>
      </div>
      <div class="space-y-1">
        <Label class="text-[11px] text-muted-foreground tracking-wide uppercase">Table</Label>
        <div class="text-foreground font-mono tabular-nums">
          {{ sourceInfo.table }}
        </div>
      </div>
      <div class="space-y-1">
        <Label class="text-[11px] text-muted-foreground tracking-wide uppercase">Format</Label>
        <div>
          <Badge class="text-[10px] text-primary font-mono px-1.5 py-0.5 border border-primary/60 bg-primary/[0.04] uppercase hover:bg-primary/[0.08]" variant="outline">
            {{ sourceInfo.format }}
          </Badge>
        </div>
      </div>
    </div>

    <Card class="border-border/40 shadow-sm overflow-hidden">
      <CardContent class="p-0 flex flex-col">
        <div class="px-3 py-2 border-b border-border/40 bg-muted/20 flex items-center justify-between">
          <Label id="preview-label" class="text-[11px] tracking-wide font-semibold flex gap-1.5 uppercase items-center"><span class="i-carbon-view text-muted-foreground" /> Preview <span class="text-muted-foreground font-normal normal-case">(first {{ props.previewRows || 10 }} rows)</span></Label>
          <div class="text-[10px] text-muted-foreground font-mono tabular-nums">
            <span v-if="estimatedRows">~{{ estimatedRows.toLocaleString() }} rows • </span>{{ sourceInfo.columns }} cols
          </div>
        </div>
        <div
          role="region"
          aria-labelledby="preview-label"
          tabindex="0"
          class="text-[11px] leading-snug font-mono p-3 bg-card max-h-[200px] whitespace-pre overflow-auto tabular-nums focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-primary"
        >
          {{ previewData || 'Loading preview...' }}
        </div>
      </CardContent>
    </Card>

    <div class="pt-2 space-y-1.5">
      <Label class="text-[11px] text-muted-foreground tracking-wide uppercase">Output File</Label>
      <div class="flex gap-2">
        <div class="flex-1 relative">
          <span class="i-carbon-document text-muted-foreground left-2.5 top-1/2 absolute -translate-y-1/2" />
          <Input
            v-model="outputPath"
            placeholder="/path/to/output.csv"
            class="text-xs font-mono pl-8 h-8 tabular-nums"
          />
        </div>
        <Button variant="outline" size="sm" class="text-xs px-3 h-8" @click="handleBrowse">
          Browse
        </Button>
      </div>
    </div>

    <div class="pt-2 border-t border-border/40 flex justify-end">
      <Button
        :disabled="!outputPath"
        size="sm"
        class="text-xs font-semibold px-4 h-8"
        @click="emit('execute')"
      >
        <span class="i-carbon-play mr-1.5" /> Start Export
      </Button>
    </div>
  </div>
</template>
