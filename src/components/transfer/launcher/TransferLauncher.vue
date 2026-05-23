<script setup lang="ts">
import type { LauncherState } from './types'
import type { ObjectSelection } from '@/types/transfer'
import { computed, ref } from 'vue'
import { useI18n } from 'vue-i18n'

import { Button } from '@/components/ui/button'
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card'
import { toast } from '@/composables/useNotifications'
import { useTransferStore } from '@/store/transferStore'
import ActionPicker from './ActionPicker.vue'
import OptionsPanel from './OptionsPanel.vue'
import PresetsBar from './PresetsBar.vue'
import SourcePicker from './SourcePicker.vue'
import TargetPicker from './TargetPicker.vue'

const { t } = useI18n()
const store = useTransferStore()

const state = ref<LauncherState>({
  action: 'backup',
  source: {},
  target: {},
  options: {},
})

const isActionValid = computed(() => {
  const { action, source, target, options } = state.value

  if (!source.connectionId)
    return false
  if (!source.scope)
    return false
  if (source.scope === 'database' && !source.database)
    return false
  if (source.scope === 'table' && (!source.database || !source.tables?.length))
    return false

  if ((action === 'migrate' || action === 'restore') && !target.connectionId)
    return false

  if ((action === 'backup' || action === 'export') && !options.destination)
    return false

  if (action === 'restore') {
    if (!options.filePath)
      return false
    if (!options.fileFormat)
      return false
    if ((options.fileFormat === 'csv' || options.fileFormat === 'excel') && !options.targetTable?.trim())
      return false
  }

  return true
})

function buildSelection(): ObjectSelection {
  const { source } = state.value
  const db = source.database
  const schemaKey = source.schema ? `${db}.${source.schema}` : db
  return {
    serverId: source.connectionId || '',
    databases: db ? [db] : [],
    schemas: db && source.schema ? { [db]: [source.schema] } : {},
    tables: schemaKey && source.tables?.length ? { [schemaKey]: source.tables } : {},
  }
}

async function handleStart() {
  if (!isActionValid.value)
    return

  const { action, source, target, options } = state.value
  const selection = buildSelection()
  const label = source.database || 'server'

  try {
    if (action === 'backup') {
      await store.startBackupServer({
        connectionId: source.connectionId!,
        name: `Backup ${label}`,
        selection,
        format: options.format || 'sql',
        destination: options.destination!,
        options,
      })
    }
    else if (action === 'export') {
      await store.startExport({
        connectionId: source.connectionId!,
        name: `Export ${label}`,
        selection,
        format: options.format || 'sql',
        destination: options.destination!,
        options,
      })
    }
    else if (action === 'migrate') {
      await store.startMigrateServer({
        sourceConnectionId: source.connectionId!,
        targetConnectionId: target.connectionId!,
        name: `Migrate ${label}`,
        selection,
        options,
      })
    }
    else if (action === 'restore') {
      await store.startRestore({
        connectionId: target.connectionId || source.connectionId!,
        name: `Restore ${target.database || 'server'}`,
        targetDatabase: target.database,
        filePath: options.filePath!,
        fileFormat: options.fileFormat || 'sql',
        targetTable: options.targetTable,
        dropTargetFirst: options.dropTargetFirst || false,
      })
    }

    toast.success(t('transfer.launcher.started'), {
      description: t('transfer.launcher.startedDesc'),
    })
  }
  catch (error: unknown) {
    console.error(error)
    toast.error(t('transfer.launcher.failed'), {
      description: error instanceof Error ? error.message : String(error),
    })
  }
}
</script>

<template>
  <div class="mx-auto pb-32 flex flex-col gap-6 max-w-4xl w-full">
    <PresetsBar v-model="state" />

    <Card class="shadow-sm">
      <CardHeader class="pb-4">
        <CardTitle class="text-xl">
          {{ t('transfer.launcher.whatDoYouWant') }}
        </CardTitle>
      </CardHeader>
      <CardContent class="flex flex-col gap-8">
        <ActionPicker v-model="state.action" />

        <div class="space-y-4">
          <h3 class="text-sm text-muted-foreground font-medium">
            {{ t('transfer.launcher.source') }}
          </h3>
          <SourcePicker v-model="state.source" />
        </div>

        <div v-if="state.action === 'migrate' || state.action === 'restore'" class="space-y-4">
          <h3 class="text-sm text-muted-foreground font-medium">
            {{ t('transfer.launcher.target') }}
          </h3>
          <TargetPicker v-model="state.target" :source-connection-id="state.source.connectionId" />
        </div>

        <div class="space-y-4">
          <h3 class="text-sm text-muted-foreground font-medium">
            {{ t('transfer.launcher.options') }}
          </h3>
          <OptionsPanel v-model="state.options" :action="state.action" />
        </div>

        <div class="pt-4 border-t flex gap-3 justify-end">
          <Button variant="outline">
            {{ t('common.cancel') }}
          </Button>
          <Button :disabled="!isActionValid" @click="handleStart">
            {{ t(`transfer.launcher.actions.${state.action}.start`) }}
            <span class="i-carbon-arrow-right ml-2" />
          </Button>
        </div>
      </CardContent>
    </Card>
  </div>
</template>
