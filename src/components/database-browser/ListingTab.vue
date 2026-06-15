<script setup lang="ts">
import { computed, ref } from 'vue'
import { useI18n } from 'vue-i18n'
import type { ObjectInfo } from '@/datasources/browseApi'
import { browseApi } from '@/datasources/browseApi'
import { Button } from '@/components/ui/button'
import { Input } from '@/components/ui/input'
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
import {
  Dialog,
  DialogContent,
  DialogTitle,
} from '@/components/ui/dialog'
import { Spinner } from '@/components/ui/spinner'
import { toast } from '@/composables/useNotifications'
import DdlModal from './DdlModal.vue'

type ListingType = 'VIEW' | 'PROCEDURE' | 'FUNCTION'

const props = defineProps<{
  connectionId: string
  database: string
  schema: string | null
  type: ListingType
  objects: ObjectInfo[]
  loading: boolean
  error: string | null
}>()

const emit = defineEmits<{
  'openObject': [obj: ObjectInfo]
  'refresh': []
}>()

const { t } = useI18n()

const searchQuery = ref('')

const typeConfig: Record<ListingType, { icon: string, label: string }> = {
  VIEW: { icon: '👁', label: t('components.listingTab.views') },
  PROCEDURE: { icon: '⚙', label: t('components.listingTab.procedures') },
  FUNCTION: { icon: '⚙', label: t('components.listingTab.functions') },
}

const filteredObjects = computed(() => {
  const q = searchQuery.value.toLowerCase().trim()
  if (!q) {
    return props.objects
  }
  return props.objects.filter(obj => obj.name.toLowerCase().includes(q))
})

// --- DDL Modal ---
const ddlModalOpen = ref(false)
const ddlTitle = ref('')
const ddlContent = ref('')
const ddlLoading = ref(false)

async function openDdlModal(obj: ObjectInfo) {
  ddlTitle.value = `${obj.name} — ${typeConfig[props.type]?.label ?? props.type}`
  ddlLoading.value = true
  ddlModalOpen.value = true
  try {
    const ddl = await browseApi.getObjectDdl(
      props.connectionId,
      props.database,
      props.schema,
      obj.name,
      props.type,
    )
    ddlContent.value = ddl
  }
  catch (err) {
    ddlContent.value = `-- Failed to load DDL:\n-- ${String(err)}`
  }
  finally {
    ddlLoading.value = false
  }
}

// --- Drop dialog ---
const dropDialogOpen = ref(false)
const dropTarget = ref<ObjectInfo | null>(null)
const isDropping = ref(false)

function confirmDrop(obj: ObjectInfo) {
  dropTarget.value = obj
  dropDialogOpen.value = true
}

async function executeDrop() {
  if (!dropTarget.value) {
    return
  }
  isDropping.value = true
  try {
    await browseApi.dropObject(
      props.connectionId,
      props.database,
      props.schema,
      dropTarget.value.name,
      props.type,
    )
    toast.success(`${dropTarget.value.name} dropped`)
    dropDialogOpen.value = false
    dropTarget.value = null
    emit('refresh')
  }
  catch (err) {
    toast.error(String(err))
  }
  finally {
    isDropping.value = false
  }
}

// --- Rename dialog ---
const renameDialogOpen = ref(false)
const renameTarget = ref<ObjectInfo | null>(null)
const newName = ref('')
const isRenaming = ref(false)

function confirmRename(obj: ObjectInfo) {
  renameTarget.value = obj
  newName.value = ''
  renameDialogOpen.value = true
}

async function executeRename() {
  if (!renameTarget.value || !newName.value.trim()) {
    return
  }
  isRenaming.value = true
  try {
    await browseApi.renameObject(
      props.connectionId,
      props.database,
      props.schema,
      renameTarget.value.name,
      props.type,
      newName.value.trim(),
    )
    toast.success(`${renameTarget.value.name} → ${newName.value.trim()}`)
    renameDialogOpen.value = false
    renameTarget.value = null
    emit('refresh')
  }
  catch (err) {
    toast.error(String(err))
  }
  finally {
    isRenaming.value = false
  }
}

// --- Context menu ---
const contextMenuTarget = ref<ObjectInfo | null>(null)
const contextMenuPos = ref({ x: 0, y: 0 })
const showContextMenu = ref(false)

