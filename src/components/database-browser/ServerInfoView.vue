<script setup lang="ts">
import { computed } from 'vue'
import { ConnectionStatus, useConnectionStore, useDatabaseStore } from '@/store'
import DbTypeIcon from './DbTypeIcon.vue'

const props = defineProps<{
  connectionId?: string
  selectedDatabase?: string
}>()

const connectionStore = useConnectionStore()
const databaseStore = useDatabaseStore()

const activeConnection = computed(() =>
  props.connectionId ? connectionStore.getConnectionById(props.connectionId) : connectionStore.activeConnection,
)

const isConnected = computed(() =>
  props.connectionId
    ? connectionStore.getConnectionStatus(props.connectionId) === ConnectionStatus.CONNECTED
    : false,
)

const databaseCount = computed(() =>
  props.connectionId
    ? databaseStore.metadata[props.connectionId]?.databases.length ?? 0
    : 0,
)

const currentDatabase = computed(() =>
  props.selectedDatabase
  || (props.connectionId ? connectionStore.getCurrentDatabase(props.connectionId) : '')
  || activeConnection.value?.database
  || '',
)
</script>

<template>
  <div v-if="!activeConnection" class="p-3 text-center">
    <p class="text-xs text-muted-foreground">
      No connection selected
    </p>
  </div>
  <div v-else class="p-2 space-y-2">
    <div class="flex gap-2 items-center">
      <DbTypeIcon :type="activeConnection.type" :size="16" />
      <span class="text-sm font-semibold truncate">{{ activeConnection.name }}</span>
      <span
        v-if="isConnected"
        class="text-[10px] text-green-600 font-medium px-1.5 py-0.5 rounded-full bg-green-500/20 dark:text-green-400"
      >
        connected
      </span>
    </div>

    <div class="text-xs text-muted-foreground space-y-0.5">
      <p>{{ activeConnection.host }}:{{ activeConnection.port }}</p>
      <p v-if="activeConnection.username">
        {{ activeConnection.username }}
      </p>
    </div>

    <div v-if="isConnected" class="pt-1 border-t space-y-0.5">
      <div class="text-xs flex items-center justify-between">
        <span class="text-muted-foreground">Databases</span>
        <span class="font-medium">{{ databaseCount }}</span>
      </div>
      <div v-if="currentDatabase" class="text-xs flex items-center justify-between">
        <span class="text-muted-foreground">Current</span>
        <span class="font-medium ml-2 truncate">{{ currentDatabase }}</span>
      </div>
    </div>
  </div>
</template>
