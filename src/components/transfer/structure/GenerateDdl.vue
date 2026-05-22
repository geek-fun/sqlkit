<script setup lang="ts">
import type { DdlOptions } from '@/types/transfer'

import { invoke } from '@tauri-apps/api/core'
import { save } from '@tauri-apps/plugin-dialog'
import { computed, ref, watch } from 'vue'

import { Button } from '@/components/ui/button'
import { Checkbox } from '@/components/ui/checkbox'
import { Label } from '@/components/ui/label'
import { generateDdl as generateDdlApi } from '@/datasources/transferApi'

import { ConnectionStatus, useConnectionStore } from '@/store/connectionStore'
import ConnectionSelector from '../shared/ConnectionSelector.vue'
import TransferStepCard from '../shared/TransferStepCard.vue'

const connectionStore = useConnectionStore()

const connectionId = ref('')
const database = ref('')
const schema = ref('')
const selectedObjects = ref<string[]>([])
const generatedDdl = ref('')
const loadingObjects = ref(false)
const loadingDdl = ref(false)

const objects = ref<{ name: string, objectType: string }[]>([])

const ddlOptions = ref<DdlOptions>({
  includeCreateTable: true,
  includePrimaryKeys: true,
  includeForeignKeys: false,
  includeIndexes: false,
  includeConstraints: true,
  includeDropIfExists: true,
  includeIfNotExists: false,
})

// Check connection status
const isConnected = computed(() => {
  if (!connectionId.value)
    return false
  return connectionStore.getConnectionStatus(connectionId.value) === ConnectionStatus.CONNECTED
})

// Summary for display
const selectionSummary = computed(() => {
  if (selectedObjects.value.length)
    return `${selectedObjects.value.length} objects`
  return ''
})

// Load objects when database changes
const ddlFetchParams = computed(() => {
  if (!isConnected.value || !connectionId.value || !database.value)
    return null
  return {
    connectionId: connectionId.value,
    database: database.value,
    schema: schema.value,
  }
})

watch(ddlFetchParams, async (params, oldParams) => {
  if (params && JSON.stringify(params) !== JSON.stringify(oldParams)) {
    loadingObjects.value = true
    try {
      const tables = await invoke<{ name: string, schema?: string }[]>('list_tables', {
        connectionId: params.connectionId,
        database: params.database,
        schema: params.schema || null,
      })
      objects.value = tables.map(table => ({
        name: table.name,
        objectType: 'table',
      }))
    }
    catch (error) {
      console.error('Failed to load objects:', error)
    }
    finally {
      loadingObjects.value = false
    }
  }
}, { deep: true })

// Toggle object selection
function toggleObject(name: string) {
  const current = [...selectedObjects.value]
  const index = current.indexOf(name)
  if (index > -1) {
    current.splice(index, 1)
  }
  else {
    current.push(name)
  }
  selectedObjects.value = current
}

function selectAll() {
  selectedObjects.value = objects.value.map(o => o.name)
}

function deselectAll() {
  selectedObjects.value = []
}

// Generate DDL
async function generateDdl() {
  if (!connectionId.value || selectedObjects.value.length === 0)
    return

  loadingDdl.value = true
  try {
    const result = await generateDdlApi({
      connectionId: connectionId.value,
      database: database.value || undefined,
      schema: schema.value || undefined,
      objects: objects.value
        .filter(o => selectedObjects.value.includes(o.name))
        .map(o => ({ name: o.name, objectType: 'table' as const, schema: schema.value || undefined })),
      options: ddlOptions.value,
    })
    generatedDdl.value = result
  }
  catch (error) {
    console.error('Failed to generate DDL:', error)
  }
  finally {
    loadingDdl.value = false
  }
}

// Copy to clipboard
async function copyToClipboard() {
  await navigator.clipboard.writeText(generatedDdl.value)
}

// Save to file
async function saveToFile() {
  try {
    const path = await save({
      defaultPath: 'ddl.sql',
      filters: [{ name: 'SQL', extensions: ['sql'] }],
    })
    if (path) {
      await invoke('write_text_file', { path, content: generatedDdl.value })
    }
  }
  catch (error) {
    console.error('Failed to save file:', error)
  }
}

