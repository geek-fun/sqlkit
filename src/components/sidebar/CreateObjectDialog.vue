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
  /** Object type label — 'database', 'schema', or custom */
  objectType: string
  /** Optional placeholder for the name input */
  placeholder?: string
  /** Optional description text */
  description?: string
}

const props = withDefaults(defineProps<Props>(), {
  placeholder: '',
  description: '',
})

const emit = defineEmits<{
  (e: 'update:open', value: boolean): void
  (e: 'confirm', name: string): void
}>()

const { t } = useI18n()
const objectName = ref('')

watch(() => props.open, (open) => {
  if (open)
    objectName.value = ''
})

function handleConfirm() {
  const trimmed = objectName.value.trim()
  if (trimmed)
    emit('confirm', trimmed)
}
</script>

<template>
  <Dialog :open="open" @update:open="(v: boolean) => emit('update:open', v)">
    <DialogContent class="sm:max-w-[400px]">
      <DialogHeader>
        <DialogTitle>{{ t('sidebar.databases.actions.createObject.title', { type: objectType }) }}</DialogTitle>
        <DialogDescription v-if="description">
          {{ description }}
        </DialogDescription>
      </DialogHeader>
      <Input
        v-model="objectName"
        :placeholder="placeholder || t('sidebar.databases.actions.createObject.placeholder', { type: objectType })"
        class="text-xs"
        @keydown.enter="handleConfirm"
      />
      <DialogFooter>
        <Button variant="outline" size="sm" @click="emit('update:open', false)">
          {{ t('sidebar.databases.actions.createObject.cancel') }}
        </Button>
        <Button size="sm" :disabled="!objectName.trim()" @click="handleConfirm">
          {{ t('sidebar.databases.actions.createObject.confirm') }}
        </Button>
      </DialogFooter>
    </DialogContent>
  </Dialog>
</template>
