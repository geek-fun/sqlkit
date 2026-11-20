<script setup lang="ts">
import { ref } from 'vue'
import { useI18n } from 'vue-i18n'
import { Button } from '@/components/ui/button'

defineProps<{ code: string, label: string }>()

const { t } = useI18n()
const copied = ref(false)

const copy = async (code: string) => {
  try {
    await navigator.clipboard.writeText(code)
  }
  catch {
    const ta = document.createElement('textarea')
    ta.value = code
    document.body.appendChild(ta)
    ta.select()
    document.execCommand('copy')
    document.body.removeChild(ta)
  }
  copied.value = true
  setTimeout(() => (copied.value = false), 2000)
}
</script>

<template>
  <div class="min-w-0 relative">
    <pre class="text-xs leading-relaxed font-mono px-3 py-2 pr-10 rounded-md bg-muted overflow-x-auto">{{ code }}</pre>
    <Button
      type="button"
      variant="ghost"
      size="icon-sm"
      class="right-1 top-1 absolute"
      :aria-label="label"
      @click="copy"
    >
      <span
        aria-hidden="true"
        class="h-3.5 w-3.5"
        :class="copied ? 'i-lucide-check text-success' : 'i-lucide-copy'"
      />
    </Button>
    <span v-if="copied" role="status" class="sr-only">
      {{ t('pages.settings.mcp.install.copied') }}
    </span>
  </div>
</template>
