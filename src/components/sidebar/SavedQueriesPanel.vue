<script setup lang="ts">
import type { SavedQueryInfo, SavedQueryMetadata } from '@/datasources/fileApi'
import { revealItemInDir } from '@tauri-apps/plugin-opener'
import { computed, onMounted, ref, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import { Button } from '@/components/ui/button'
import { toast } from '@/composables/useNotifications'
import { deleteQueryFile, listSavedQueryFiles, loadQueryFile, readSavedQueriesMetadata, saveQueryFile, saveQueryMetadata, writeSavedQueriesMetadata } from '@/datasources'
import ChangeConnectionDialog from './ChangeConnectionDialog.vue'
import DeleteQueryDialog from './DeleteQueryDialog.vue'
import RenameQueryDialog from './RenameQueryDialog.vue'
import SavedQueryItem from './SavedQueryItem.vue'

type Props = {
  collapsed: boolean
}

const props = defineProps<Props>()

const emit = defineEmits<{
  (e: 'update:collapsed', value: boolean): void
  (e: 'open', filePath: string): void
  (e: 'newQuery'): void
}>()

const { t } = useI18n()

const files = ref<SavedQueryInfo[]>([])
const metadataMap = ref<Record<string, SavedQueryMetadata>>({})
const loading = ref(false)

// Dialog state
const deleteTarget = ref<{ query: SavedQueryInfo, metadata: SavedQueryMetadata | null } | null>(null)
const showDeleteDialog = ref(false)
const renameTarget = ref<SavedQueryInfo | null>(null)
const showRenameDialog = ref(false)
const changeConnTarget = ref<SavedQueryInfo | null>(null)
const showChangeConnDialog = ref(false)

const isCollapsed = computed({
  get: () => props.collapsed,
  set: (val: boolean) => emit('update:collapsed', val),
})

const sortedFiles = computed(() =>
  files.value
    .filter(f => metadataMap.value[f.file_path])
    .sort((a, b) => (metadataMap.value[b.file_path]?.modifiedAt ?? 0) - (metadataMap.value[a.file_path]?.modifiedAt ?? 0)),
)

async function fetchFiles() {
  loading.value = true
  try {
    const [fileList, metadata] = await Promise.all([
      listSavedQueryFiles(),
      readSavedQueriesMetadata(),
    ])
    files.value = fileList
    metadataMap.value = metadata.queries
  }
  catch (error) {
    console.error('Failed to fetch saved queries:', error)
    files.value = []
    metadataMap.value = {}
  }
  finally {
    loading.value = false
  }
}

function getMetadata(filePath: string): SavedQueryMetadata | null {
  return metadataMap.value[filePath] ?? null
}

async function writeMetadata() {
  try {
    await writeSavedQueriesMetadata({ queries: metadataMap.value })
  }
  catch (error) {
    console.error('Failed to write metadata:', error)
  }
}

function toggleExpand() {
  isCollapsed.value = !isCollapsed.value
}

watch(() => props.collapsed, async (collapsed) => {
  if (!collapsed && files.value.length === 0) {
    await fetchFiles()
  }
})

// Fetch files on mount so the count badge shows immediately
onMounted(() => {
  if (files.value.length === 0)
    fetchFiles()
})

function handleItemAction(query: SavedQueryInfo, kind: string) {
  switch (kind) {
    case 'open':
      emit('open', query.file_path)
      break
    case 'rename':
      renameTarget.value = query
      showRenameDialog.value = true
      break
    case 'changeConnection':
      changeConnTarget.value = query
      showChangeConnDialog.value = true
      break
    case 'reveal':
      handleReveal(query)
      break
    case 'delete':
      deleteTarget.value = { query, metadata: getMetadata(query.file_path) }
      showDeleteDialog.value = true
      break
  }
}

async function handleReveal(query: SavedQueryInfo) {
  try {
    await revealItemInDir(query.file_path)
  }
  catch (error) {
    console.error('Failed to reveal file:', error)
  }
}

async function handleDeleteConfirm() {
  if (!deleteTarget.value)
    return
  const { query } = deleteTarget.value
  try {
    await deleteQueryFile(query.file_path)
    files.value = files.value.filter(f => f.file_path !== query.file_path)
    delete metadataMap.value[query.file_path]
    await writeMetadata()
    toast.success(t('sidebar.savedQueries.actions.delete'))
  }
  catch (error) {
    toast.error(`Failed to delete: ${error instanceof Error ? error.message : String(error)}`)
  }
  finally {
    showDeleteDialog.value = false
    deleteTarget.value = null
  }
}

async function handleRenameConfirm(newName: string) {
  if (!renameTarget.value)
    return
  const oldPath = renameTarget.value.file_path
  const dir = oldPath.substring(0, Math.max(oldPath.lastIndexOf('/'), oldPath.lastIndexOf('\\')))
  const newPath = `${dir}/${newName}`
  try {
    const loadResult = await loadQueryFile(oldPath)
    if (loadResult.success && loadResult.content) {
      await saveQueryFile(loadResult.content, newPath)
      await deleteQueryFile(oldPath)
      // Update metadata key
      const meta = metadataMap.value[oldPath]
      if (meta) {
        metadataMap.value[newPath] = { ...meta }
        delete metadataMap.value[oldPath]
        await writeMetadata()
      }
      await fetchFiles()
      toast.success(t('sidebar.savedQueries.renameDialog.confirm'))
    }
  }
  catch (error) {
    toast.error(`Failed to rename: ${error instanceof Error ? error.message : String(error)}`)
  }
  finally {
    showRenameDialog.value = false
    renameTarget.value = null
  }
}

async function handleChangeConnectionConfirm(connectionId: string | null, connectionName: string | null) {
  if (!changeConnTarget.value)
    return
  const filePath = changeConnTarget.value.file_path
  const existing = metadataMap.value[filePath] ?? {
    connectionId: null,
    connectionName: null,
    createdAt: changeConnTarget.value.modified_at,
    modifiedAt: changeConnTarget.value.modified_at,
  }
  const now = Math.floor(Date.now() / 1000)
  const updatedMetadata = {
    ...existing,
    connectionId,
    connectionName,
    modifiedAt: now,
  }
  metadataMap.value[filePath] = updatedMetadata
  await saveQueryMetadata(filePath, updatedMetadata)
  showChangeConnDialog.value = false
  changeConnTarget.value = null
}

const deleteDialogCreatedAt = computed(() => {
  if (!deleteTarget.value?.metadata?.createdAt)
    return null
  const ts = deleteTarget.value.metadata.createdAt
  return new Date(ts * 1000).toLocaleDateString()
})

defineExpose({ refresh: fetchFiles })
</script>

<template>
  <div class="flex flex-col h-full">
    <!-- Header -->
    <div class="px-2 py-1.5 border-b flex shrink-0 gap-2 items-center">
      <span
        class="i-carbon-chevron-right shrink-0 h-3 w-3 cursor-pointer transition-transform"
        :class="{ 'rotate-90': !isCollapsed }"
        @click="toggleExpand"
      />
      <span class="i-carbon-document-blank text-muted-foreground shrink-0 h-3.5 w-3.5" />
      <span class="text-xs text-muted-foreground font-semibold flex-1 cursor-pointer truncate uppercase" @click="toggleExpand">
        {{ t('sidebar.savedQueries.header') }}
      </span>
      <span v-if="files.length > 0" class="text-xs text-muted-foreground px-1.5 py-0.5 rounded bg-muted">
        {{ files.length }}
      </span>
      <Button
        variant="ghost"
        size="icon"
        class="shrink-0 h-5 w-5"
        :title="t('sidebar.savedQueries.newQuery')"
        @click="emit('newQuery')"
      >
        <span class="i-carbon-add h-3 w-3" />
      </Button>
    </div>

    <!-- Body (when expanded) -->
    <div v-if="!isCollapsed" class="flex-1 overflow-auto">
      <div v-if="loading" class="text-xs text-muted-foreground px-2 py-2 flex gap-2 items-center">
        <span class="i-carbon-loading h-3 w-3 animate-spin" />
        {{ t('sidebar.loading') }}
      </div>
      <template v-else>
        <SavedQueryItem
          v-for="file in sortedFiles"
          :key="file.file_path"
          :query="file"
          :metadata="getMetadata(file.file_path)"
          @action="(kind: string) => handleItemAction(file, kind)"
        />
        <div v-if="sortedFiles.length === 0" class="text-xs text-muted-foreground px-2 py-2">
          {{ t('sidebar.savedQueries.empty') }}
        </div>
      </template>
    </div>
  </div>

  <!-- Delete Dialog -->
  <DeleteQueryDialog
    v-model:open="showDeleteDialog"
    :query-name="deleteTarget?.query.file_name ?? null"
    :target-connection="deleteTarget?.metadata?.connectionName ?? null"
    :created-at="deleteDialogCreatedAt"
    @confirm="handleDeleteConfirm"
  />

  <!-- Rename Dialog -->
  <RenameQueryDialog
    v-model:open="showRenameDialog"
    :current-name="renameTarget?.file_name ?? null"
    @confirm="handleRenameConfirm"
  />

  <!-- Change Connection Dialog -->
  <ChangeConnectionDialog
    v-model:open="showChangeConnDialog"
    :current-connection-id="changeConnTarget ? (getMetadata(changeConnTarget.file_path)?.connectionId ?? null) : null"
    @confirm="handleChangeConnectionConfirm"
  />
</template>
