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
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select'
import { useConnectionStore } from '@/store'

type Props = {
  open: boolean
  currentConnectionId: string | null
}

const props = defineProps<Props>()

const emit = defineEmits<{
  (e: 'update:open', value: boolean): void
  (e: 'confirm', connectionId: string | null, connectionName: string | null): void
}>()

const { t } = useI18n()
const connectionStore = useConnectionStore()
const selectedId = ref<string>('')

watch(() => props.open, (open) => {
  if (open) {
    selectedId.value = props.currentConnectionId ?? ''
  }
})

const connections = () => connectionStore.connections

function handleConfirm() {
  if (selectedId.value === '') {
    emit('confirm', null, null)
  }
  else {
    const conn = connectionStore.getConnectionById(selectedId.value)
    emit('confirm', selectedId.value, conn?.name ?? null)
  }
}
</script>

<template>
  <Dialog :open="open" @update:open="(v: boolean) => emit('update:open', v)">
    <DialogContent class="sm:max-w-[400px]">
      <DialogHeader>
        <DialogTitle>{{ t('sidebar.savedQueries.connectionDialog.title') }}</DialogTitle>
        <DialogDescription>{{ t('sidebar.savedQueries.connectionDialog.label') }}</DialogDescription>
      </DialogHeader>
      <Select v-model="selectedId">
        <SelectTrigger class="text-xs h-8">
          <SelectValue :placeholder="t('sidebar.savedQueries.connectionDialog.label')" />
        </SelectTrigger>
        <SelectContent>
          <SelectItem value="">
            {{ t('sidebar.savedQueries.connectionDialog.none') }}
          </SelectItem>
          <SelectItem
            v-for="conn in connections()"
            :key="conn.id"
            :value="conn.id!"
            class="text-xs"
          >
            {{ conn.name }}
          </SelectItem>
        </SelectContent>
      </Select>
      <DialogFooter>
        <Button variant="outline" size="sm" @click="emit('update:open', false)">
          {{ t('sidebar.savedQueries.connectionDialog.cancel') }}
        </Button>
        <Button size="sm" @click="handleConfirm">
          {{ t('sidebar.savedQueries.connectionDialog.confirm') }}
        </Button>
      </DialogFooter>
    </DialogContent>
  </Dialog>
</template>
