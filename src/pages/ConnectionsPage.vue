<script setup lang="ts">
import type { ServerConnection } from '@/store'
import { computed, onMounted, ref } from 'vue'
import { ServerCard, ServerFormDialog } from '@/components/connections'
import AppLayout from '@/components/layout/AppLayout.vue'
import {
  AlertDialog,
  AlertDialogAction,
  AlertDialogCancel,
  AlertDialogContent,
  AlertDialogDescription,
  AlertDialogFooter,
  AlertDialogHeader,
  AlertDialogTitle,
} from '@/components/ui/alert-dialog'
import { Button } from '@/components/ui/button'
import { Card } from '@/components/ui/card'
import { Input } from '@/components/ui/input'
import { ConnectionStatus, useConnectionStore } from '@/store'

const connectionStore = useConnectionStore()

const searchQuery = ref('')
const viewMode = ref<'grid' | 'list'>('grid')
const isFormDialogOpen = ref(false)
const editingConnection = ref<ServerConnection | null>(null)
const deleteDialogOpen = ref(false)
const connectionToDelete = ref<ServerConnection | null>(null)
const connectError = ref<string | null>(null)

const filteredConnections = computed(() => {
  if (!searchQuery.value.trim()) {
    return connectionStore.connections
  }
  const query = searchQuery.value.toLowerCase()
  return connectionStore.connections.filter(
    conn =>
      conn.name.toLowerCase().includes(query)
      || conn.host.toLowerCase().includes(query)
      || conn.type.toLowerCase().includes(query),
  )
})

const stats = computed(() => ({
  total: connectionStore.connections.length,
  active: connectionStore.connectedConnections.length,
}))

onMounted(async () => {
  await connectionStore.fetchConnections()
})

function handleAddConnection() {
  editingConnection.value = null
  isFormDialogOpen.value = true
}

function handleEditConnection(connection: ServerConnection) {
  editingConnection.value = connection
  isFormDialogOpen.value = true
}

async function handleConnect(connection: ServerConnection) {
  connectError.value = null
  if (!connection.id) {
    return
  }

  const status = connectionStore.getConnectionStatus(connection.id)

  if (status === ConnectionStatus.CONNECTED) {
    // Disconnect if already connected
    try {
      await connectionStore.disconnect(connection.id)
    }
    catch (error) {
      connectError.value = error instanceof Error ? error.message : String(error)
    }
  }
  else {
    // Connect
    try {
      await connectionStore.connect(connection.id)
    }
    catch (error) {
      connectError.value = error instanceof Error ? error.message : String(error)
    }
  }
}

function handleDeleteConnection(connection: ServerConnection) {
  connectionToDelete.value = connection
  deleteDialogOpen.value = true
}

async function confirmDelete() {
  if (connectionToDelete.value) {
    await connectionStore.removeConnection(connectionToDelete.value)
    connectionToDelete.value = null
    deleteDialogOpen.value = false
  }
}

function handleDuplicateConnection(connection: ServerConnection) {
  editingConnection.value = {
    ...connection,
    id: undefined,
    name: `${connection.name} (Copy)`,
  }
  isFormDialogOpen.value = true
}

async function handleSaveConnection(connection: ServerConnection) {
  const result = await connectionStore.saveConnection(connection)
  if (!result.success) {
    // Show error notification (could be enhanced with a toast component)
    console.error('Failed to save connection:', result.message)
  }
}

function getConnectionStatus(connectionId: string | undefined): ConnectionStatus {
  if (!connectionId) {
    return ConnectionStatus.DISCONNECTED
  }
  return connectionStore.getConnectionStatus(connectionId)
}
</script>

