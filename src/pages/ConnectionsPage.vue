<script setup lang="ts">
import type { ServerConnection } from '@/store'
import type { SshProfile } from '@/store/sshProfileStore'
import { invoke } from '@tauri-apps/api/core'
import { Pencil, Trash2 } from 'lucide-vue-next'
import { computed, onMounted, ref } from 'vue'
import { useI18n } from 'vue-i18n'
import { useRouter } from 'vue-router'
import { isSshTunnelStrippedError, shouldStripSshTunnel } from '@/common'
import { ServerCard, ServerFormDialog } from '@/components/connections'
import ConnectingModal from '@/components/connections/ConnectingModal.vue'
import SshProfileDialog from '@/components/connections/SshProfileDialog.vue'
import AppLayout from '@/components/layout/AppLayout.vue'
import { Badge } from '@/components/ui/badge'
import { Button } from '@/components/ui/button'
import { Card } from '@/components/ui/card'
import { DestructiveConfirmDialog } from '@/components/ui/destructive-confirm-dialog'
import { Input } from '@/components/ui/input'
import { Tooltip, TooltipContent, TooltipProvider, TooltipTrigger } from '@/components/ui/tooltip'
import { openUpgradeDialog } from '@/components/upgrade'
import { toast } from '@/composables/useNotifications'
import { ConnectionStatus, useConnectionStore, useEntitlementStore } from '@/store'
import { useSshProfileStore } from '@/store/sshProfileStore'

const MIN_LOADING_TIME = 1500

const { t } = useI18n()
const router = useRouter()
const connectionStore = useConnectionStore()
const entitlementStore = useEntitlementStore()
const sshProfileStore = useSshProfileStore()
const sshProfileDialogRef = ref<InstanceType<typeof SshProfileDialog> | null>(null)

function openProfileCreate() {
  if (!guardSshProfileManagement())
    return
  sshProfileDialogRef.value?.show()
}

const searchQuery = ref('')
const viewMode = ref<'grid' | 'list'>('grid')
const isFormDialogOpen = ref(false)
const editingConnection = ref<ServerConnection | null>(null)
const deleteDialogOpen = ref(false)
const connectionToDelete = ref<ServerConnection | null>(null)
const connectError = ref<string | null>(null)
const connectionCancelled = ref(false)
const profileDeleteOpen = ref(false)
const profileToDelete = ref<SshProfile | null>(null)

const profileAuthLabel = (profile: SshProfile) => {
  switch (profile.authMethod) {
    case 'key':
      return t('components.serverForm.ssh.privateKey')
    case 'agent':
      return t('components.serverForm.ssh.sshAgent')
    default:
      return t('components.serverForm.ssh.password')
  }
}

function openProfileDelete(profile: SshProfile) {
  profileToDelete.value = profile
  profileDeleteOpen.value = true
}

async function confirmProfileDelete() {
  if (profileToDelete.value) {
    await sshProfileStore.remove(profileToDelete.value.id)
    profileToDelete.value = null
    profileDeleteOpen.value = false
  }
}
const connectingModal = ref<InstanceType<typeof ConnectingModal> | null>(null)

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
  sshProfiles: sshProfileStore.profiles.length,
}))

