<script setup lang="ts">
import { computed, ref } from 'vue'
import { useI18n } from 'vue-i18n'
import { Button } from '@/components/ui/button'
import {
  Dialog,
  DialogContent,
  DialogTitle,
} from '@/components/ui/dialog'

const props = defineProps<{
  open: boolean
  title: string
  ddl: string
}>()

const emit = defineEmits<{
  'update:open': [value: boolean]
}>()

const { t } = useI18n()

const copySuccess = ref(false)

async function copyDdl() {
  try {
    await navigator.clipboard.writeText(props.ddl)
    copySuccess.value = true
    setTimeout(() => { copySuccess.value = false }, 2000)
  }
  catch {
    // Fallback
    const ta = document.createElement('textarea')
    ta.value = props.ddl
    document.body.appendChild(ta)
    ta.select()
    document.execCommand('copy')
    document.body.removeChild(ta)
    copySuccess.value = true
    setTimeout(() => { copySuccess.value = false }, 2000)
  }
}

const formattedDdl = computed(() => {
  // Basic SQL formatting: ensure consistent line endings
  return props.ddl.replace(/\r\n/g, '\n')
})
</script>

<template>
  <Dialog :open="open" @update:open="emit('update:open', $event)">
    <DialogContent class="max-w-3xl max-h-[80vh] flex flex-col p-0 gap-0">
      <div class="px-4 py-3 border-b flex items-center gap-2">
        <DialogTitle class="text-sm font-semibold flex-1 truncate">
          {{ title }}
        </DialogTitle>
        <Button variant="outline" size="sm" class="text-xs h-7" @click="copyDdl">
          <svg
            v-if="!copySuccess"
            xmlns="http://www.w3.org/2000/svg" width="12" height="12" viewBox="0 0 24 24"
            fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round"
            stroke-linejoin="round" class="mr-1"
          >
            <rect width="14" height="14" x="8" y="8" rx="2" ry="2" />
            <path d="M4 16c-1.1 0-2-.9-2-2V4c0-1.1.9-2 2-2h10c1.1 0 2 .9 2 2" />
          </svg>
          <svg
            v-else
            xmlns="http://www.w3.org/2000/svg" width="12" height="12" viewBox="0 0 24 24"
            fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round"
            stroke-linejoin="round" class="mr-1"
          >
            <polyline points="20 6 9 17 4 12" />
          </svg>
          {{ copySuccess ? t('common.copied') : t('common.copy') }}
        </Button>
      </div>
      <div class="flex-1 overflow-auto p-4">
        <pre class="text-xs font-mono leading-relaxed whitespace-pre-wrap break-all bg-muted rounded-md p-4 overflow-x-auto"><code>{{ formattedDdl }}</code></pre>
      </div>
    </DialogContent>
  </Dialog>
</template>