function handleContextMenu(event: MouseEvent, obj: ObjectInfo) {
  event.preventDefault()
  contextMenuTarget.value = obj
  contextMenuPos.value = { x: event.clientX, y: event.clientY }
  showContextMenu.value = true
}

function closeContextMenu() {
  showContextMenu.value = false
  contextMenuTarget.value = null
}

async function copyName() {
  if (!contextMenuTarget.value) {
    return
  }
  try {
    await navigator.clipboard.writeText(contextMenuTarget.value.name)
    toast.success(t('common.copied'))
  }
  catch {
    // ignore
  }
  closeContextMenu()
}

async function copyDdlContext() {
  if (!contextMenuTarget.value) {
    return
  }
  try {
    const ddl = await browseApi.getObjectDdl(
      props.connectionId,
      props.database,
      props.schema,
      contextMenuTarget.value.name,
      props.type,
    )
    await navigator.clipboard.writeText(ddl)
    toast.success(t('common.copied'))
  }
  catch {
    // ignore
  }
  closeContextMenu()
}
</script>

<template>
  <div class="listing-tab flex flex-col h-full" @click="closeContextMenu">
    <!-- Search bar -->
    <div class="px-3 py-2 border-b flex gap-2 items-center">
      <svg
        xmlns="http://www.w3.org/2000/svg" width="14" height="14" viewBox="0 0 24 24"
        fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round"
        stroke-linejoin="round" class="text-muted-foreground flex-shrink-0"
      >
        <circle cx="11" cy="11" r="8" />
        <path d="m21 21-4.3-4.3" />
      </svg>
      <Input
        v-model="searchQuery"
        :placeholder="t('components.listingTab.searchPlaceholder')"
        class="text-xs h-7 flex-1"
      />
      <span class="text-xs text-muted-foreground">
        {{ filteredObjects.length }} / {{ objects.length }}
      </span>
    </div>

    <!-- Content -->
    <div class="flex-1 overflow-auto">
      <!-- Loading -->
      <div v-if="loading" class="flex items-center justify-center gap-2 py-8 text-sm text-muted-foreground">
        <Spinner size="sm" />
        {{ t('common.loading') }}
      </div>

      <!-- Error -->
      <div v-else-if="error" class="py-8 text-sm text-destructive text-center">
        {{ error }}
      </div>

      <!-- Empty state -->
      <div v-else-if="filteredObjects.length === 0" class="py-8 text-sm text-muted-foreground text-center">
        {{ searchQuery ? t('components.listingTab.noResults') : t('components.listingTab.empty') }}
      </div>

      <!-- Table -->
      <table v-else class="w-full text-xs">
        <thead>
          <tr class="text-left text-muted-foreground border-b">
            <th class="px-3 py-1.5 font-medium">
              {{ t('components.listingTab.name') }}
            </th>
            <th class="px-3 py-1.5 font-medium">
              {{ t('components.listingTab.detail') }}
            </th>
            <th class="px-3 py-1.5 font-medium">
              {{ t('components.listingTab.ddl') }}
            </th>
            <th class="px-3 py-1.5 font-medium">
              {{ t('components.listingTab.actions') }}
            </th>
          </tr>
        </thead>
        <tbody>
          <tr
            v-for="obj in filteredObjects"
            :key="obj.name"
            class="border-b border-border/50 hover:bg-accent/50 cursor-pointer"
            @click="emit('openObject', obj)"
            @contextmenu="handleContextMenu($event, obj)"
          >
            <td class="px-3 py-2 font-medium">
              {{ obj.name }}
            </td>
            <td class="px-3 py-2 text-muted-foreground max-w-[200px] truncate">
              {{ obj.detail ?? '-' }}
            </td>
            <td class="px-3 py-2">
              <Button variant="ghost" size="sm" class="text-xs h-6" @click.stop="openDdlModal(obj)">
                {{ t('components.listingTab.viewDdl') }}
              </Button>
            </td>
            <td class="px-3 py-2">
              <div class="flex gap-1">
                <Button variant="ghost" size="sm" class="text-xs h-6 text-destructive" @click.stop="confirmDrop(obj)">
                  {{ t('components.listingTab.drop') }}
                </Button>
                <Button variant="ghost" size="sm" class="text-xs h-6" @click.stop="confirmRename(obj)">
                  {{ t('components.listingTab.rename') }}
                </Button>
              </div>
            </td>
          </tr>
        </tbody>
      </table>
    </div>

    <!-- Context Menu -->
    <div
      v-if="showContextMenu"
      class="text-popover-foreground border rounded-md bg-popover w-44 shadow-md fixed z-50"
      :style="{ left: `${contextMenuPos.x}px`, top: `${contextMenuPos.y}px` }"
    >
      <div class="p-1">
        <div
          class="text-sm px-2 py-1.5 rounded-sm flex cursor-pointer items-center hover:text-accent-foreground hover:bg-accent"
          @click="copyName"
        >
          {{ t('components.listingTab.copyName') }}
        </div>
        <div
          class="text-sm px-2 py-1.5 rounded-sm flex cursor-pointer items-center hover:text-accent-foreground hover:bg-accent"
          @click="copyDdlContext"
        >
          {{ t('components.listingTab.copyDdl') }}
        </div>
        <div class="my-1 bg-border h-px" />
        <div
          class="text-sm px-2 py-1.5 rounded-sm flex cursor-pointer items-center hover:text-accent-foreground hover:bg-accent"
          @click="confirmDrop(contextMenuTarget!)"
        >
          {{ t('components.listingTab.drop') }}
        </div>
        <div
          class="text-sm px-2 py-1.5 rounded-sm flex cursor-pointer items-center hover:text-accent-foreground hover:bg-accent"
          @click="confirmRename(contextMenuTarget!)"
        >
          {{ t('components.listingTab.rename') }}
        </div>
      </div>
    </div>

    <!-- DDL Modal -->
    <DdlModal
      :open="ddlModalOpen"
      :title="ddlTitle"
      :ddl="ddlContent"
      @update:open="ddlModalOpen = $event"
    />

    <!-- Drop Confirmation Dialog -->
    <AlertDialog :open="dropDialogOpen" @update:open="dropDialogOpen = $event">
      <AlertDialogContent>
        <AlertDialogHeader>
          <AlertDialogTitle>{{ t('components.listingTab.dropDialogTitle') }}</AlertDialogTitle>
          <AlertDialogDescription>
            {{ t('components.listingTab.dropDialogDescription', { name: dropTarget?.name }) }}
          </AlertDialogDescription>
        </AlertDialogHeader>
        <AlertDialogFooter>
          <AlertDialogCancel :disabled="isDropping">
            {{ t('common.buttons.cancel') }}
          </AlertDialogCancel>
          <AlertDialogAction :disabled="isDropping" @click.prevent="executeDrop">
            <Spinner v-if="isDropping" size="sm" class="mr-1" />
            {{ t('components.listingTab.confirmDrop') }}
          </AlertDialogAction>
        </AlertDialogFooter>
      </AlertDialogContent>
    </AlertDialog>

    <!-- Rename Dialog -->
    <Dialog :open="renameDialogOpen" @update:open="renameDialogOpen = $event">
      <DialogContent class="max-w-sm">
        <div class="px-4 py-3 border-b">
          <DialogTitle class="text-sm font-semibold">
            {{ t('components.listingTab.renameDialogTitle') }}
          </DialogTitle>
        </div>
        <div class="p-4 space-y-3">
          <div class="text-xs text-muted-foreground">
            {{ t('components.listingTab.renameDialogDescription', { name: renameTarget?.name }) }}
          </div>
          <Input
            v-model="newName"
            :placeholder="t('components.listingTab.newNamePlaceholder')"
            class="text-xs h-8"
            @keydown.enter="executeRename"
          />
        </div>
        <div class="px-4 py-3 border-t flex gap-2 justify-end">
          <Button variant="outline" size="sm" :disabled="isRenaming" @click="renameDialogOpen = false">
            {{ t('common.buttons.cancel') }}
          </Button>
          <Button size="sm" :disabled="isRenaming || !newName.trim()" @click="executeRename">
            <Spinner v-if="isRenaming" size="sm" class="mr-1" />
            {{ t('components.listingTab.confirmRename') }}
          </Button>
        </div>
      </DialogContent>
    </Dialog>
  </div>
</template>