onMounted(async () => {
  sshProfileStore.fetch()
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

// DockIt parity: a tunnel opted into the system proxy connects direct when the
// OS no longer has one — probe the FIRST hop (the backend routes the proxy
// against the bastion, not the database host) and warn instead of silently
// bypassing.
const systemProxyWarning = async (connection: ServerConnection): Promise<string | undefined> => {
  const sshTunnel = connection.sshTunnel
  if (!sshTunnel?.enabled || !sshTunnel.useSystemProxy)
    return undefined
  const firstHopProfile = sshTunnel.profileIds?.length
    ? sshProfileStore.byId(sshTunnel.profileIds[0])
    : undefined
  const firstHop = firstHopProfile
    ? { host: firstHopProfile.host, port: firstHopProfile.port }
    : sshTunnel.host
      ? { host: sshTunnel.host, port: sshTunnel.port ?? 22 }
      : null
  try {
    const detected = firstHop
      ? await invoke<string | null>('detect_system_proxy', { host: firstHop.host, port: firstHop.port })
      : await invoke<string | null>('detect_system_proxy')
    return detected ? undefined : t('components.serverForm.ssh.systemProxyNotDetected')
  }
  catch {
    return undefined
  }
}

const sshTunnelSkippedWarning = (connection: ServerConnection): string | undefined =>
  shouldStripSshTunnel(connection.sshTunnel?.enabled, entitlementStore.isLocalUltimate)
    ? t('components.connectingModal.sshTunnelSkipped')
    : undefined

async function establishConnection(connection: ServerConnection, navigateOnSuccess = true) {
  if (!connection.id) {
    return
  }

  connectionCancelled.value = false

  // Show the modal first (instant click feedback), then resolve the async
  // diagnostic and patch the warning in; a dropped tunnel outranks the proxy probe.
  connectingModal.value?.show(
    connection.name,
    () => {
      connectionCancelled.value = true
    },
    () => establishConnection(connection, navigateOnSuccess),
  )
  const tunnelWarning = sshTunnelSkippedWarning(connection)
  const proxyWarning = tunnelWarning ? undefined : await systemProxyWarning(connection)
  connectingModal.value?.setWarning(tunnelWarning ?? proxyWarning ?? null)

  const startTime = Date.now()

  try {
    await connectionStore.connect(connection.id)

    if (connectionCancelled.value) {
      return
    }

    const elapsed = Date.now() - startTime
    const remainingTime = Math.max(0, MIN_LOADING_TIME - elapsed)
    if (remainingTime > 0) {
      await new Promise(resolve => setTimeout(resolve, remainingTime))
    }

    if (connectionCancelled.value) {
      return
    }

    connectingModal.value?.hide()

    if (navigateOnSuccess) {
      router.push({
        name: 'queries',
        query: { connectionId: connection.id },
      })
    }
  }
  catch (err) {
    if (connectionCancelled.value) {
      return
    }

    const elapsed = Date.now() - startTime
    const remainingTime = Math.max(0, MIN_LOADING_TIME - elapsed)
    if (remainingTime > 0) {
      await new Promise(resolve => setTimeout(resolve, remainingTime))
    }

    if (connectionCancelled.value) {
      return
    }

    const errorMessage = err instanceof Error ? err.message : String(err)
    connectingModal.value?.showError(
      errorMessage,
      isSshTunnelStrippedError(err) ? t('components.connectingModal.sshTunnelStripped') : '',
    )
  }
}

// SSH profile management is Ultimate-only — the modal carries pricing + trial
function guardSshProfileManagement(): boolean {
  if (entitlementStore.isLocalUltimate)
    return true
  openUpgradeDialog('ssh_tunnel')
  return false
}

function openProfileEdit(profile: SshProfile) {
  if (!guardSshProfileManagement())
    return
  sshProfileDialogRef.value?.show(profile)
}

async function handleConnect(connection: ServerConnection) {
  connectError.value = null
  if (!connection.id) {
    return
  }

  const status = connectionStore.getConnectionStatus(connection.id)

  if (status === ConnectionStatus.CONNECTED) {
    // Already connected — primary action opens the connection in the editor.
    router.push({
      name: 'queries',
      query: { connectionId: connection.id },
    })
  }
  else {
    await establishConnection(connection, true)
  }
}

async function handleDoubleClick(connection: ServerConnection) {
  connectError.value = null
  if (!connection.id) {
    return
  }

  const status = connectionStore.getConnectionStatus(connection.id)

  if (status === ConnectionStatus.CONNECTED) {
    router.push({
      name: 'queries',
      query: { connectionId: connection.id },
    })
  }
  else {
    await establishConnection(connection, true)
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
    name: `${connection.name}${t('connectionsPage.copySuffix')}`,
  }
  isFormDialogOpen.value = true
}

async function handleDisconnect(connection: ServerConnection) {
  connectError.value = null
  if (!connection.id)
    return
  try {
    await connectionStore.disconnect(connection.id)
  }
  catch (error) {
    connectError.value = error instanceof Error ? error.message : String(error)
  }
}

async function handleSaveConnection(connection: ServerConnection) {
  const result = await connectionStore.saveConnection(connection)
  if (!result.success) {
    toast.error(t('pages.connections.saveFailed'), { description: result.message })
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
    <div class="flex flex-col h-full relative">
      <div class="p-6 pb-0 shrink-0 space-y-6">
        <div class="flex items-center justify-between">
          <div class="flex gap-3 items-center">
            <h1 class="text-xl font-semibold">
              {{ t('pages.connections.title') }}
            </h1>
            <span class="text-muted-foreground">|</span>
            <span class="text-sm text-muted-foreground">{{ t('pages.connections.subtitle') }}</span>
          </div>
          <div class="w-64 relative">
            <svg xmlns="http://www.w3.org/2000/svg" width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" class="text-muted-foreground h-4 w-4 left-3 top-1/2 absolute -translate-y-1/2">
              <circle cx="11" cy="11" r="8" />
              <path d="m21 21-4.3-4.3" />
            </svg>
            <Input
              v-model="searchQuery"
              :placeholder="t('pages.connections.search')"
              class="pl-9 bg-muted/50"
            />
          </div>
        </div>

        <div class="gap-4 grid md:grid-cols-3">
          <Card class="p-4">
            <div class="flex items-center justify-between">
              <div>
                <p class="text-sm text-muted-foreground">
                  {{ t('pages.connections.stats.totalConnections') }}
                </p>
                <p class="text-2xl font-bold">
                  {{ stats.total }}
                </p>
              </div>
              <div class="rounded-lg bg-primary/10 flex h-10 w-10 items-center justify-center">
                <svg xmlns="http://www.w3.org/2000/svg" width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" class="text-primary h-5 w-5">
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
                  {{ t('pages.connections.stats.sshProfiles') }}
                </p>
                <p class="text-2xl font-bold">
                  {{ stats.sshProfiles }}
                </p>
              </div>
              <div class="rounded-lg bg-violet-100 flex h-10 w-10 items-center justify-center dark:bg-violet-900/30">
                <span class="i-carbon-key text-violet-600 h-5 w-5 dark:text-violet-400" />
              </div>
            </div>
          </Card>
          <Card class="p-4">
            <div class="flex items-center justify-between">
              <div>
                <p class="text-sm text-muted-foreground">
                  {{ t('pages.connections.stats.activeSessions') }}
                </p>
                <p class="text-2xl font-bold">
                  {{ stats.active }}
                </p>
              </div>
              <div class="rounded-lg bg-green-100 flex h-10 w-10 items-center justify-center dark:bg-green-900/30">
                <svg xmlns="http://www.w3.org/2000/svg" width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" class="text-green-600 h-5 w-5 dark:text-green-400">
                  <polygon points="13 2 3 14 12 14 11 22 21 10 12 10 13 2" />
                </svg>
              </div>
            </div>
          </Card>
        </div>

        <div v-if="connectError" class="p-4 rounded-md bg-red-50 dark:bg-red-900/20">
          <div class="flex gap-3 items-start">
            <svg xmlns="http://www.w3.org/2000/svg" width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" class="text-red-500 h-5 w-5">
              <circle cx="12" cy="12" r="10" />
              <line x1="12" y1="8" x2="12" y2="12" />
              <line x1="12" y1="16" x2="12.01" y2="16" />
            </svg>
            <div class="flex-1">
              <h3 class="text-sm text-red-800 font-medium dark:text-red-200">
                {{ t('pages.connections.error.title') }}
              </h3>
              <p class="text-sm text-red-700 mt-1 dark:text-red-300">
                {{ connectError }}
              </p>
            </div>
            <button type="button" class="text-red-500 hover:text-red-700" @click="connectError = null">
              <svg xmlns="http://www.w3.org/2000/svg" width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" class="h-4 w-4">
                <path d="M18 6 6 18" />
                <path d="m6 6 12 12" />
              </svg>
            </button>
          </div>
        </div>
      </div>

      <div class="flex-1 min-h-0 overflow-y-auto">
        <div class="mb-2 p-6 pb-3 flex items-center justify-between">
          <h2 class="text-lg font-semibold">
            {{ t('connectionsPage.sshProfilesSection') }}
          </h2>
        </div>
        <div
          class="mx-6 mb-8 gap-4 grid lg:grid-cols-3 sm:grid-cols-2 xl:grid-cols-4"
        >
          <div
            v-for="profile in sshProfileStore.profiles"
            :key="profile.id"
            class="p-4 border rounded-lg cursor-pointer transition-colors hover:bg-accent/50"
            role="button"
            tabindex="0"
            @click="openProfileEdit(profile)"
            @keydown.enter.self.prevent="openProfileEdit(profile)"
            @keydown.space.self.prevent="openProfileEdit(profile)"
          >
            <div class="flex gap-3 items-start">
              <div class="p-1.5 rounded-lg bg-violet-100 flex shrink-0 h-10 w-10 items-center justify-center dark:bg-violet-900/30">
                <svg xmlns="http://www.w3.org/2000/svg" width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" class="text-violet-600 h-5 w-5 dark:text-violet-400">
                  <path d="m21 2-2 2m-7.61 7.61a5.5 5.5 0 1 1-7.778 7.778 5.5 5.5 0 0 1 7.777-7.777zm0 0L15.5 7.5m0 0 3 3L22 7l-3-3m-3.5 3.5L19 4" />
                </svg>
              </div>
              <div class="flex-1 min-w-0">
                <span class="text-base font-medium block truncate">{{ profile.name }}</span>
                <span class="text-sm text-muted-foreground block truncate">
                  {{ profile.username }}@{{ profile.host }}:{{ profile.port }}
                </span>
              </div>
            </div>
            <div class="mt-2 flex gap-1.5 items-center">
              <Badge variant="outline" class="text-xs shrink-0">
                {{ t('connectionsPage.sshProfile') }}
              </Badge>
              <Badge variant="outline" class="text-xs text-muted-foreground shrink-0">
                {{ profileAuthLabel(profile) }}
              </Badge>
              <div class="ml-auto flex shrink-0 gap-1 items-center">
                <TooltipProvider>
                  <Tooltip>
                    <TooltipTrigger as-child>
                      <Button
                        variant="ghost"
                        size="icon"
                        type="button"
                        class="h-7 w-7"
                        @click.stop="openProfileEdit(profile)"
                      >
                        <Pencil class="h-3.5 w-3.5" />
                      </Button>
                    </TooltipTrigger>
                    <TooltipContent>
                      <p>{{ t('components.serverCard.actions.edit') }}</p>
                    </TooltipContent>
                  </Tooltip>
                </TooltipProvider>
                <TooltipProvider>
                  <Tooltip>
                    <TooltipTrigger as-child>
                      <Button
                        variant="ghost"
                        size="icon"
                        type="button"
                        class="text-destructive h-7 w-7 hover:text-destructive"
                        @click.stop="openProfileDelete(profile)"
                      >
                        <Trash2 class="h-3.5 w-3.5" />
                      </Button>
                    </TooltipTrigger>
                    <TooltipContent>
                      <p>{{ t('components.serverCard.actions.delete') }}</p>
                    </TooltipContent>
                  </Tooltip>
                </TooltipProvider>
              </div>
            </div>
          </div>

          <div
            class="p-4 border rounded-lg border-dashed flex min-h-[86px] cursor-pointer transition-colors items-center justify-center hover:bg-accent/50"
            role="button"
            tabindex="0"
            @click="openProfileCreate"
            @keydown.enter.self.prevent="openProfileCreate"
            @keydown.space.self.prevent="openProfileCreate"
          >
            <div class="text-muted-foreground flex gap-2 items-center">
              <span class="i-carbon-add h-4 w-4" />
              <span class="text-sm">{{ t('connectionsPage.addSshProfile') }}</span>
            </div>
          </div>
        </div>

        <div class="mb-4 p-6 pt-0 flex items-center justify-between">
          <h2 class="text-lg font-semibold">
            {{ t('pages.connections.saved') }}
          </h2>
          <div class="flex gap-2 items-center">
            <Button
              variant="ghost"
              size="icon"
              :class="{ 'bg-accent': viewMode === 'grid' }"
              @click="viewMode = 'grid'"
            >
              <svg xmlns="http://www.w3.org/2000/svg" width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" class="h-4 w-4">
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
              <svg xmlns="http://www.w3.org/2000/svg" width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" class="h-4 w-4">
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

        <div
          class="mx-6 pb-24" :class="viewMode === 'grid'
            ? 'grid gap-4 sm:grid-cols-2 lg:grid-cols-3 xl:grid-cols-4'
            : 'flex flex-col gap-3'"
        >
          <ServerCard
            v-for="connection in filteredConnections"
            :key="connection.id"
            :connection="connection"
            :connection-status="getConnectionStatus(connection.id)"
            @connect="handleConnect"
            @dblclick="handleDoubleClick"
            @edit="handleEditConnection"
            @delete="handleDeleteConnection"
            @duplicate="handleDuplicateConnection"
            @disconnect="handleDisconnect"
          />

          <Card
            class="border-dashed cursor-pointer transition-colors hover:border-primary hover:bg-accent/50"
            @click="handleAddConnection"
          >
            <div class="p-4 text-center flex flex-col min-h-40 items-center justify-center space-y-3">
              <div class="text-muted-foreground border-2 rounded-full border-dashed flex h-10 w-10 items-center justify-center">
                <svg xmlns="http://www.w3.org/2000/svg" width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" class="h-5 w-5">
                  <path d="M5 12h14" />
                  <path d="M12 5v14" />
                </svg>
              </div>
              <div>
                <p class="font-medium">
                  {{ t('pages.connections.addNew') }}
                </p>
                <p class="text-sm text-muted-foreground">
                  {{ t('pages.connections.addNewHint') }}
                </p>
              </div>
            </div>
          </Card>
        </div>

        <div v-if="filteredConnections.length === 0 && searchQuery" class="py-12 text-center">
          <svg xmlns="http://www.w3.org/2000/svg" width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" class="text-muted-foreground mx-auto mb-4 h-12 w-12">
            <circle cx="11" cy="11" r="8" />
            <path d="m21 21-4.3-4.3" />
          </svg>
          <h3 class="text-lg font-semibold">
            {{ t('pages.connections.empty.title') }}
          </h3>
          <p class="text-muted-foreground mt-1">
            {{ t('pages.connections.empty.message', { query: searchQuery }) }}
          </p>
        </div>
      </div>

      <!-- Floating Add Connection — INSIDE the relative page container so the
           absolute anchor tracks the content edge even with a side panel open -->
      <button
        class="text-white rounded-full bg-blue-600 flex h-14 w-14 cursor-pointer shadow-lg transition-colors items-center bottom-6 right-6 justify-center absolute dark:bg-blue-600 hover:bg-blue-700 dark:hover:bg-blue-700"
        @click="handleAddConnection"
      >
        <svg xmlns="http://www.w3.org/2000/svg" width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" class="h-6 w-6">
          <path d="M5 12h14" />
          <path d="M12 5v14" />
        </svg>
      </button>
    </div>

    <!-- Server Form Dialog -->
    <ServerFormDialog
      v-model:open="isFormDialogOpen"
      :connection="editingConnection"
      @save="handleSaveConnection"
    />
    <DestructiveConfirmDialog
      v-model:open="deleteDialogOpen"
      :title="t('components.destructiveDialog.deleteConnection.title')"
      :message="t('components.destructiveDialog.deleteConnection.message', { name: connectionToDelete?.name })"
      :detail="t('components.destructiveDialog.deleteConnection.detail')"
      :confirm-label="t('components.destructiveDialog.deleteConnection.confirm')"
      @confirm="confirmDelete"
    />
    <DestructiveConfirmDialog
      v-model:open="profileDeleteOpen"
      :title="t('components.destructiveDialog.deleteProfile.title')"
      :message="t('components.destructiveDialog.deleteProfile.message', { name: profileToDelete?.name })"
      :detail="t('components.destructiveDialog.deleteProfile.detail')"
      :confirm-label="t('components.destructiveDialog.deleteProfile.confirm')"
      @confirm="confirmProfileDelete"
    />

    <ConnectingModal ref="connectingModal" />
    <SshProfileDialog ref="sshProfileDialogRef" @save="sshProfileStore.fetch()" />
  </AppLayout>
</template>
