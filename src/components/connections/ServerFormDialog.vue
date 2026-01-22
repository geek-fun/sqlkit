<script setup lang="ts">
import type { ServerConnection } from '@/store'
import { invoke } from '@tauri-apps/api/core'
import { computed, ref, watch } from 'vue'
import { Button } from '@/components/ui/button'
import { Dialog, DialogContent, DialogDescription, DialogTitle } from '@/components/ui/dialog'
import { Input } from '@/components/ui/input'
import { Label } from '@/components/ui/label'
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select'
import { DatabaseType } from '@/store'

const props = defineProps<{
  open: boolean
  connection: ServerConnection | null
}>()

const emit = defineEmits<{
  (e: 'update:open', value: boolean): void
  (e: 'save', connection: ServerConnection): void
}>()

const isOpen = computed({
  get: () => props.open,
  set: value => emit('update:open', value),
})

const isEditing = computed(() => !!props.connection?.id)

const defaultPorts: Record<DatabaseType, number> = {
  [DatabaseType.POSTGRESQL]: 5432,
  [DatabaseType.MYSQL]: 3306,
  [DatabaseType.MARIADB]: 3306,
  [DatabaseType.SQLITE]: 0,
  [DatabaseType.SQLSERVER]: 1433,
}

const defaultConnection: ServerConnection = {
  name: '',
  type: DatabaseType.POSTGRESQL,
  host: 'localhost',
  port: 5432,
  username: '',
  password: '',
  database: '',
  ssl: false,
}

const formData = ref<ServerConnection>({ ...defaultConnection })
const testStatus = ref<'idle' | 'testing' | 'success' | 'error'>('idle')
const testError = ref<string>('')
const formErrors = ref<Record<string, string>>({})

watch(() => props.open, (open) => {
  if (open) {
    if (props.connection) {
      formData.value = { ...props.connection }
    }
    else {
      formData.value = { ...defaultConnection }
    }
    testStatus.value = 'idle'
    testError.value = ''
    formErrors.value = {}
  }
})

function handleDatabaseTypeChange(value: string) {
  // Validate that value is a valid DatabaseType
  if (!Object.values(DatabaseType).includes(value as DatabaseType)) {
    console.error(`Invalid database type: ${value}`)
    return
  }
  const type = value as DatabaseType
  formData.value.type = type
  if (!props.connection || formData.value.port === defaultPorts[props.connection.type]) {
    formData.value.port = defaultPorts[type]
  }
  if (type === DatabaseType.SQLITE) {
    formData.value.host = ''
    formData.value.port = 0
    formData.value.username = ''
    formData.value.password = ''
  }
}

function validateForm(): boolean {
  const errors: Record<string, string> = {}

  if (!formData.value.name.trim()) {
    errors.name = 'Connection name is required'
  }

  if (formData.value.type === DatabaseType.SQLITE) {
    if (!formData.value.host.trim()) {
      errors.host = 'Database file path is required'
    }
  }
  else {
    if (!formData.value.host.trim()) {
      errors.host = 'Host is required'
    }
    if (!formData.value.port || formData.value.port <= 0) {
      errors.port = 'Port must be a positive number'
    }
  }

  formErrors.value = errors
  return Object.keys(errors).length === 0
}

async function handleTestConnection() {
  if (!validateForm()) {
    return
  }

  testStatus.value = 'testing'
  testError.value = ''

  try {
    // Prepare config for Tauri backend
    const config = {
      id: formData.value.id || crypto.randomUUID(),
      name: formData.value.name,
      db_type: mapDatabaseTypeToBackend(formData.value.type),
      host: formData.value.host,
      port: formData.value.port,
      username: formData.value.username || '',
      password: formData.value.password || undefined,
      database: formData.value.database || undefined,
      ssl_mode: formData.value.ssl ? 'require' : 'disable',
    }

    const result = await invoke<{ is_connected: boolean, server_version?: string }>('test_connection', { config })

    if (result.is_connected) {
      testStatus.value = 'success'
    }
    else {
      testStatus.value = 'error'
      testError.value = 'Connection failed'
    }
  }
  catch (error) {
    testStatus.value = 'error'
    testError.value = error instanceof Error ? error.message : String(error)
  }
}

