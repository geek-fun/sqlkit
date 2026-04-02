<script setup lang="ts">
import type { ServerConnection } from '@/store'
import { invoke } from '@tauri-apps/api/core'
import { computed, ref, watch } from 'vue'
import { useI18n } from 'vue-i18n'
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
import { useDatabaseIcon } from '@/composables/useDatabaseIcon'
import { DatabaseType, resolveDatabase } from '@/store'
import { DEFAULT_SSL_MODE, sslModeToBackend, validateSslConfig } from '@/types/connection'
import SslConfigSection from './ssl/SslConfigSection.vue'

const props = defineProps<{
  open: boolean
  connection: ServerConnection | null
}>()

const emit = defineEmits<{
  (e: 'update:open', value: boolean): void
  (e: 'save', connection: ServerConnection): void
}>()

const { t } = useI18n()
const { getDatabaseIcon } = useDatabaseIcon()

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
  ssl: { mode: DEFAULT_SSL_MODE },
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
    errors.name = t('components.serverForm.errors.nameRequired')
  }

  if (formData.value.type === DatabaseType.SQLITE) {
    if (!formData.value.host.trim()) {
      errors.host = t('components.serverForm.errors.filePathRequired')
    }
  }
  else {
    if (!formData.value.host.trim()) {
      errors.host = t('components.serverForm.errors.hostRequired')
    }
    if (!formData.value.port || formData.value.port <= 0) {
      errors.port = t('components.serverForm.errors.portInvalid')
    }

    const dbTypeBackend = mapDatabaseTypeToBackend(formData.value.type)
    const sslErrors = validateSslConfig(formData.value.ssl, dbTypeBackend)
    sslErrors.forEach((err) => {
      errors[`ssl.${err.field}`] = err.message
    })
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
    const config = {
      id: formData.value.id || crypto.randomUUID(),
      name: formData.value.name,
      db_type: mapDatabaseTypeToBackend(formData.value.type),
      host: formData.value.host,
      port: formData.value.port,
      username: formData.value.username || '',
      password: formData.value.password || undefined,
      database: resolveDatabase(formData.value.type, formData.value.database) ?? undefined,
      ssl_mode: sslModeToBackend(formData.value.ssl),
      ssl_ca_cert: formData.value.ssl.caCertPath || null,
      ssl_client_cert: formData.value.ssl.clientCertPath || null,
      ssl_client_key: formData.value.ssl.clientKeyPath || null,
      trust_server_certificate: formData.value.ssl.trustServerCertificate ?? null,
    }

    const result = await invoke<{ is_connected: boolean, server_version?: string }>('test_connection', { config })

    if (result.is_connected) {
      testStatus.value = 'success'
    }
    else {
      testStatus.value = 'error'
      testError.value = t('common.status.failed')
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

const isSqlite = computed(() => formData.value.type === DatabaseType.SQLITE)
</script>

<template>
  <Dialog v-model:open="isOpen">
    <DialogContent class="sm:max-w-lg">
      <DialogTitle>
        {{ isEditing ? t('components.serverForm.title.edit') : t('components.serverForm.title.new') }}
      </DialogTitle>
      <DialogDescription>
        {{ isEditing ? t('components.serverForm.description.edit') : t('components.serverForm.description.new') }}
      </DialogDescription>

      <form class="space-y-4" @submit.prevent="handleSave">
        <!-- Connection Name -->
        <div class="space-y-2">
          <Label for="name">{{ t('components.serverForm.labels.connectionName') }}</Label>
          <Input
            id="name"
            v-model="formData.name"
            :placeholder="t('components.serverForm.placeholders.connectionName')"
            :class="{ 'border-destructive': formErrors.name }"
          />
          <p v-if="formErrors.name" class="text-sm text-destructive">
            {{ formErrors.name }}
          </p>
        </div>

        <!-- Database Type -->
        <div class="space-y-2">
          <Label for="type">{{ t('components.serverForm.labels.databaseType') }}</Label>
          <Select :model-value="formData.type" @update:model-value="handleDatabaseTypeChange">
            <SelectTrigger>
              <SelectValue :placeholder="t('components.serverForm.placeholders.selectType')" />
            </SelectTrigger>
            <SelectContent>
              <SelectItem :value="DatabaseType.POSTGRESQL">
                <div class="flex gap-2 items-center">
                  <img :src="getDatabaseIcon(DatabaseType.POSTGRESQL)" alt="PostgreSQL" class="h-5 w-5 object-contain">
                  {{ t('components.serverForm.databaseTypes.postgresql') }}
                </div>
              </SelectItem>
              <SelectItem :value="DatabaseType.SQLSERVER">
                <div class="flex gap-2 items-center">
                  <img :src="getDatabaseIcon(DatabaseType.SQLSERVER)" alt="SQL Server" class="h-5 w-5 object-contain">
                  {{ t('components.serverForm.databaseTypes.sqlserver') }}
                </div>
              </SelectItem>
              <SelectItem :value="DatabaseType.MYSQL">
                <div class="flex gap-2 items-center">
                  <img :src="getDatabaseIcon(DatabaseType.MYSQL)" alt="MySQL" class="h-5 w-5 object-contain">
                  {{ t('components.serverForm.databaseTypes.mysql') }}
                </div>
              </SelectItem>
              <SelectItem :value="DatabaseType.MARIADB">
                <div class="flex gap-2 items-center">
                  <img :src="getDatabaseIcon(DatabaseType.MARIADB)" alt="MariaDB" class="h-5 w-5 object-contain">
                  {{ t('components.serverForm.databaseTypes.mariadb') }}
                </div>
              </SelectItem>
              <SelectItem :value="DatabaseType.SQLITE">
                <div class="flex gap-2 items-center">
                  <img :src="getDatabaseIcon(DatabaseType.SQLITE)" alt="SQLite" class="h-5 w-5 object-contain">
                  {{ t('components.serverForm.databaseTypes.sqlite') }}
                </div>
              </SelectItem>
            </SelectContent>
          </Select>
        </div>

        <!-- Host / File path (for SQLite) -->
        <div class="space-y-2">
          <Label for="host">{{ isSqlite ? t('components.serverForm.labels.databaseFilePath') : t('components.serverForm.labels.host') }}</Label>
          <Input
            id="host"
            v-model="formData.host"
            :placeholder="isSqlite ? t('components.serverForm.placeholders.filePath') : t('components.serverForm.placeholders.host')"
            :class="{ 'border-destructive': formErrors.host }"
          />
          <p v-if="formErrors.host" class="text-sm text-destructive">
            {{ formErrors.host }}
          </p>
        </div>

        <!-- Port and Database (not for SQLite) -->
        <div v-if="!isSqlite" class="gap-4 grid grid-cols-2">
          <div class="space-y-2">
            <Label for="port">{{ t('components.serverForm.labels.port') }}</Label>
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
            <Label for="database">{{ t('components.serverForm.labels.database') }}</Label>
            <Input
              id="database"
              v-model="formData.database"
              :placeholder="t('components.serverForm.placeholders.database')"
            />
          </div>
        </div>

        <!-- Username and Password (not for SQLite) -->
        <div v-if="!isSqlite" class="gap-4 grid grid-cols-2">
          <div class="space-y-2">
            <Label for="username">{{ t('components.serverForm.labels.username') }}</Label>
            <Input
              id="username"
              v-model="formData.username"
              :placeholder="t('components.serverForm.placeholders.username')"
              autocomplete="off"
            />
          </div>
          <div class="space-y-2">
            <Label for="password">{{ t('components.serverForm.labels.password') }}</Label>
            <Input
              id="password"
              v-model="formData.password"
              type="password"
              :placeholder="t('components.serverForm.placeholders.password')"
              autocomplete="new-password"
            />
          </div>
        </div>

        <!-- SSL Configuration (not for SQLite) -->
        <SslConfigSection
          v-if="!isSqlite"
          v-model="formData.ssl"
          :db-type="formData.type"
          :errors="formErrors"
        />

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
              {{ testStatus === 'testing' ? t('common.status.testing') : testStatus === 'success' ? t('common.status.success') : t('common.status.failed') }}
            </span>
          </div>
          <p v-if="testError" class="text-sm text-red-600 mt-1 dark:text-red-500">
            {{ testError }}
          </p>
        </div>

        <!-- Actions -->
        <div class="pt-4 flex gap-2 justify-end">
          <Button
            type="button"
            variant="outline"
            :disabled="testStatus === 'testing'"
            @click="handleTestConnection"
          >
            {{ t('common.buttons.testConnection') }}
          </Button>
          <Button type="submit">
            {{ isEditing ? t('common.buttons.saveChanges') : t('common.buttons.createConnection') }}
          </Button>
        </div>
      </form>
    </DialogContent>
  </Dialog>
</template>
