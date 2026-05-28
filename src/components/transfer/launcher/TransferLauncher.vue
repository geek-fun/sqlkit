<script setup lang="ts">
import type { LauncherState } from './types'
import type { ObjectSelection } from '@/types/transfer'
import { computed, ref } from 'vue'
import { useI18n } from 'vue-i18n'

import { Button } from '@/components/ui/button'

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
  <div class="mx-auto pb-32 flex flex-col gap-4 max-w-5xl w-full">
    <!-- Presets Bar -->
    <PresetsBar v-model="state" />

    <hr class="transfer-divider">

    <!-- Context Bar -->
    <div class="transfer-context-bar rounded-md">
      <div class="transfer-context-segment">
        <span class="i-carbon-server text-xs" />
        <span>CONN</span>
        <strong>{{ state.source.connectionId ? 'connected' : '—' }}</strong>
      </div>
      <span class="transfer-context-separator">▸</span>
      <div class="transfer-context-segment">
        <span>SCOPE</span>
        <strong>{{ state.source.scope || '—' }}</strong>
      </div>
      <span v-if="state.source.database" class="transfer-context-separator">▸</span>
      <div v-if="state.source.database" class="transfer-context-segment">
        <span>DB</span>
        <strong>{{ state.source.database }}</strong>
      </div>
      <span v-if="state.source.schema" class="transfer-context-separator">▸</span>
      <div v-if="state.source.schema" class="transfer-context-segment">
        <span>SCHEMA</span>
        <strong>{{ state.source.schema }}</strong>
      </div>

      <span class="flex-1" />

      <div class="transfer-context-segment">
        <span class="i-carbon-arrow-right text-[10px]" />
        <span>{{ state.action }}</span>
      </div>
      <span class="transfer-context-separator">▸</span>
      <div class="transfer-context-segment">
        <span>FMT</span>
        <strong>{{ state.options.format || state.options.fileFormat || '—' }}</strong>
      </div>
    </div>

    <!-- Action Tiles -->
    <ActionPicker v-model="state.action" />

    <!-- Split Console Panels -->
    <div class="transfer-console-split">
      <!-- Source Panel -->
      <div class="transfer-console-section">
        <div class="transfer-console-section-header">
          <span class="i-carbon-arrow-down-left text-xs" />
          Source
        </div>
        <div class="transfer-console-section-body">
          <SourcePicker v-model="state.source" />
        </div>
      </div>

      <!-- Destination / Target Panel -->
      <div class="transfer-console-section">
        <div class="transfer-console-section-header">
          <span class="i-carbon-arrow-up-right text-xs" />
          Destination
        </div>
        <div class="transfer-console-section-body space-y-3">
          <TargetPicker
            v-if="state.action === 'migrate' || state.action === 'restore'"
            v-model="state.target"
            :source-connection-id="state.source.connectionId"
          />
          <OptionsPanel v-model="state.options" :action="state.action" />
        </div>
      </div>
    </div>

    <!-- Summary Bar -->
    <div class="transfer-summary-bar">
      <div class="flex flex-wrap gap-6 items-center">
        <div class="transfer-summary-stat">
          <span>ACTION</span>
          <strong>{{ state.action }}</strong>
        </div>
        <div class="transfer-summary-stat">
          <span>SCOPE</span>
          <strong>{{ state.source.scope || '—' }}</strong>
        </div>
        <div v-if="state.source.database" class="transfer-summary-stat">
          <span>DB</span>
          <strong>{{ state.source.database }}</strong>
        </div>
        <div v-if="state.source.tables?.length" class="transfer-summary-stat">
          <span>TABLES</span>
          <strong>{{ state.source.tables.length }}</strong>
        </div>
        <div v-if="state.options.format || state.options.fileFormat" class="transfer-summary-stat">
          <span>FORMAT</span>
          <strong>{{ state.options.format || state.options.fileFormat }}</strong>
        </div>
        <div v-if="state.options.destination" class="transfer-summary-stat">
          <span>DEST</span>
          <strong>{{ state.options.destination.split('/').pop() || state.options.destination }}</strong>
        </div>
      </div>
      <div class="transfer-summary-stat">
        <span>STATUS</span>
        <strong :class="isActionValid ? 'text-green-500' : 'text-muted-foreground'">
          {{ isActionValid ? 'READY' : 'INCOMPLETE' }}
        </strong>
      </div>
    </div>

    <!-- Start Button Row -->
    <div class="pt-2 flex gap-3 justify-end">
      <Button variant="outline">
        {{ t('common.cancel') }}
      </Button>
      <Button :disabled="!isActionValid" @click="handleStart">
        {{ t(`transfer.launcher.actions.${state.action}.start`) }}
        <span class="i-carbon-arrow-right ml-2" />
      </Button>
    </div>
  </div>
</template>

<style scoped>
</style>
