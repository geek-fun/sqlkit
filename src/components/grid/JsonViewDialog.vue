<script setup lang="ts">
import { computed } from 'vue'
import { useI18n } from 'vue-i18n'
import { Button } from '@/components/ui/button'
import { Dialog, DialogContent, DialogTitle } from '@/components/ui/dialog'
import { toast } from '@/composables/useNotifications'

type Props = {
  open: boolean
  value: unknown
  column: string
}

const props = defineProps<Props>()

const emit = defineEmits<{
  (e: 'update:open', open: boolean): void
  (e: 'close'): void
}>()

const { t } = useI18n()

const formattedJson = computed(() => {
  try {
    return JSON.stringify(
      typeof props.value === 'string' ? JSON.parse(props.value) : props.value,
      null,
      2,
    )
  }
  catch {
    return String(props.value ?? '')
  }
})

async function copyJson() {
  await navigator.clipboard.writeText(formattedJson.value)
  toast.success(t('dataGrid.json.copy'))
}
</script>

<template>
  <Dialog :open="open" @update:open="emit('update:open', $event)">
    <DialogContent class="flex flex-col max-h-[80vh] max-w-3xl">
      <DialogTitle>{{ column }}</DialogTitle>
      <div class="mt-4 flex-1 min-h-0 relative">
        <Button
          variant="ghost"
          size="sm"
          class="right-2 top-2 absolute z-10"
          @click="copyJson"
        >
          <span class="i-carbon-copy mr-1 h-3.5 w-3.5" />{{ t('dataGrid.json.copy') }}
        </Button>
        <pre class="text-sm font-mono p-4 rounded-md bg-muted/50 max-h-[60vh] whitespace-pre-wrap break-all overflow-auto">{{ formattedJson }}</pre>
      </div>
    </DialogContent>
  </Dialog>
</template>
