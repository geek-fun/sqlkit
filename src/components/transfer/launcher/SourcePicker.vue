<script setup lang="ts">
import type { LauncherScope, LauncherSource } from './types'
import type { DatabaseSchema, TableInfo } from '@/store/databaseStore'
import { invoke } from '@tauri-apps/api/core'
import { computed, ref, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import { Checkbox } from '@/components/ui/checkbox'

import { Label } from '@/components/ui/label'
import { RadioGroup, RadioGroupItem } from '@/components/ui/radio-group'
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from '@/components/ui/select'
import { useConnectionStore } from '@/store/connectionStore'

const props = defineProps<{
  modelValue: LauncherSource
}>()

const emit = defineEmits<{
  (e: 'update:modelValue', value: LauncherSource): void
}>()

const { t } = useI18n()
const connectionStore = useConnectionStore()
const connections = computed(() => connectionStore.connections)

const state = computed({
  get: () => props.modelValue,
  set: val => emit('update:modelValue', val),
})

const databases = ref<DatabaseSchema[]>([])
const schemas = ref<string[]>([])
const tables = ref<TableInfo[]>([])

const isLoadingDb = ref(false)
const isLoadingSchema = ref(false)
const isLoadingTable = ref(false)

const dbReqId = ref(0)
const schemaReqId = ref(0)
const tableReqId = ref(0)

async function loadDatabases(connectionId: string) {
  const myId = ++dbReqId.value
  try {
    isLoadingDb.value = true
    const result = await invoke<DatabaseSchema[]>('list_databases', { connectionId })
    if (myId !== dbReqId.value)
      return
    databases.value = result.filter(db => !db.is_system)
  }
  catch (e) {
    if (myId !== dbReqId.value)
      return
    console.error('Failed to load databases', e)
    databases.value = []
  }
  finally {
    if (myId === dbReqId.value)
      isLoadingDb.value = false
  }
}

async function loadSchemas(connectionId: string, database: string) {
  const myId = ++schemaReqId.value
  try {
    isLoadingSchema.value = true
    const result = await invoke<string[]>('list_schemas', { connectionId, database })
    if (myId !== schemaReqId.value)
      return
    schemas.value = result

    if (result.length === 1 && state.value.schema !== result[0]) {
      state.value = { ...state.value, schema: result[0] }
    }
  }
  catch (e) {
    if (myId !== schemaReqId.value)
      return
    console.error('Failed to load schemas', e)
    schemas.value = []
  }
  finally {
    if (myId === schemaReqId.value)
      isLoadingSchema.value = false
  }
}

async function loadTables(connectionId: string, database: string, schema?: string) {
  const myId = ++tableReqId.value
  try {
    isLoadingTable.value = true
    const result = await invoke<TableInfo[]>('list_tables', { connectionId, database, schema: schema || undefined })
    if (myId !== tableReqId.value)
      return
    tables.value = result
  }
  catch (e) {
    if (myId !== tableReqId.value)
      return
    console.error('Failed to load tables', e)
    tables.value = []
  }
  finally {
    if (myId === tableReqId.value)
      isLoadingTable.value = false
  }
}

watch(() => state.value.connectionId, (newId) => {
  state.value = { ...state.value, database: undefined, schema: undefined, tables: [] }
  if (newId && state.value.scope && state.value.scope !== 'server') {
    loadDatabases(newId)
  }
})

watch(() => state.value.scope, (newScope) => {
  state.value = { ...state.value, database: undefined, schema: undefined, tables: [] }
  if (state.value.connectionId && newScope && newScope !== 'server') {
    loadDatabases(state.value.connectionId)
  }
})

watch(() => state.value.database, (newDb) => {
  state.value = { ...state.value, schema: undefined, tables: [] }
  if (state.value.connectionId && newDb && state.value.scope === 'table') {
    loadSchemas(state.value.connectionId, newDb)
  }
})

watch(() => state.value.schema, (newSchema) => {
  state.value = { ...state.value, tables: [] }
  if (state.value.connectionId && state.value.database && state.value.scope === 'table') {
    loadTables(state.value.connectionId, state.value.database, newSchema)
  }
})

watch([() => schemas.value, () => isLoadingSchema.value], ([s, loading]) => {
  if (!loading && s.length === 0 && state.value.connectionId && state.value.database && state.value.scope === 'table') {
    loadTables(state.value.connectionId, state.value.database)
  }
})

function handleTableToggle(tableName: string, checked: boolean) {
  const current = state.value.tables || []
  if (checked) {
    state.value = { ...state.value, tables: [...current, tableName] }
  }
  else {
    state.value = { ...state.value, tables: current.filter((t: string) => t !== tableName) }
  }
}

function selectAllTables() {
  state.value = { ...state.value, tables: tables.value.map(t => t.name) }
}

function clearAllTables() {
  state.value = { ...state.value, tables: [] }
}
</script>

<template>
  <div class="p-4 border rounded-md bg-muted/20 space-y-4">
    <div class="gap-4 grid grid-cols-[120px_1fr] items-center">
      <Label class="text-muted-foreground text-right">{{ t('transfer.launcher.connection') }}</Label>
      <Select :model-value="state.connectionId || ''" @update:model-value="(v) => state = { ...state, connectionId: v }">
        <SelectTrigger class="w-[300px]">
          <SelectValue :placeholder="t('transfer.launcher.selectConnection')" />
        </SelectTrigger>
        <SelectContent>
          <SelectItem v-for="conn in connections" :key="conn.id" :value="conn.id || ''">
            {{ conn.name }}
          </SelectItem>
        </SelectContent>
      </Select>
    </div>

    <div class="gap-4 grid grid-cols-[120px_1fr] items-center">
      <Label class="text-muted-foreground text-right">{{ t('transfer.launcher.scope') }}</Label>
      <RadioGroup :model-value="state.scope" class="flex gap-4" @update:model-value="(v) => state = { ...state, scope: v as LauncherScope }">
        <div class="flex items-center space-x-2">
          <RadioGroupItem id="scope-server" value="server" />
          <Label for="scope-server">{{ t('transfer.launcher.scopes.server') }}</Label>
        </div>
        <div class="flex items-center space-x-2">
          <RadioGroupItem id="scope-database" value="database" />
          <Label for="scope-database">{{ t('transfer.launcher.scopes.database') }}</Label>
        </div>
        <div class="flex items-center space-x-2">
          <RadioGroupItem id="scope-table" value="table" />
          <Label for="scope-table">{{ t('transfer.launcher.scopes.table') }}</Label>
        </div>
      </RadioGroup>
    </div>

    <div v-if="state.scope === 'database' || state.scope === 'table'" class="gap-4 grid grid-cols-[120px_1fr] items-center">
      <Label class="text-muted-foreground text-right">{{ t('transfer.launcher.database') }}</Label>
      <div class="flex gap-2 items-center">
        <Select :model-value="state.database || ''" :disabled="!state.connectionId || isLoadingDb" @update:model-value="(v) => state = { ...state, database: v }">
          <SelectTrigger class="w-[300px]">
            <SelectValue :placeholder="t('transfer.launcher.selectDatabase')" />
          </SelectTrigger>
          <SelectContent>
            <SelectItem v-for="db in databases" :key="db.name" :value="db.name">
              {{ db.name }}
            </SelectItem>
          </SelectContent>
        </Select>
        <span v-if="isLoadingDb" class="i-carbon-circle-dash text-muted-foreground animate-spin" />
      </div>
    </div>

    <div v-if="state.scope === 'table' && schemas.length > 0" class="gap-4 grid grid-cols-[120px_1fr] items-center">
      <Label class="text-muted-foreground text-right">{{ t('transfer.launcher.schema') }}</Label>
      <div class="flex gap-2 items-center">
        <Select :model-value="state.schema || ''" :disabled="!state.database || isLoadingSchema" @update:model-value="(v) => state = { ...state, schema: v }">
          <SelectTrigger class="w-[300px]">
            <SelectValue :placeholder="t('transfer.launcher.selectSchema')" />
          </SelectTrigger>
          <SelectContent>
            <SelectItem v-for="schema in schemas" :key="schema" :value="schema">
              {{ schema }}
            </SelectItem>
          </SelectContent>
        </Select>
        <span v-if="isLoadingSchema" class="i-carbon-circle-dash text-muted-foreground animate-spin" />
      </div>
    </div>

    <div v-if="state.scope === 'table'" class="gap-4 grid grid-cols-[120px_1fr]">
      <Label class="text-muted-foreground pt-2 text-right">{{ t('transfer.launcher.tables') }}</Label>
      <div class="border rounded-md bg-background max-w-xl w-full">
        <div class="p-2 border-b bg-muted/30 flex items-center justify-between">
          <span class="text-xs text-muted-foreground ml-2">
            {{ state.tables?.length || 0 }} {{ t('transfer.launcher.selected') }}
          </span>
          <div class="text-xs flex gap-2">
            <button class="transition-colors hover:text-primary" @click="selectAllTables">
              {{ t('transfer.launcher.selectAll') }}
            </button>
            <button class="transition-colors hover:text-primary" @click="clearAllTables">
              {{ t('transfer.launcher.clear') }}
            </button>
          </div>
        </div>
        <div class="p-2 max-h-[200px] overflow-y-auto space-y-1">
          <div v-if="isLoadingTable" class="p-4 flex justify-center">
            <span class="i-carbon-circle-dash text-xl text-muted-foreground animate-spin" />
          </div>
          <div v-else-if="tables.length === 0" class="text-sm text-muted-foreground p-4 text-center">
            {{ t('transfer.launcher.noTables') }}
          </div>
          <label v-for="table in tables" :key="table.name" class="p-1.5 rounded flex gap-2 cursor-pointer items-center hover:bg-muted/50">
            <Checkbox :checked="state.tables?.includes(table.name)" @update:checked="(v) => handleTableToggle(table.name, !!v)" />
            <span class="text-sm font-medium">{{ table.name }}</span>
          </label>
        </div>
      </div>
    </div>
  </div>
</template>