<template>
  <AppLayout>
    <div class="p-6 h-full relative">
      <div class="space-y-6">
        <!-- Page Header - matching design -->
        <div class="flex items-center justify-between">
          <div class="flex gap-3 items-center">
            <h1 class="text-xl font-semibold">
              Connections
            </h1>
            <span class="text-muted-foreground">|</span>
            <span class="text-sm text-muted-foreground">Manage your database connections</span>
          </div>
          <div class="w-64 relative">
            <svg
              xmlns="http://www.w3.org/2000/svg"
              width="24"
              height="24"
              viewBox="0 0 24 24"
              fill="none"
              stroke="currentColor"
              stroke-width="2"
              stroke-linecap="round"
              stroke-linejoin="round"
              class="text-muted-foreground h-4 w-4 left-3 top-1/2 absolute -translate-y-1/2"
            >
              <circle cx="11" cy="11" r="8" />
              <path d="m21 21-4.3-4.3" />
            </svg>
            <Input
              v-model="searchQuery"
              placeholder="Search connections..."
              class="pl-9 bg-muted/50"
            />
          </div>
        </div>

        <!-- Stats Cards -->
        <div class="gap-4 grid md:grid-cols-3">
          <Card class="p-4">
            <div class="flex items-center justify-between">
              <div>
                <p class="text-sm text-muted-foreground">
                  Total Connections
                </p>
                <p class="text-2xl font-bold">
                  {{ stats.total }}
                </p>
              </div>
              <div class="rounded-lg bg-primary/10 flex h-10 w-10 items-center justify-center">
                <svg
                  xmlns="http://www.w3.org/2000/svg"
                  width="24"
                  height="24"
                  viewBox="0 0 24 24"
                  fill="none"
                  stroke="currentColor"
                  stroke-width="2"
                  stroke-linecap="round"
                  stroke-linejoin="round"
                  class="text-primary h-5 w-5"
                >
                  <ellipse cx="12" cy="5" rx="9" ry="3" />
                  <path d="M3 5v14a9 3 0 0 0 18 0V5" />
                </svg>
              </div>
            </div>
          </Card>
          <Card class="p-4">
            <div class="flex items-center justify-between">
              <div>
                <p class="text-sm text-muted-foreground">
                  Active Sessions
                </p>
                <p class="text-2xl font-bold">
                  {{ stats.active }}
                </p>
              </div>
              <div class="rounded-lg bg-green-100 flex h-10 w-10 items-center justify-center dark:bg-green-900/30">
                <svg
                  xmlns="http://www.w3.org/2000/svg"
                  width="24"
                  height="24"
                  viewBox="0 0 24 24"
                  fill="none"
                  stroke="currentColor"
                  stroke-width="2"
                  stroke-linecap="round"
                  stroke-linejoin="round"
                  class="text-green-600 h-5 w-5 dark:text-green-400"
                >
                  <polygon points="13 2 3 14 12 14 11 22 21 10 12 10 13 2" />
                </svg>
              </div>
            </div>
          </Card>
          <Card class="p-4">
            <div class="flex items-center justify-between">
              <div>
                <p class="text-sm text-muted-foreground">
                  Last Sync
                </p>
                <p class="text-2xl font-bold">
                  2m ago
                </p>
              </div>
              <div class="rounded-lg bg-orange-100 flex h-10 w-10 items-center justify-center dark:bg-orange-900/30">
                <svg
                  xmlns="http://www.w3.org/2000/svg"
                  width="24"
                  height="24"
                  viewBox="0 0 24 24"
                  fill="none"
                  stroke="currentColor"
                  stroke-width="2"
                  stroke-linecap="round"
                  stroke-linejoin="round"
                  class="text-orange-600 h-5 w-5 dark:text-orange-400"
                >
                  <circle cx="12" cy="12" r="10" />
                  <polyline points="12 6 12 12 16 14" />
                </svg>
              </div>
            </div>
          </Card>
        </div>

        <!-- Error notification -->
        <div
          v-if="connectError"
          class="p-4 rounded-md bg-red-50 dark:bg-red-900/20"
        >
          <div class="flex gap-3 items-start">
            <svg
              xmlns="http://www.w3.org/2000/svg"
              width="24"
              height="24"
              viewBox="0 0 24 24"
              fill="none"
              stroke="currentColor"
              stroke-width="2"
              stroke-linecap="round"
              stroke-linejoin="round"
              class="text-red-600 mt-0.5 flex-shrink-0 h-5 w-5 dark:text-red-400"
            >
              <circle cx="12" cy="12" r="10" />
              <line x1="12" x2="12" y1="8" y2="12" />
              <line x1="12" x2="12.01" y1="16" y2="16" />
            </svg>
            <div class="flex-1">
              <h3 class="text-sm text-red-800 font-medium dark:text-red-200">
                Connection Error
              </h3>
              <p class="text-sm text-red-700 mt-1 dark:text-red-300">
                {{ connectError }}
              </p>
            </div>
            <button
              type="button"
              class="text-red-500 hover:text-red-700"
              @click="connectError = null"
            >
              <svg
                xmlns="http://www.w3.org/2000/svg"
                width="24"
                height="24"
                viewBox="0 0 24 24"
                fill="none"
                stroke="currentColor"
                stroke-width="2"
                stroke-linecap="round"
                stroke-linejoin="round"
                class="h-4 w-4"
              >
                <path d="M18 6 6 18" />
                <path d="m6 6 12 12" />
              </svg>
            </button>
          </div>
        </div>

        <!-- Saved Connections Section -->
        <div>
          <div class="mb-4 flex items-center justify-between">
            <h2 class="text-lg font-semibold">
              Saved Connections
            </h2>
            <div class="flex gap-2 items-center">
              <Button
                variant="ghost"
                size="icon"
                :class="{ 'bg-accent': viewMode === 'grid' }"
                @click="viewMode = 'grid'"
              >
                <svg
                  xmlns="http://www.w3.org/2000/svg"
                  width="24"
                  height="24"
                  viewBox="0 0 24 24"
                  fill="none"
                  stroke="currentColor"
                  stroke-width="2"
                  stroke-linecap="round"
                  stroke-linejoin="round"
                  class="h-4 w-4"
                >
                  <rect width="7" height="7" x="3" y="3" rx="1" />
                  <rect width="7" height="7" x="14" y="3" rx="1" />
                  <rect width="7" height="7" x="14" y="14" rx="1" />
                  <rect width="7" height="7" x="3" y="14" rx="1" />
                </svg>
              </Button>
              <Button
                variant="ghost"
                size="icon"
                :class="{ 'bg-accent': viewMode === 'list' }"
                @click="viewMode = 'list'"
              >
                <svg
                  xmlns="http://www.w3.org/2000/svg"
                  width="24"
                  height="24"
                  viewBox="0 0 24 24"
                  fill="none"
                  stroke="currentColor"
                  stroke-width="2"
                  stroke-linecap="round"
                  stroke-linejoin="round"
                  class="h-4 w-4"
                >
                  <line x1="8" x2="21" y1="6" y2="6" />
                  <line x1="8" x2="21" y1="12" y2="12" />
                  <line x1="8" x2="21" y1="18" y2="18" />
                  <line x1="3" x2="3.01" y1="6" y2="6" />
                  <line x1="3" x2="3.01" y1="12" y2="12" />
                  <line x1="3" x2="3.01" y1="18" y2="18" />
                </svg>
              </Button>
            </div>
          </div>

          <!-- Connections Grid/List -->
          <div
            :class="viewMode === 'grid'
              ? 'grid gap-4 sm:grid-cols-2 lg:grid-cols-3 xl:grid-cols-4'
              : 'flex flex-col gap-3'"
          >
            <!-- Add New Connection Card -->
            <Card
              class="border-dashed cursor-pointer transition-colors hover:border-primary hover:bg-accent/50"
              @click="handleAddConnection"
            >
              <div class="p-4 text-center flex flex-col min-h-40 items-center justify-center space-y-3">
                <div class="text-muted-foreground border-2 rounded-full border-dashed flex h-10 w-10 items-center justify-center">
                  <svg
                    xmlns="http://www.w3.org/2000/svg"
                    width="24"
                    height="24"
                    viewBox="0 0 24 24"
                    fill="none"
                    stroke="currentColor"
                    stroke-width="2"
                    stroke-linecap="round"
                    stroke-linejoin="round"
                    class="h-5 w-5"
                  >
                    <path d="M5 12h14" />
                    <path d="M12 5v14" />
                  </svg>
                </div>
                <div>
                  <p class="font-medium">
                    Add New Connection
                  </p>
                  <p class="text-sm text-muted-foreground">
                    PostgreSQL, MySQL, SQLite...
                  </p>
                </div>
              </div>
            </Card>

            <!-- Connection Cards -->
            <ServerCard
              v-for="connection in filteredConnections"
              :key="connection.id"
              :connection="connection"
              :connection-status="getConnectionStatus(connection.id)"
              @connect="handleConnect"
              @edit="handleEditConnection"
              @delete="handleDeleteConnection"
              @duplicate="handleDuplicateConnection"
            />
          </div>

          <!-- Empty state -->
          <div
            v-if="filteredConnections.length === 0 && searchQuery"
            class="py-12 text-center"
          >
            <svg
              xmlns="http://www.w3.org/2000/svg"
              width="24"
              height="24"
              viewBox="0 0 24 24"
              fill="none"
              stroke="currentColor"
              stroke-width="2"
              stroke-linecap="round"
              stroke-linejoin="round"
              class="text-muted-foreground mx-auto mb-4 h-12 w-12"
            >
              <circle cx="11" cy="11" r="8" />
              <path d="m21 21-4.3-4.3" />
            </svg>
            <h3 class="text-lg font-semibold">
              No connections found
            </h3>
            <p class="text-muted-foreground mt-1">
              No connections match your search "{{ searchQuery }}"
            </p>
          </div>
        </div>
      </div>

      <!-- Floating Action Button -->
      <Button
        class="rounded-full h-14 w-14 shadow-lg bottom-6 right-6 fixed"
        size="icon"
        @click="handleAddConnection"
      >
        <svg
          xmlns="http://www.w3.org/2000/svg"
          width="24"
          height="24"
          viewBox="0 0 24 24"
          fill="none"
          stroke="currentColor"
          stroke-width="2"
          stroke-linecap="round"
          stroke-linejoin="round"
          class="h-6 w-6"
        >
          <path d="M5 12h14" />
          <path d="M12 5v14" />
        </svg>
      </Button>
    </div>

    <!-- Server Form Dialog -->
    <ServerFormDialog
      v-model:open="isFormDialogOpen"
      :connection="editingConnection"
      @save="handleSaveConnection"
    />

    <!-- Delete Confirmation Dialog -->
    <AlertDialog v-model:open="deleteDialogOpen">
      <AlertDialogContent>
        <AlertDialogHeader>
          <AlertDialogTitle>Delete Connection</AlertDialogTitle>
          <AlertDialogDescription>
            Are you sure you want to delete "{{ connectionToDelete?.name }}"?
            This action cannot be undone.
          </AlertDialogDescription>
        </AlertDialogHeader>
        <AlertDialogFooter>
          <AlertDialogCancel>Cancel</AlertDialogCancel>
          <AlertDialogAction
            class="text-destructive-foreground bg-destructive hover:bg-destructive/90"
            @click="confirmDelete"
          >
            Delete
          </AlertDialogAction>
        </AlertDialogFooter>
      </AlertDialogContent>
    </AlertDialog>
  </AppLayout>
</template>
