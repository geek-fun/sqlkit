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

const fileInputRef = ref<HTMLInputElement | null>(null)

function triggerFileInput() {
  fileInputRef.value?.click()
}

function handleKeyDown(e: KeyboardEvent) {
  if (e.key === 'Enter' || e.key === ' ') {
    e.preventDefault()
    triggerFileInput()
  }
}

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
    class="border-dashed cursor-pointer transition-all duration-200 focus-visible:outline-none hover:bg-muted/50 focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2"
    :class="isDragging ? 'border-primary bg-primary/5' : 'border-border'"
    role="button"
    tabindex="0"
    aria-label="Drop a file or click to browse"
    @dragover="handleDragOver"
    @dragleave="handleDragLeave"
    @drop="handleDrop"
    @click="triggerFileInput"
    @keydown="handleKeyDown"
  >
    <CardContent class="p-8 text-center flex flex-col min-h-[160px] items-center justify-center">
      <div
        class="text-primary mb-4 rounded-full bg-primary/10 flex h-12 w-12 transition-transform duration-200 items-center justify-center"
        :class="{ 'scale-110': isDragging }"
        aria-hidden="true"
      >
        <span class="i-carbon-document-upload h-6 w-6" />
      </div>
      <h3 class="text-sm text-foreground tracking-tight font-semibold">
        Drag & drop a file here
      </h3>
      <p class="text-sm text-muted-foreground mt-1">
        or click to browse from your computer
      </p>
      <div class="text-xs text-muted-foreground font-medium mt-4 px-2 py-1 rounded bg-muted/50">
        Supported formats: {{ formats.map(f => f.toUpperCase()).join(', ') }}
      </div>
      <input
        ref="fileInputRef"
        type="file"
        class="hidden"
        tabindex="-1"
        :accept="formats.map(f => `.${f}`).join(',')"
        @change="handleFileInput"
      >
    </CardContent>
  </Card>
</template>
