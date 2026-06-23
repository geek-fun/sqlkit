<script setup lang="ts">
import { ref, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import { Button } from '@/components/ui/button'
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog'
import { Input } from '@/components/ui/input'

type Props = {
  open: boolean
  currentName: string | null
}

const props = defineProps<Props>()

const emit = defineEmits<{
  (e: 'update:open', value: boolean): void
  (e: 'confirm', newName: string): void
}>()

const { t } = useI18n()
const newName = ref('')

watch(() => props.open, (open) => {
  if (open && props.currentName) {
    newName.value = props.currentName.replace(/\.sql$/i, '')
  }
})

function handleConfirm() {
  const trimmed = newName.value.trim()
  if (trimmed) {
    const withExt = trimmed.endsWith('.sql') ? trimmed : `${trimmed}.sql`
    emit('confirm', withExt)
  }
}
</script>

<template>
  <Dialog :open="open" @update:open="(v: boolean) => emit('update:open', v)">
    <DialogContent class="sm:max-w-[400px]">
      <DialogHeader>
        <DialogTitle>{{ t('sidebar.savedQueries.renameDialog.title') }}</DialogTitle>
        <DialogDescription>{{ t('sidebar.savedQueries.renameDialog.label') }}</DialogDescription>
      </DialogHeader>
      <Input
        v-model="newName"
        :placeholder="t('sidebar.savedQueries.renameDialog.placeholder')"
        class="text-xs"
        @keydown.enter="handleConfirm"
      />
      <DialogFooter>
        <Button variant="outline" size="sm" @click="emit('update:open', false)">
          {{ t('sidebar.savedQueries.renameDialog.cancel') }}
        </Button>
        <Button size="sm" :disabled="!newName.trim()" @click="handleConfirm">
          {{ t('sidebar.savedQueries.renameDialog.confirm') }}
        </Button>
      </DialogFooter>
    </DialogContent>
  </Dialog>
</template>
