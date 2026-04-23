<!--
  Visual Role: Configuration form for source/destination connections.
  Tight grid layout with small uppercase labels and mono-styled status badges.
-->
<script setup lang="ts">
import type { DatabaseSchema } from '@/store/databaseStore'
import { computed, ref, watch } from 'vue'
import { useI18n } from 'vue-i18n'

import { Label } from '@/components/ui/label'
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from '@/components/ui/select'

import { toast } from '@/composables/useNotifications'
import { ConnectionStatus, useConnectionStore } from '@/store/connectionStore'
import { useDatabaseStore } from '@/store/databaseStore'

const props = defineProps<{
  connectionId?: string
  database?: string
  schema?: string
  showSchema?: boolean
}>()

const emit = defineEmits<{
  'update:connectionId': [value: string]
  'update:database': [value: string]
  'update:schema': [value: string]
}>()

const { t } = useI18n()

const connectionStore = useConnectionStore()
const databaseStore = useDatabaseStore()

const isConnecting = ref(false)
const loadingDatabases = ref(false)
const loadingSchemas = ref(false)

const selectedConnectionId = computed({
  get: () => props.connectionId || '',
  set: val => emit('update:connectionId', val),
})

const selectedDatabase = computed({
  get: () => props.database || '',
  set: val => emit('update:database', val),
})

const selectedSchema = computed({
  get: () => props.schema || '',
  set: val => emit('update:schema', val),
})

// Show ALL connections, with status indicator
const allConnections = computed(() => connectionStore.connections)

const selectedConnection = computed(() =>
  connectionStore.getConnectionById(selectedConnectionId.value),
)

const isConnected = computed(() => {
  if (!selectedConnectionId.value)
    return false
  return connectionStore.getConnectionStatus(selectedConnectionId.value) === ConnectionStatus.CONNECTED
})

// Fetch real databases from server
const databases = computed<DatabaseSchema[]>(() => {
  if (!selectedConnectionId.value || !isConnected.value)
    return []
  return databaseStore.metadata[selectedConnectionId.value]?.databases ?? []
})

// Fetch real schemas from server
const schemas = computed<string[]>(() => {
  if (!selectedConnectionId.value || !selectedDatabase.value || !isConnected.value)
    return []
  return databaseStore.metadata[selectedConnectionId.value]?.schemas[selectedDatabase.value] ?? []
})

// Auto-connect when connection is selected but not connected
async function handleConnectionSelect(connId: string) {
  if (!connId)
    return

  selectedConnectionId.value = connId

  const status = connectionStore.getConnectionStatus(connId)
  if (status !== ConnectionStatus.CONNECTED) {
    isConnecting.value = true
    try {
      await connectionStore.connect(connId)
      // Fetch databases after connecting
      await fetchDatabases(connId)
    }
    catch (error) {
      toast.error(t('transfer.connection.connectFailed'), {
        description: error instanceof Error ? error.message : String(error),
      })
      isConnecting.value = false
      return
    }
    finally {
      isConnecting.value = false
    }
  }
  else {
    // Already connected, fetch databases
    await fetchDatabases(connId)
  }
}

async function fetchDatabases(connId: string) {
  if (!connId)
    return

  loadingDatabases.value = true
  try {
    await databaseStore.fetchDatabases(connId)

    // Auto-select first database if none selected
    const dbs = databaseStore.metadata[connId]?.databases ?? []
    if (!selectedDatabase.value && dbs.length > 0) {
      selectedDatabase.value = dbs[0].name
    }
  }
  catch (error) {
    toast.error(t('transfer.connection.databasesFailed'), {
      description: error instanceof Error ? error.message : String(error),
    })
  }
  finally {
    loadingDatabases.value = false
  }
}

async function fetchSchemas(connId: string, db: string) {
  if (!connId || !db)
    return

  loadingSchemas.value = true
  try {
    await databaseStore.fetchSchemas(connId, db)

    // Auto-select first schema if none selected
    const schemaList = databaseStore.metadata[connId]?.schemas[db] ?? []
    if (!selectedSchema.value && schemaList.length > 0) {
      selectedSchema.value = schemaList[0]
    }
  }
  catch (error) {
    toast.error(t('transfer.connection.schemasFailed'), {
      description: error instanceof Error ? error.message : String(error),
    })
  }
  finally {
    loadingSchemas.value = false
  }
}

// Watch for connection changes
watch(selectedConnectionId, (newId, oldId) => {
  if (newId && newId !== oldId) {
    // Reset database and schema when connection changes
    selectedDatabase.value = ''
    selectedSchema.value = ''
    handleConnectionSelect(newId)
  }
}, { immediate: true })

// Watch for database changes to fetch schemas
watch(selectedDatabase, (newDb, oldDb) => {
  if (newDb && newDb !== oldDb && selectedConnectionId.value && isConnected.value) {
    selectedSchema.value = '' // Reset schema when database changes
    fetchSchemas(selectedConnectionId.value, newDb)
  }
})

