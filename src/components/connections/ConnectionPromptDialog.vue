<script setup lang="ts">
import { ref } from 'vue'
import { useI18n } from 'vue-i18n'
import { Button } from '@/components/ui/button'
import { Dialog, DialogContent, DialogDescription, DialogFooter, DialogHeader, DialogTitle } from '@/components/ui/dialog'

const emit = defineEmits<{ save: [value: string] }>()
const { t } = useI18n()
const open = ref(false)
const draft = ref('')

const show = (value?: string) => {
  draft.value = value ?? ''
  open.value = true
}

const save = () => {
  emit('save', draft.value)
  open.value = false
}

defineExpose({ show })
</script>

<template>
  <Dialog v-model:open="open">
    <DialogContent class="sm:max-w-[560px]">
      <DialogHeader>
        <DialogTitle>{{ t('connection.prompt.title') }}</DialogTitle>
        <DialogDescription>{{ t('connection.prompt.description') }}</DialogDescription>
      </DialogHeader>
      <textarea
        v-model="draft"
        autocomplete="off"
        class="text-sm px-3 py-2 border border-input rounded-md bg-background max-h-80 min-h-48 w-full resize-y"
        :placeholder="t('connection.prompt.placeholder')"
      />
      <DialogFooter class="sm:justify-between">
        <Button type="button" variant="outline" :disabled="!draft.trim()" @click="draft = ''">
          {{ t('connection.prompt.clear') }}
        </Button>
        <div class="flex gap-2">
          <Button type="button" variant="outline" @click="open = false">
            {{ t('connection.prompt.cancel') }}
          </Button>
          <Button type="button" @click="save">
            {{ t('connection.prompt.confirm') }}
          </Button>
        </div>
      </DialogFooter>
    </DialogContent>
  </Dialog>
</template>
