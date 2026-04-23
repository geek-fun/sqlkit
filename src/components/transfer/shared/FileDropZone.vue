<!--
  Visual Role: Compact file upload area for import wizard.
  Uses subtle primary outlines and monospaced format badges on hover/drag.
-->
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
    class="rounded-md border-dashed cursor-pointer transition-all duration-200 focus-visible:outline-none hover:bg-muted/30 focus-visible:ring-1 focus-visible:ring-primary focus-visible:ring-offset-1"
    :class="isDragging ? 'border-primary/60 bg-primary/[0.04]' : 'border-border/60'"
    role="button"
    tabindex="0"
    aria-label="Drop a file or click to browse"
    @dragover="handleDragOver"
    @dragleave="handleDragLeave"
    @drop="handleDrop"
    @click="triggerFileInput"
    @keydown="handleKeyDown"
  >
    <CardContent class="p-4 text-center flex flex-col min-h-[120px] items-center justify-center">
      <div
        class="text-primary mb-2.5 rounded-full bg-primary/5 flex h-9 w-9 items-center justify-center"
        aria-hidden="true"
      >
        <span class="i-carbon-document-upload h-4.5 w-4.5" />
      </div>
      <h3 class="text-sm text-foreground tracking-tight font-medium">
        Drag & drop a file here
      </h3>
      <p class="text-[11px] text-muted-foreground mt-0.5">
        or click to browse from your computer
      </p>
      <div class="mt-3 flex flex-wrap gap-1.5 justify-center">
        <span
          v-for="f in formats"
          :key="f"
          class="text-[10px] text-muted-foreground tracking-wider font-mono px-1.5 py-0.5 rounded-sm bg-muted/60 uppercase"
        >
          {{ f }}
        </span>
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