function mapDatabaseTypeToBackend(type: DatabaseType): string {
  switch (type) {
    case DatabaseType.POSTGRESQL:
      return 'postgresql'
    case DatabaseType.MYSQL:
      return 'mysql'
    case DatabaseType.MARIADB:
      return 'mysql'
    case DatabaseType.SQLITE:
      return 'sqlite'
    case DatabaseType.SQLSERVER:
      return 'sqlserver'
    default:
      return 'postgresql'
  }
}

function handleSave() {
  if (!validateForm()) {
    return
  }

  emit('save', { ...formData.value })
  isOpen.value = false
}

function handleCancel() {
  isOpen.value = false
}

const isSqlite = computed(() => formData.value.type === DatabaseType.SQLITE)
</script>

<template>
  <Dialog v-model:open="isOpen">
    <DialogContent class="sm:max-w-lg">
      <DialogTitle>
        {{ isEditing ? 'Edit Connection' : 'New Connection' }}
      </DialogTitle>
      <DialogDescription>
        {{ isEditing ? 'Update your database connection settings.' : 'Configure a new database connection.' }}
      </DialogDescription>

      <form class="space-y-4" @submit.prevent="handleSave">
        <!-- Connection Name -->
        <div class="space-y-2">
          <Label for="name">Connection Name</Label>
          <Input
            id="name"
            v-model="formData.name"
            placeholder="My Database"
            :class="{ 'border-destructive': formErrors.name }"
          />
          <p v-if="formErrors.name" class="text-sm text-destructive">
            {{ formErrors.name }}
          </p>
        </div>

        <!-- Database Type -->
        <div class="space-y-2">
          <Label for="type">Database Type</Label>
          <Select :model-value="formData.type" @update:model-value="handleDatabaseTypeChange">
            <SelectTrigger>
              <SelectValue placeholder="Select database type" />
            </SelectTrigger>
            <SelectContent>
              <SelectItem :value="DatabaseType.POSTGRESQL">
                🐘 PostgreSQL
              </SelectItem>
              <SelectItem :value="DatabaseType.MYSQL">
                🐬 MySQL
              </SelectItem>
              <SelectItem :value="DatabaseType.MARIADB">
                🦭 MariaDB
              </SelectItem>
              <SelectItem :value="DatabaseType.SQLITE">
                📦 SQLite
              </SelectItem>
              <SelectItem :value="DatabaseType.SQLSERVER">
                🔷 SQL Server
              </SelectItem>
            </SelectContent>
          </Select>
        </div>

        <!-- Host / File path (for SQLite) -->
        <div class="space-y-2">
          <Label for="host">{{ isSqlite ? 'Database File Path' : 'Host' }}</Label>
          <Input
            id="host"
            v-model="formData.host"
            :placeholder="isSqlite ? '/path/to/database.db' : 'localhost'"
            :class="{ 'border-destructive': formErrors.host }"
          />
          <p v-if="formErrors.host" class="text-sm text-destructive">
            {{ formErrors.host }}
          </p>
        </div>

        <!-- Port and Database (not for SQLite) -->
        <div v-if="!isSqlite" class="gap-4 grid grid-cols-2">
          <div class="space-y-2">
            <Label for="port">Port</Label>
            <Input
              id="port"
              v-model.number="formData.port"
              type="number"
              :class="{ 'border-destructive': formErrors.port }"
            />
            <p v-if="formErrors.port" class="text-sm text-destructive">
              {{ formErrors.port }}
            </p>
          </div>
          <div class="space-y-2">
            <Label for="database">Database</Label>
            <Input
              id="database"
              v-model="formData.database"
              placeholder="database_name"
            />
          </div>
        </div>

        <!-- Username and Password (not for SQLite) -->
        <div v-if="!isSqlite" class="gap-4 grid grid-cols-2">
          <div class="space-y-2">
            <Label for="username">Username</Label>
            <Input
              id="username"
              v-model="formData.username"
              placeholder="username"
              autocomplete="off"
            />
          </div>
          <div class="space-y-2">
            <Label for="password">Password</Label>
            <Input
              id="password"
              v-model="formData.password"
              type="password"
              placeholder="••••••••"
              autocomplete="new-password"
            />
          </div>
        </div>

        <!-- SSL Toggle (not for SQLite) -->
        <div v-if="!isSqlite" class="flex items-center space-x-2">
          <input
            id="ssl"
            v-model="formData.ssl"
            type="checkbox"
            class="text-primary border-input rounded h-4 w-4 focus:ring-ring"
          >
          <Label for="ssl" class="cursor-pointer">
            Use SSL/TLS encryption
          </Label>
        </div>

        <!-- Test Connection Status -->
        <div
          v-if="testStatus !== 'idle'" class="p-3 rounded-md" :class="{
            'bg-blue-50 dark:bg-blue-900/10': testStatus === 'testing',
            'bg-green-50 dark:bg-green-900/10': testStatus === 'success',
            'bg-red-50 dark:bg-red-900/10': testStatus === 'error',
          }"
        >
          <div class="flex gap-2 items-center">
            <!-- Loading spinner -->
            <svg
              v-if="testStatus === 'testing'"
              class="text-blue-500 h-4 w-4 animate-spin dark:text-blue-400"
              xmlns="http://www.w3.org/2000/svg"
              fill="none"
              viewBox="0 0 24 24"
            >
              <circle class="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" stroke-width="4" />
              <path class="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z" />
            </svg>
            <!-- Success icon -->
            <svg
              v-if="testStatus === 'success'"
              class="text-green-500 h-4 w-4 dark:text-green-400"
              xmlns="http://www.w3.org/2000/svg"
              viewBox="0 0 24 24"
              fill="none"
              stroke="currentColor"
              stroke-width="2"
              stroke-linecap="round"
              stroke-linejoin="round"
            >
              <polyline points="20 6 9 17 4 12" />
            </svg>
            <!-- Error icon -->
            <svg
              v-if="testStatus === 'error'"
              class="text-red-500 h-4 w-4 dark:text-red-400"
              xmlns="http://www.w3.org/2000/svg"
              viewBox="0 0 24 24"
              fill="none"
              stroke="currentColor"
              stroke-width="2"
              stroke-linecap="round"
              stroke-linejoin="round"
            >
              <circle cx="12" cy="12" r="10" />
              <line x1="15" y1="9" x2="9" y2="15" />
              <line x1="9" y1="9" x2="15" y2="15" />
            </svg>
            <span
              :class="{
                'text-blue-700 dark:text-blue-400': testStatus === 'testing',
                'text-green-700 dark:text-green-400': testStatus === 'success',
                'text-red-700 dark:text-red-400': testStatus === 'error',
              }"
            >
              {{ testStatus === 'testing' ? 'Testing connection...' : testStatus === 'success' ? 'Connection successful!' : 'Connection failed' }}
            </span>
          </div>
          <p v-if="testError" class="text-sm text-red-600 mt-1 dark:text-red-500">
            {{ testError }}
          </p>
        </div>

        <!-- Actions -->
        <div class="pt-4 flex justify-between">
          <Button
            type="button"
            variant="outline"
            :disabled="testStatus === 'testing'"
            @click="handleTestConnection"
          >
            Test Connection
          </Button>
          <div class="flex gap-2">
            <Button type="button" variant="outline" @click="handleCancel">
              Cancel
            </Button>
            <Button type="submit">
              {{ isEditing ? 'Save Changes' : 'Create Connection' }}
            </Button>
          </div>
        </div>
      </form>
    </DialogContent>
  </Dialog>
</template>
