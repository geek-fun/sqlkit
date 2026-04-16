<script setup lang="ts">
import { ref } from 'vue'

import { Card, CardContent } from '@/components/ui/card'

const props = defineProps<{
  acceptedFormats?: string[]
}>()

const emit = defineEmits<{
  fileSelected: [file: File]
}>()

const isDragging = ref(false)

const formats = props.acceptedFormats || ['csv', 'jsonl', 'sql', 'xlsx']

function handleDragOver(e: DragEvent) {
  e.preventDefault()
  isDragging.value = true
}

function handleDragLeave() {
  isDragging.value = false
}

function handleDrop(e: DragEvent) {
  e.preventDefault()
  isDragging.value = false
  const files = e.dataTransfer?.files
  if (files?.length) {
    emit('fileSelected', files[0])
  }
}

function handleFileInput(e: Event) {
  const target = e.target as HTMLInputElement
  const files = target.files
  if (files?.length) {
    emit('fileSelected', files[0])
  }
}
</script>

<template>
  <Card
    class="cursor-pointer transition-colors"
    :class="isDragging ? 'border-primary bg-secondary' : 'border-border'"
    @dragover="handleDragOver"
    @dragleave="handleDragLeave"
    @drop="handleDrop"
  >
    <CardContent class="pt-6 flex flex-col min-h-32 items-center justify-center">
      <div class="text-4xl mb-2">
        📄
      </div>
      <div class="text-sm font-medium">
        Drag & drop a file here
      </div>
      <div class="text-xs text-muted-foreground mt-1">
        or click to browse
      </div>
      <div class="text-xs text-muted-foreground mt-2">
        Supported: {{ formats.map(f => f.toUpperCase()).join(', ') }}
      </div>
      <input
        type="file"
        class="hidden"
        :accept="formats.map(f => `.${f}`).join(',')"
        @change="handleFileInput"
      >
    </CardContent>
  </Card>
</template>