const canGenerate = computed(() =>
  connectionId.value !== '' && isConnected.value && selectedObjects.value.length > 0,
)
</script>

<template>
  <div class="pb-8 flex flex-col gap-4">
    <!-- Source -->
    <TransferStepCard
      title="Source Database"
      :step-number="1"
      icon="i-carbon-data-base"
      icon-class="text-emerald-600 dark:text-emerald-500"
      :summary="selectionSummary"
    >
      <ConnectionSelector
        v-model:connection-id="connectionId"
        v-model:database="database"
        v-model:schema="schema"
        show-schema
      />

      <!-- Objects List -->
      <div class="mt-6 pt-4 border-t border-border/40">
        <div class="mb-3 flex items-center justify-between">
          <Label class="text-[11px] text-muted-foreground tracking-wide font-semibold flex gap-1.5 uppercase items-center">
            <span class="i-carbon-list" />
            Objects
          </Label>
          <div class="flex gap-2 items-center">
            <Button variant="ghost" size="sm" class="text-xs px-2 h-8" @click="selectAll">
              Select All
            </Button>
            <Button variant="ghost" size="sm" class="text-xs px-2 h-8" @click="deselectAll">
              Deselect All
            </Button>
          </div>
        </div>

        <div v-if="loadingObjects" class="text-xs text-muted-foreground p-4 border border-border/40 rounded-md border-dashed flex items-center justify-center">
          <span class="i-carbon-circle-dash mr-2 animate-spin" /> Loading objects...
        </div>

        <div v-else-if="objects.length === 0 && connectionId" class="text-xs text-muted-foreground p-6 text-center border border-border/40 rounded-md border-dashed bg-muted/10 flex flex-col items-center justify-center">
          <span class="i-carbon-data-base mb-2 opacity-50 h-5 w-5" />
          No objects found
        </div>

        <div v-else class="scrollbar-thin scrollbar-track-transparent scrollbar-thumb-border pr-2 gap-2 grid grid-cols-1 max-h-[300px] overflow-y-auto md:grid-cols-3 sm:grid-cols-2">
          <label
            v-for="obj in objects"
            :key="obj.name"
            class="px-3 py-2 border border-border/40 rounded-md flex cursor-pointer transition-colors items-center space-x-2.5 hover:bg-muted/50"
            :class="selectedObjects.includes(obj.name) ? 'border-primary/30 bg-primary/5' : 'bg-transparent'"
          >
            <Checkbox
              :id="`ddl-obj-${obj.name}`"
              :checked="selectedObjects.includes(obj.name)"
              @update:checked="toggleObject(obj.name)"
            />
            <div class="flex flex-col">
              <span class="text-xs leading-none font-mono">{{ obj.name }}</span>
              <span class="text-[10px] text-muted-foreground tracking-wide mt-1 uppercase">{{ obj.objectType }}</span>
            </div>
          </label>
        </div>

        <div v-if="objects.length > 0" class="text-[11px] text-muted-foreground font-mono mt-3 tabular-nums">
          {{ selectedObjects.length }} / {{ objects.length }} objects selected
        </div>
      </div>
    </TransferStepCard>

    <!-- Options -->
    <TransferStepCard
      title="DDL Options"
      :step-number="2"
      icon="i-carbon-settings"
      icon-class="text-amber-600 dark:text-amber-500"
    >
      <div class="flex flex-wrap gap-2">
        <label class="px-2 py-1 rounded-sm bg-muted flex cursor-pointer transition-colors items-center space-x-1.5 hover:bg-muted/80" :class="ddlOptions.includeCreateTable ? 'bg-primary/10 text-primary border border-primary/30' : 'border border-transparent'">
          <Checkbox id="ddl-opt-create" v-model:checked="ddlOptions.includeCreateTable" class="h-3 w-3" />
          <span class="text-[11px] leading-none font-medium">CREATE TABLE</span>
        </label>
        <label class="px-2 py-1 rounded-sm bg-muted flex cursor-pointer transition-colors items-center space-x-1.5 hover:bg-muted/80" :class="ddlOptions.includePrimaryKeys ? 'bg-primary/10 text-primary border border-primary/30' : 'border border-transparent'">
          <Checkbox id="ddl-opt-pk" v-model:checked="ddlOptions.includePrimaryKeys" class="h-3 w-3" />
          <span class="text-[11px] leading-none font-medium">Primary Keys</span>
        </label>
        <label class="px-2 py-1 rounded-sm bg-muted flex cursor-pointer transition-colors items-center space-x-1.5 hover:bg-muted/80" :class="ddlOptions.includeForeignKeys ? 'bg-primary/10 text-primary border border-primary/30' : 'border border-transparent'">
          <Checkbox id="ddl-opt-fk" v-model:checked="ddlOptions.includeForeignKeys" class="h-3 w-3" />
          <span class="text-[11px] leading-none font-medium">Foreign Keys</span>
        </label>
        <label class="px-2 py-1 rounded-sm bg-muted flex cursor-pointer transition-colors items-center space-x-1.5 hover:bg-muted/80" :class="ddlOptions.includeIndexes ? 'bg-primary/10 text-primary border border-primary/30' : 'border border-transparent'">
          <Checkbox id="ddl-opt-idx" v-model:checked="ddlOptions.includeIndexes" class="h-3 w-3" />
          <span class="text-[11px] leading-none font-medium">Indexes</span>
        </label>
        <label class="px-2 py-1 rounded-sm bg-muted flex cursor-pointer transition-colors items-center space-x-1.5 hover:bg-muted/80" :class="ddlOptions.includeConstraints ? 'bg-primary/10 text-primary border border-primary/30' : 'border border-transparent'">
          <Checkbox id="ddl-opt-const" v-model:checked="ddlOptions.includeConstraints" class="h-3 w-3" />
          <span class="text-[11px] leading-none font-medium">Constraints</span>
        </label>
        <label class="px-2 py-1 rounded-sm bg-muted flex cursor-pointer transition-colors items-center space-x-1.5 hover:bg-muted/80" :class="ddlOptions.includeDropIfExists ? 'bg-primary/10 text-primary border border-primary/30' : 'border border-transparent'">
          <Checkbox id="ddl-opt-drop" v-model:checked="ddlOptions.includeDropIfExists" class="h-3 w-3" />
          <span class="text-[11px] leading-none font-medium">DROP IF EXISTS</span>
        </label>
      </div>

      <div class="mt-6 pt-4 border-t border-border/40 flex justify-end">
        <Button :disabled="!canGenerate" size="sm" class="h-8 min-w-[120px]" @click="generateDdl">
          <span v-if="loadingDdl" class="i-carbon-circle-dash mr-2 animate-spin" />
          <span v-else class="i-carbon-document-add mr-2" />
          Generate DDL
        </Button>
      </div>
    </TransferStepCard>

    <!-- Output -->
    <TransferStepCard
      v-if="generatedDdl"
      title="Generated DDL"
      :step-number="3"
      icon="i-carbon-document"
      icon-class="text-blue-600 dark:text-blue-500"
      variant="highlight"
    >
      <div class="scrollbar-thin scrollbar-track-transparent scrollbar-thumb-border text-xs leading-snug font-mono p-3 rounded-md bg-muted/40 max-h-[400px] shadow-sm overflow-auto">
        <pre class="whitespace-pre-wrap">{{ generatedDdl }}</pre>
      </div>
      <div class="mt-4 flex gap-2 justify-end">
        <Button variant="outline" size="sm" class="h-8" @click="copyToClipboard">
          <span class="i-carbon-copy mr-1.5" /> Copy to Clipboard
        </Button>
        <Button variant="outline" size="sm" class="h-8" @click="saveToFile">
          <span class="i-carbon-save mr-1.5" /> Save to File
        </Button>
      </div>
    </TransferStepCard>
  </div>
</template>
