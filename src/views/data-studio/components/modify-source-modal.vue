<script setup lang="ts">
import type { DataSourcePermissions } from '@/store/dataStudioStore'
import { ref, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import { Button } from '@/components/ui/button'
import { Checkbox } from '@/components/ui/checkbox'
import { DialogContent, DialogDescription, DialogTitle } from '@/components/ui/dialog'
import { Label } from '@/components/ui/label'
import { useDataStudioStore } from '@/store/dataStudioStore'

const props = defineProps<{
  open: boolean
  sourceIdx: number
}>()

const emit = defineEmits<{
  'update:open': [value: boolean]
}>()

const { t } = useI18n()
const dataStudioStore = useDataStudioStore()

const permissions = ref<DataSourcePermissions>({
  read: true,
  create: false,
  update: false,
  delete: false,
})

watch(() => props.open, (open) => {
  if (open) {
    const session = dataStudioStore.activeSession
    if (session && session.sources[props.sourceIdx]) {
      const source = session.sources[props.sourceIdx]
      permissions.value = { ...source.permissions }
    }
  }
})

function save() {
  const session = dataStudioStore.activeSession
  if (!session)
    return
  const source = session.sources[props.sourceIdx]
  if (!source)
    return

  dataStudioStore.updateSessionSourcePermissions(
    source.sourceId,
    permissions.value,
  )
  emit('update:open', false)
}

function close() {
  emit('update:open', false)
}
</script>

<template>
  <DialogContent @close="close">
    <DialogTitle>{{ t('dataStudio.modifySource.title') }}</DialogTitle>
    <DialogDescription>
      {{ t('dataStudio.modifySource.description') }}
    </DialogDescription>
    <div class="py-4 space-y-4">
      <div class="space-y-3">
        <Label class="text-sm font-medium">{{ t('dataStudio.modifySource.accessPermissions') }}</Label>
        <div class="space-y-2">
          <div class="flex gap-2 items-center">
            <Checkbox id="perm-read" v-model="permissions.read" />
            <Label for="perm-read">{{ t('dataStudio.modifySource.read') }}</Label>
          </div>
          <div class="flex gap-2 items-center">
            <Checkbox id="perm-create" v-model="permissions.create" />
            <Label for="perm-create">{{ t('dataStudio.modifySource.create') }}</Label>
          </div>
          <div class="flex gap-2 items-center">
            <Checkbox id="perm-update" v-model="permissions.update" />
            <Label for="perm-update">{{ t('dataStudio.modifySource.update') }}</Label>
          </div>
          <div class="flex gap-2 items-center">
            <Checkbox id="perm-delete" v-model="permissions.delete" />
            <Label for="perm-delete">{{ t('dataStudio.modifySource.delete') }}</Label>
          </div>
        </div>
      </div>
    </div>
    <div class="flex gap-2 justify-end">
      <Button variant="outline" @click="close">
        {{ t('common.cancel') }}
      </Button>
      <Button @click="save">
        {{ t('common.save') }}
      </Button>
    </div>
  </DialogContent>
</template>
