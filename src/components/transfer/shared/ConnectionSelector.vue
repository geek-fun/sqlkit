<script setup lang="ts">
import { computed, ref, watch } from 'vue'

import { Label } from '@/components/ui/label'
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from '@/components/ui/select'

import { useConnectionStore } from '@/store/connectionStore'

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

const connectionStore = useConnectionStore()

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

const connectedConnections = computed(() =>
  connectionStore.connections.filter(c => c.isConnected),
)

const activeConnection = computed(() =>
  connectionStore.getConnectionById(selectedConnectionId.value),
)

const databases = ref<string[]>([])
const schemas = ref<string[]>([])

watch(selectedConnectionId, async (newId) => {
  if (newId && activeConnection.value) {
    databases.value = [activeConnection.value.database || '']
    if (!selectedDatabase.value && databases.value[0]) {
      selectedDatabase.value = databases.value[0]
    }
  }
}, { immediate: true })

watch(selectedDatabase, async (newDb) => {
  if (newDb) {
    schemas.value = ['public']
    if (!selectedSchema.value) {
      selectedSchema.value = schemas.value[0]
    }
  }
}, { immediate: true })
</script>

<template>
  <div class="space-y-4">
    <div class="gap-4 grid grid-cols-3">
      <div class="space-y-2">
        <Label>Connection</Label>
        <Select v-model="selectedConnectionId">
          <SelectTrigger>
            <SelectValue placeholder="Select connection" />
          </SelectTrigger>
          <SelectContent>
            <SelectItem
              v-for="conn in connectedConnections"
              :key="conn.id"
              :value="conn.id!"
            >
              {{ conn.name }}
            </SelectItem>
          </SelectContent>
        </Select>
      </div>

      <div class="space-y-2">
        <Label>Database</Label>
        <Select v-model="selectedDatabase" :disabled="!selectedConnectionId">
          <SelectTrigger>
            <SelectValue placeholder="Select database" />
          </SelectTrigger>
          <SelectContent>
            <SelectItem
              v-for="db in databases"
              :key="db"
              :value="db"
            >
              {{ db }}
            </SelectItem>
          </SelectContent>
        </Select>
      </div>

      <div v-if="showSchema" class="space-y-2">
        <Label>Schema</Label>
        <Select v-model="selectedSchema" :disabled="!selectedDatabase">
          <SelectTrigger>
            <SelectValue placeholder="Select schema" />
          </SelectTrigger>
          <SelectContent>
            <SelectItem
              v-for="s in schemas"
              :key="s"
              :value="s"
            >
              {{ s }}
            </SelectItem>
          </SelectContent>
        </Select>
      </div>
    </div>
  </div>
</template>
