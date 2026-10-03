<script setup lang="ts">
import type { DatabaseFileAssessment, DatabaseFileFormat, DatabaseFileProbe, RecentDatabaseEntry } from '@/utils/databaseFiles'
import { invoke } from '@tauri-apps/api/core'
import { open as openDialog, save as saveDialog } from '@tauri-apps/plugin-dialog'
import { revealItemInDir } from '@tauri-apps/plugin-opener'
import { computed, onMounted, ref, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import { Button } from '@/components/ui/button'
import { Input } from '@/components/ui/input'
import { Label } from '@/components/ui/label'
import { toast } from '@/composables/useNotifications'
import {
  assessDatabaseFile,
  DUCKDB_EXTENSIONS,
  formatFileSize,
  mergeRecentDatabase,
  recentDatabasesKey,
  SQLITE_EXTENSIONS,
  storageVersionMinRelease,
  suggestDatabaseFileName,
  withDatabaseExtension,
} from '@/utils/databaseFiles'

const props = defineProps<{
  modelValue: string
  engine: 'sqlite' | 'duckdb'
  label: string
  placeholder: string
  errorMessage?: string
  required?: boolean
  /** Base name offered to the save dialog when creating a new file. */
  suggestedName?: string
}>()

const emit = defineEmits<{
  (e: 'update:modelValue', value: string): void
  (e: 'status', value: DatabaseFileAssessment): void
  (e: 'picking', value: boolean): void
}>()

const { t } = useI18n()

const isPicking = ref(false)
const probeResult = ref<DatabaseFileProbe | null>(null)
const recentDatabases = ref<RecentDatabaseEntry[]>([])

const pathModel = computed({
  get: () => props.modelValue,
  set: value => emit('update:modelValue', value),
})

const expectedFormat = computed<DatabaseFileFormat>(() => (props.engine === 'duckdb' ? 'duckdb' : 'sqlite'))

const filters = computed(() => props.engine === 'duckdb'
  ? [
      { name: 'DuckDB', extensions: DUCKDB_EXTENSIONS },
      { name: 'All Files', extensions: ['*'] },
    ]
  : [
      { name: 'Database', extensions: SQLITE_EXTENSIONS },
      { name: 'All Files', extensions: ['*'] },
    ])

const storageKey = computed(() => recentDatabasesKey(props.engine))

const assessment = computed(() => assessDatabaseFile(probeResult.value, expectedFormat.value, props.modelValue))

function loadRecentDatabases() {
  try {
    const stored = localStorage.getItem(storageKey.value)
    recentDatabases.value = stored ? JSON.parse(stored) : []
  }
  catch {
    recentDatabases.value = []
  }
}

function persistRecentDatabases(entries: RecentDatabaseEntry[]) {
  recentDatabases.value = entries
  localStorage.setItem(storageKey.value, JSON.stringify(entries))
}

function rememberDatabase(path: string) {
  return persistRecentDatabases(mergeRecentDatabase(recentDatabases.value, path, Date.now()))
}

function removeRecentDatabase(path: string) {
  return persistRecentDatabases(recentDatabases.value.filter(entry => entry.path !== path))
}

async function runProbe(path: string) {
  const trimmed = (path ?? '').trim()
  if (!trimmed || trimmed.startsWith('memory:') || trimmed === ':memory:') {
    probeResult.value = null
    return
  }
  try {
    probeResult.value = await invoke<DatabaseFileProbe>('probe_database_file', { path: trimmed })
  }
  catch {
    probeResult.value = null
  }
}

let probeTimer: ReturnType<typeof setTimeout> | undefined

watch(() => props.modelValue, (value) => {
  if (probeTimer)
    clearTimeout(probeTimer)
  probeTimer = setTimeout(() => runProbe(value ?? ''), 250)
})

watch(assessment, value => emit('status', value), { immediate: true })

onMounted(() => {
  loadRecentDatabases()
  runProbe(props.modelValue ?? '')
})

function selectPath(path: string) {
  pathModel.value = path
}

function applyPicked(path: string) {
  const normalized = props.engine === 'duckdb' ? withDatabaseExtension(path, 'duckdb') : path
  selectPath(normalized)
  rememberDatabase(normalized)
  runProbe(normalized)
}

async function createDatabaseFile() {
  isPicking.value = true
  emit('picking', true)
  try {
    const selected = await saveDialog({
      defaultPath: props.suggestedName || suggestDatabaseFileName(),
      filters: filters.value,
    })
    if (typeof selected === 'string' && selected)
      applyPicked(selected)
  }
  catch (error) {
    toast.error(t('components.serverForm.errors.filePickerFailed'), {
      description: error instanceof Error ? error.message : String(error),
    })
  }
  finally {
    isPicking.value = false
    emit('picking', false)
  }
}

async function openDatabaseFile() {
  isPicking.value = true
  emit('picking', true)
  try {
    const selected = await openDialog({ multiple: false, filters: filters.value })
    if (typeof selected === 'string' && selected)
      applyPicked(selected)
  }
  catch (error) {
    toast.error(t('components.serverForm.errors.filePickerFailed'), {
      description: error instanceof Error ? error.message : String(error),
    })
  }
  finally {
    isPicking.value = false
    emit('picking', false)
  }
}

async function createParentDirectory() {
  try {
    probeResult.value = await invoke<DatabaseFileProbe>('create_database_directory', {
      path: props.modelValue.trim(),
    })
  }
  catch (error) {
    toast.error(t('components.serverForm.fileStatus.createDirectoryFailed'), {
      description: error instanceof Error ? error.message : String(error),
    })
  }
}

async function revealDatabaseFile(path: string) {
  try {
    await revealItemInDir(path)
  }
  catch (error) {
    toast.error(t('components.serverForm.fileStatus.revealFailed'), {
      description: error instanceof Error ? error.message : String(error),
    })
  }
}

type StatusDescriptor = {
  tone: 'info' | 'success' | 'warning' | 'error'
  message: string
  action?: 'create-directory'
}

const status = computed<StatusDescriptor | null>(() => {
  const value = assessment.value
  const engineName = props.engine === 'duckdb' ? 'DuckDB' : 'SQLite'
  const formatName = value.format === 'sqlite' ? 'SQLite' : t('components.serverForm.fileStatus.unknownFormat')

  const descriptors: Record<Exclude<DatabaseFileAssessment['status'], 'memory'>, StatusDescriptor> = {
    'unknown': { tone: 'info', message: '' },
    'will-create': { tone: 'info', message: t('components.serverForm.fileStatus.willCreate', { engine: engineName }) },
    'missing-parent': {
      tone: 'warning',
      message: t('components.serverForm.fileStatus.missingParent'),
      action: 'create-directory',
    },
    'unwritable': { tone: 'warning', message: t('components.serverForm.fileStatus.unwritable') },
    'directory': { tone: 'error', message: t('components.serverForm.fileStatus.directory') },
    'empty': { tone: 'info', message: t('components.serverForm.fileStatus.empty') },
    'ready': { tone: 'success', message: t('components.serverForm.fileStatus.ready', { size: formatFileSize(value.sizeBytes) }) },
    'unsupported-path': { tone: 'error', message: t('components.serverForm.fileStatus.unsupportedPath') },
    'wrong-format': {
      tone: 'warning',
      message: t('components.serverForm.fileStatus.wrongFormat', { format: formatName, engine: engineName }),
    },
  }

  if (value.status === 'memory')
    return null

  const descriptor = descriptors[value.status]
  return descriptor.message ? descriptor : null
})

const fileDetails = computed(() => {
  const value = probeResult.value
  const details: string[] = []
  if (!value)
    return details

  const storageVersion = value.storageVersion
  if (storageVersion !== null && storageVersion !== undefined) {
    const minRelease = storageVersionMinRelease(storageVersion)
    details.push(minRelease
      ? t('components.serverForm.fileStatus.storageVersion', { version: minRelease, storage: storageVersion })
      : t('components.serverForm.fileStatus.storageVersionUnknown', { storage: storageVersion }))
  }
  if (value.hasWal)
    details.push(t('components.serverForm.fileStatus.writeAheadLog'))

  return details
})

const toneClass: Record<StatusDescriptor['tone'], string> = {
  info: 'text-muted-foreground',
  success: 'text-muted-foreground',
  warning: 'text-amber-600 dark:text-amber-500',
  error: 'text-destructive',
}

const toneIcon: Record<StatusDescriptor['tone'], string> = {
  info: 'i-carbon-information',
  success: 'i-carbon-checkmark',
  warning: 'i-carbon-warning',
  error: 'i-carbon-warning-alt',
}
</script>

<template>
  <div class="space-y-2">
    <Label :for="`db-file-${engine}`">
      {{ label }}<span v-if="required" class="text-destructive ml-0.5">*</span>
    </Label>
    <div class="flex gap-2 items-center">
      <Input
        :id="`db-file-${engine}`"
        v-model="pathModel"
        :placeholder="placeholder"
        :class="{ 'border-destructive': errorMessage }"
        class="flex-1"
      />
      <Button type="button" variant="outline" size="sm" :disabled="isPicking" @click="createDatabaseFile">
        {{ t('components.serverForm.buttons.create') }}
      </Button>
      <Button type="button" variant="outline" size="sm" :disabled="isPicking" @click="openDatabaseFile">
        {{ t('components.serverForm.buttons.open') }}
      </Button>
    </div>

    <p v-if="errorMessage" class="text-sm text-destructive">
      {{ errorMessage }}
    </p>

    <div v-if="status" class="text-xs flex gap-2 items-center" :class="toneClass[status.tone]">
      <span class="shrink-0 h-3.5 w-3.5" :class="toneIcon[status.tone]" />
      <span class="flex-1">{{ status.message }}</span>
      <Button
        v-if="status.action === 'create-directory'"
        type="button"
        variant="ghost"
        size="sm"
        class="text-xs px-2 h-6"
        @click="createParentDirectory"
      >
        {{ t('components.serverForm.fileStatus.createDirectory') }}
      </Button>
    </div>

    <div v-if="fileDetails.length > 0" class="text-xs text-muted-foreground flex flex-wrap gap-x-3 gap-y-1">
      <span v-for="detail in fileDetails" :key="detail">{{ detail }}</span>
    </div>

    <div v-if="recentDatabases.length > 0" class="space-y-1">
      <Label class="text-xs text-muted-foreground">{{ t('components.serverForm.labels.recentDatabases') }}</Label>
      <div class="p-1 border rounded-md max-h-32 overflow-y-auto space-y-0.5">
        <div
          v-for="entry in recentDatabases"
          :key="entry.path"
          class="group text-sm p-1 rounded flex gap-2 cursor-pointer items-center hover:bg-muted"
          @click="selectPath(entry.path)"
        >
          <span class="i-carbon-document text-muted-foreground shrink-0 h-4 w-4" />
          <span class="flex-1 truncate" :title="entry.path">{{ entry.path }}</span>
          <button
            type="button"
            class="text-muted-foreground opacity-0 shrink-0 hover:text-foreground group-hover:opacity-100"
            :title="t('components.serverForm.fileStatus.reveal')"
            @click.stop="revealDatabaseFile(entry.path)"
          >
            <span class="i-carbon-launch h-4 w-4 block" />
          </button>
          <button
            type="button"
            class="text-muted-foreground opacity-0 shrink-0 hover:text-destructive group-hover:opacity-100"
            :title="t('components.serverForm.fileStatus.removeRecent')"
            @click.stop="removeRecentDatabase(entry.path)"
          >
            <span class="i-carbon-close h-4 w-4 block" />
          </button>
        </div>
      </div>
    </div>
  </div>
</template>