// Whether to show schema selector (PostgreSQL typically has schemas)
const shouldShowSchema = computed(() => {
  if (!props.showSchema)
    return false
  // PostgreSQL and some others use schemas, MySQL/SQLite typically don't
  const conn = selectedConnection.value
  if (!conn)
    return false
  return conn.type === 'POSTGRESQL' || conn.type === 'MARIADB' || schemas.value.length > 0
})
</script>

<template>
  <div class="space-y-3">
    <!-- No connections message -->
    <div v-if="allConnections.length === 0" class="p-6 text-center border rounded-md border-dashed bg-muted/20 flex flex-col items-center justify-center">
      <span class="i-carbon-api-1 text-muted-foreground mb-2 opacity-50 h-6 w-6" />
      <p class="text-xs text-muted-foreground font-medium">
        No connections available. Please add a connection first.
      </p>
    </div>

    <div v-else class="space-y-3">
      <div class="space-y-1.5">
        <Label class="text-[11px] text-muted-foreground tracking-wide font-medium uppercase">Connection</Label>
        <Select
          :model-value="selectedConnectionId"
          :disabled="isConnecting"
          @update:model-value="handleConnectionSelect"
        >
          <SelectTrigger class="text-xs border-border/40 bg-muted/20 h-8">
            <SelectValue placeholder="Select connection">
              <template v-if="selectedConnection">
                <span class="font-mono">{{ selectedConnection.name }}</span>
                <span
                  v-if="isConnected"
                  class="text-[9px] text-green-600 font-medium ml-1.5"
                >●</span>
                <span
                  v-else-if="isConnecting"
                  class="text-[9px] text-muted-foreground ml-1.5 animate-pulse"
                >●</span>
              </template>
            </SelectValue>
          </SelectTrigger>
          <SelectContent>
            <SelectItem
              v-for="conn in allConnections"
              :key="conn.id!"
              :value="conn.id!"
              class="text-xs"
            >
              <div class="flex gap-2 items-center">
                <span>{{ conn.name }}</span>
                <span
                  v-if="connectionStore.getConnectionStatus(conn.id!) === ConnectionStatus.CONNECTED"
                  class="text-[10px] text-green-600 tracking-wide font-mono px-1 rounded-sm bg-green-500/10 uppercase"
                >
                  Connected
                </span>
                <span
                  v-else
                  class="text-[10px] text-muted-foreground tracking-wide font-mono px-1 rounded-sm bg-muted/60 uppercase"
                >
                  Offline
                </span>
                <span class="text-[10px] text-muted-foreground font-mono ml-auto">{{ conn.type }}</span>
              </div>
            </SelectItem>
          </SelectContent>
        </Select>
      </div>

      <div class="space-y-1.5">
        <Label class="text-[11px] text-muted-foreground tracking-wide font-medium uppercase">Database</Label>
        <Select
          v-model="selectedDatabase"
          :disabled="!isConnected || loadingDatabases || databases.length === 0"
        >
          <SelectTrigger class="text-xs border-border/40 bg-muted/20 h-8">
            <SelectValue placeholder="Select database" />
          </SelectTrigger>
          <SelectContent>
            <SelectItem
              v-for="db in databases"
              :key="db.name"
              :value="db.name"
              class="text-xs"
            >
              <div class="flex gap-2 w-full items-center">
                <span>{{ db.name }}</span>
                <span
                  v-if="db.is_system"
                  class="text-[10px] text-muted-foreground tracking-wide font-mono ml-auto px-1 rounded-sm bg-muted/60 uppercase"
                >
                  System
                </span>
              </div>
            </SelectItem>
          </SelectContent>
        </Select>
        <div v-if="loadingDatabases" class="text-[10px] text-muted-foreground flex items-center">
          <span class="i-carbon-circle-dash mr-1 animate-spin" /> Loading databases...
        </div>
      </div>

      <div v-if="shouldShowSchema" class="space-y-1.5">
        <Label class="text-[11px] text-muted-foreground tracking-wide font-medium uppercase">Schema</Label>
        <Select
          v-model="selectedSchema"
          :disabled="!selectedDatabase || loadingSchemas || schemas.length === 0"
        >
          <SelectTrigger class="text-xs border-border/40 bg-muted/20 h-8">
            <SelectValue placeholder="Select schema" />
          </SelectTrigger>
          <SelectContent>
            <SelectItem
              v-for="s in schemas"
              :key="s"
              :value="s"
              class="text-xs"
            >
              {{ s }}
            </SelectItem>
          </SelectContent>
        </Select>
        <div v-if="loadingSchemas" class="text-[10px] text-muted-foreground flex items-center">
          <span class="i-carbon-circle-dash mr-1 animate-spin" /> Loading schemas...
        </div>
      </div>
    </div>
  </div>
</template>
