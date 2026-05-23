<script setup lang="ts">
import type { LauncherAction, LauncherState } from './types'
import type { ObjectSelection, TransferProfile, TransferScope } from '@/types/transfer'
import { computed, onMounted, ref } from 'vue'
import { useI18n } from 'vue-i18n'
import { Button } from '@/components/ui/button'
import { Dialog, DialogContent, DialogTitle } from '@/components/ui/dialog'
import { Input } from '@/components/ui/input'
import { useTransferStore } from '@/store/transferStore'

const props = defineProps<{
  modelValue: LauncherState
}>()

const emit = defineEmits<{
  (e: 'update:modelValue', value: LauncherState): void
}>()

const { t } = useI18n()
const store = useTransferStore()

onMounted(() => {
  store.loadProfiles().catch(e => console.error(e))
})

const presets = computed(() => store.savedProfiles)

const showSaveDialog = ref(false)
const presetName = ref('')

function handleSelectPreset(profile: TransferProfile) {
  const firstDb = profile.selection?.databases?.[0]
  const newState: LauncherState = {
    action: profile.kind as LauncherAction,
    source: {
      connectionId: profile.connectionId,
      scope: profile.scope,
      database: firstDb,
      schema: firstDb ? profile.selection?.schemas?.[firstDb]?.[0] : undefined,
      tables: firstDb ? profile.selection?.tables?.[firstDb] : undefined,
    },
    target: {
      connectionId: profile.targetConnectionId,
    },
    options: profile.options || {},
  }
  emit('update:modelValue', newState)
}

function openSaveDialog() {
  presetName.value = ''
  showSaveDialog.value = true
}

async function confirmSavePreset() {
  if (!presetName.value)
    return
  showSaveDialog.value = false

  try {
    const db = props.modelValue.source.database
    const selection: ObjectSelection = {
      serverId: props.modelValue.source.connectionId || '',
      databases: db ? [db] : [],
      schemas: db && props.modelValue.source.schema ? { [db]: [props.modelValue.source.schema] } : {},
      tables: db && props.modelValue.source.tables?.length ? { [db]: props.modelValue.source.tables } : {},
    }

    await store.saveProfile({
      id: '',
      name: presetName.value,
      kind: props.modelValue.action as TransferProfile['kind'],
      scope: props.modelValue.source.scope as TransferScope,
      connectionId: props.modelValue.source.connectionId || '',
      targetConnectionId: props.modelValue.target.connectionId,
      selection,
      options: props.modelValue.options,
      createdAt: Date.now(),
      lastRunAt: 0,
    })
  }
  catch (e) {
    console.error('Failed to save preset', e)
  }
}
</script>

<template>
  <div v-if="presets.length > 0 || props.modelValue.source.connectionId" class="flex flex-col gap-2">
    <div class="hide-scrollbar pb-1 flex gap-3 items-center overflow-x-auto">
      <Button
        v-for="preset in presets"
        :key="preset.id"
        variant="secondary"
        size="sm"
        class="rounded-full shrink-0"
        @click="handleSelectPreset(preset)"
      >
        <span class="i-carbon-play filled mr-1 opacity-50" />
        {{ preset.name }}
      </Button>

      <Button
        v-if="props.modelValue.source.connectionId"
        variant="outline"
        size="sm"
        class="rounded-full border-dashed shrink-0"
        @click="openSaveDialog"
      >
        <span class="i-carbon-add mr-1" />
        {{ t('transfer.launcher.savePreset') }}
      </Button>
    </div>

    <Dialog v-model:open="showSaveDialog">
      <DialogContent>
        <div class="text-center flex flex-col space-y-1.5 sm:text-left">
          <DialogTitle>{{ t('transfer.launcher.presetNamePrompt') }}</DialogTitle>
        </div>
        <div class="py-4">
          <Input v-model="presetName" placeholder="Preset name..." @keyup.enter="confirmSavePreset" />
        </div>
        <div class="flex flex-col-reverse sm:flex-row sm:justify-end sm:space-x-2">
          <Button variant="outline" @click="showSaveDialog = false">
            {{ t('common.buttons.cancel') }}
          </Button>
          <Button @click="confirmSavePreset">
            {{ t('common.buttons.save') }}
          </Button>
        </div>
      </DialogContent>
    </Dialog>
  </div>
</template>

<style scoped>
.hide-scrollbar::-webkit-scrollbar {
  display: none;
}
.hide-scrollbar {
  -ms-overflow-style: none;
  scrollbar-width: none;
}
</style>
