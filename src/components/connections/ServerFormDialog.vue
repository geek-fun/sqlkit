<script setup lang="ts">
import type { ServerConnection } from '@/store'
import { invoke } from '@tauri-apps/api/core'
import { open } from '@tauri-apps/plugin-dialog'
import { computed, ref, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import { Button } from '@/components/ui/button'
import { Dialog, DialogContent, DialogDescription, DialogTitle } from '@/components/ui/dialog'
import { Input } from '@/components/ui/input'
import { Label } from '@/components/ui/label'
import {
  Select,
  SelectContent,
  SelectGroup,
  SelectItem,
  SelectLabel,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select'
import { Tabs, TabsContent, TabsList, TabsTrigger } from '@/components/ui/tabs'
import { useDatabaseIcon } from '@/composables/useDatabaseIcon'
import { toast } from '@/composables/useNotifications'
import { DatabaseType, buildTransportLayers, dbTypeToBackend, resolveDatabase } from '@/store'
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

const defaultPorts: Record<string, number> = {
  [DatabaseType.POSTGRESQL]: 5432,
  [DatabaseType.MYSQL]: 3306,
  [DatabaseType.MARIADB]: 3306,
  [DatabaseType.SQLITE]: 0,
  [DatabaseType.SQLSERVER]: 1433,
  [DatabaseType.DUCKDB]: 0,
  [DatabaseType.CLICKHOUSE]: 8123,
  [DatabaseType.COCKROACHDB]: 5432,
  [DatabaseType.REDSHIFT]: 5439,
  [DatabaseType.YUGABYTEDB]: 5433,
  [DatabaseType.TIMESCALEDB]: 5432,
  [DatabaseType.KINGBASEES]: 54321,
  [DatabaseType.GAUSSDB]: 5432,
  [DatabaseType.HIGHGO]: 5432,
  [DatabaseType.UXDB]: 5432,
  [DatabaseType.OPENGAUSS]: 5432,
  [DatabaseType.GBASE8C]: 5432,
  [DatabaseType.QUESTDB]: 8812,
  [DatabaseType.VASTBASE]: 5432,
  [DatabaseType.YASHANDB]: 1688,
  [DatabaseType.TIDB]: 4000,
  [DatabaseType.OCEANBASE]: 2883,
  [DatabaseType.TDSQL]: 3306,
  [DatabaseType.POLARDB]: 3306,
  [DatabaseType.DM8]: 5236,
  [DatabaseType.DORIS]: 9030,
  [DatabaseType.SELECTDB]: 9030,
  [DatabaseType.STARROCKS]: 9030,
  [DatabaseType.DATABEND]: 3307,
  [DatabaseType.GOLDENDB]: 3306,
  [DatabaseType.MANTICORESEARCH]: 9306,
  [DatabaseType.ORACLE]: 1521,
  [DatabaseType.DB2]: 50000,
  [DatabaseType.H2]: 9092,
  [DatabaseType.SNOWFLAKE]: 443,
  [DatabaseType.DM8ORACLE]: 5236,
  [DatabaseType.XUGUDB]: 5138,
  [DatabaseType.GBASE8A]: 5258,
  [DatabaseType.TRINO]: 8080,
  [DatabaseType.PRESTO]: 8080,
  [DatabaseType.DERBY]: 1527,
  [DatabaseType.HIVE]: 10000,
  [DatabaseType.DATABRICKS]: 443,
  [DatabaseType.HANA]: 30015,
  [DatabaseType.TERADATA]: 1025,
  [DatabaseType.VERTICA]: 5433,
  [DatabaseType.EXASOL]: 8563,
  [DatabaseType.BIGQUERY]: 443,
  [DatabaseType.INFORMIX]: 9088,
  [DatabaseType.KYLIN]: 7070,
  [DatabaseType.CASSANDRA]: 9042,
  [DatabaseType.IRIS]: 1972,
  [DatabaseType.ACCESS]: 0,
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
const showAdvanced = ref(false)

function toggleSsh(checked: boolean) {
  if (!formData.value.sshTunnel) {
    formData.value.sshTunnel = {
      enabled: checked,
      host: '',
      port: 22,
      username: '',
      authMethod: 'password',
    }
  }
  else {
    formData.value.sshTunnel = { ...formData.value.sshTunnel, enabled: checked }
  }
}

// SQLite-specific state
const sqliteTab = ref<'file' | 'in-memory'>('file')
const recentDatabases = ref<Array<{ path: string, timestamp: number }>>([])
const savedFilePath = ref<string>('') // Preserve file path when switching to in-memory

// Load recent databases from localStorage
const RECENT_DB_KEY = 'sqlite_recent_databases'
const MAX_RECENT_DB = 10

function loadRecentDatabases() {
  try {
    const stored = localStorage.getItem(RECENT_DB_KEY)
    if (stored)
      recentDatabases.value = JSON.parse(stored)
  }
  catch {
    recentDatabases.value = []
  }
}

function saveRecentDatabase(path: string) {
  const updated = [{ path, timestamp: Date.now() }, ...recentDatabases.value.filter(db => db.path !== path)].slice(0, MAX_RECENT_DB)
  recentDatabases.value = updated
  localStorage.setItem(RECENT_DB_KEY, JSON.stringify(updated))
}

function formatTimeAgo(timestamp: number): string {
  const seconds = Math.floor((Date.now() - timestamp) / 1000)
  const minutes = Math.floor(seconds / 60)
  const hours = Math.floor(minutes / 60)
  const days = Math.floor(hours / 24)

  if (days > 0)
    return t('components.serverForm.sqlite.ago', { time: `${days}d` })
  if (hours > 0)
    return t('components.serverForm.sqlite.ago', { time: `${hours}h` })
  if (minutes > 0)
    return t('components.serverForm.sqlite.ago', { time: `${minutes}m` })
  return t('components.serverForm.sqlite.ago', { time: `${seconds}s` })
}

watch(() => props.open, (open) => {
  if (open) {
    if (props.connection) {
      formData.value = { ...props.connection }
      // Detect SQLite mode from path
      if (props.connection.type === DatabaseType.SQLITE) {
        if (props.connection.host === ':memory:') {
          sqliteTab.value = 'in-memory'
        }
        else {
          sqliteTab.value = 'file'
          savedFilePath.value = props.connection.host
        }
      }
    }
    else {
      formData.value = { ...defaultConnection }
      sqliteTab.value = 'file'
      savedFilePath.value = ''
    }
    testStatus.value = 'idle'
    testError.value = ''
    formErrors.value = {}
    loadRecentDatabases()
  }
})

// Watch for SQLite tab changes to preserve file path
watch(sqliteTab, (tab) => {
  if (formData.value.type === DatabaseType.SQLITE) {
    if (tab === 'in-memory') {
      // Save current file path before switching to in-memory
      if (formData.value.host !== ':memory:' && formData.value.host.trim()) {
        savedFilePath.value = formData.value.host
      }
      formData.value.host = ':memory:'
    }
    else {
      // Restore saved file path when switching back to file mode
      formData.value.host = savedFilePath.value
    }
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
  if (type === DatabaseType.SQLITE || type === DatabaseType.DUCKDB) {
    formData.value.host = ''
    formData.value.port = 0
    formData.value.username = ''
    formData.value.password = ''
    sqliteTab.value = 'file'
    savedFilePath.value = ''
  }
}

const isFileBased = computed(() =>
  formData.value.type === DatabaseType.SQLITE
  || formData.value.type === DatabaseType.DUCKDB,
)

// SQLite file picker function - handles both open existing and create new
async function selectDatabaseFile() {
  try {
    const selected = await open({
      multiple: false,
      filters: [
        { name: 'SQLite', extensions: ['db', 'sqlite', 'sqlite3'] },
        { name: 'DuckDB', extensions: ['duckdb', 'db'] },
        { name: 'All Files', extensions: ['*'] },
      ],
    })
    if (typeof selected === 'string') {
      formData.value.host = selected
    }
  }
  catch (error) {
    toast.error(t('components.serverForm.errors.filePickerFailed'), {
      description: error instanceof Error ? error.message : String(error),
    })
  }
}

// Handle recent database selection
function selectRecentDatabase(path: string) {
  formData.value.host = path
}

function validateForm(): boolean {
  const errors: Record<string, string> = {}

  if (!formData.value.name.trim()) {
    errors.name = t('components.serverForm.errors.nameRequired')
  }

  if (isFileBased.value) {
    if (formData.value.host !== ':memory:' && !formData.value.host.trim()) {
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
      transport_layers: buildTransportLayers(formData.value.sshTunnel),
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
  return dbTypeToBackend[type] ?? 'PostgreSQL'
}

function handleSave() {
  if (!validateForm()) {
    return
  }

  // Save recent database path for SQLite
  if (formData.value.type === DatabaseType.SQLITE && formData.value.host !== ':memory:') {
    saveRecentDatabase(formData.value.host)
  }

  emit('save', { ...formData.value })
  isOpen.value = false
}
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
              <SelectGroup>
                <SelectLabel>{{ t('components.serverForm.databaseGroups.native') }}</SelectLabel>
                <SelectItem :value="DatabaseType.POSTGRESQL">
                  <div class="flex gap-2 items-center">
                    <img :src="getDatabaseIcon(DatabaseType.POSTGRESQL)" alt="" class="h-5 w-5 object-contain">
                    {{ t('components.serverForm.databaseTypes.postgresql') }}
                  </div>
                </SelectItem>
                <SelectItem :value="DatabaseType.MYSQL">
                  <div class="flex gap-2 items-center">
                    <img :src="getDatabaseIcon(DatabaseType.MYSQL)" alt="" class="h-5 w-5 object-contain">
                    {{ t('components.serverForm.databaseTypes.mysql') }}
                  </div>
                </SelectItem>
                <SelectItem :value="DatabaseType.SQLSERVER">
                  <div class="flex gap-2 items-center">
                    <img :src="getDatabaseIcon(DatabaseType.SQLSERVER)" alt="" class="h-5 w-5 object-contain">
                    {{ t('components.serverForm.databaseTypes.sqlserver') }}
                  </div>
                </SelectItem>
                <SelectItem :value="DatabaseType.SQLITE">
                  <div class="flex gap-2 items-center">
                    <img :src="getDatabaseIcon(DatabaseType.SQLITE)" alt="" class="h-5 w-5 object-contain">
                    {{ t('components.serverForm.databaseTypes.sqlite') }}
                  </div>
                </SelectItem>
                <SelectItem :value="DatabaseType.MARIADB">
                  <div class="flex gap-2 items-center">
                    <img :src="getDatabaseIcon(DatabaseType.MARIADB)" alt="" class="h-5 w-5 object-contain">
                    {{ t('components.serverForm.databaseTypes.mariadb') }}
                  </div>
                </SelectItem>
                <SelectItem :value="DatabaseType.CLICKHOUSE">
                  <div class="flex gap-2 items-center">
                    <img :src="getDatabaseIcon(DatabaseType.CLICKHOUSE)" alt="" class="h-5 w-5 object-contain">
                    {{ t('components.serverForm.databaseTypes.clickhouse') }}
                  </div>
                </SelectItem>
                <SelectItem :value="DatabaseType.DUCKDB">
                  <div class="flex gap-2 items-center">
                    <img :src="getDatabaseIcon(DatabaseType.DUCKDB)" alt="" class="h-5 w-5 object-contain">
                    {{ t('components.serverForm.databaseTypes.duckdb') }}
                  </div>
                </SelectItem>
                <SelectItem :value="DatabaseType.TIDB">
                  <div class="flex gap-2 items-center">
                    <img :src="getDatabaseIcon(DatabaseType.TIDB)" alt="" class="h-5 w-5 object-contain">
                    {{ t('components.serverForm.databaseTypes.tidb') }}
                  </div>
                </SelectItem>
                <SelectItem :value="DatabaseType.OCEANBASE">
                  <div class="flex gap-2 items-center">
                    <img :src="getDatabaseIcon(DatabaseType.OCEANBASE)" alt="" class="h-5 w-5 object-contain">
                    {{ t('components.serverForm.databaseTypes.oceanbase') }}
                  </div>
                </SelectItem>
                <SelectItem :value="DatabaseType.POLARDB">
                  <div class="flex gap-2 items-center">
                    <img :src="getDatabaseIcon(DatabaseType.POLARDB)" alt="" class="h-5 w-5 object-contain">
                    {{ t('components.serverForm.databaseTypes.polardb') }}
                  </div>
                </SelectItem>
                <SelectItem :value="DatabaseType.TDSQL">
                  <div class="flex gap-2 items-center">
                    <img :src="getDatabaseIcon(DatabaseType.TDSQL)" alt="" class="h-5 w-5 object-contain">
                    {{ t('components.serverForm.databaseTypes.tdsql') }}
                  </div>
                </SelectItem>
                <SelectItem :value="DatabaseType.DM8">
                  <div class="flex gap-2 items-center">
                    <img :src="getDatabaseIcon(DatabaseType.DM8)" alt="" class="h-5 w-5 object-contain">
                    {{ t('components.serverForm.databaseTypes.dm8') }}
                  </div>
                </SelectItem>
                <SelectItem :value="DatabaseType.DORIS">
                  <div class="flex gap-2 items-center">
                    <img :src="getDatabaseIcon(DatabaseType.DORIS)" alt="" class="h-5 w-5 object-contain">
                    {{ t('components.serverForm.databaseTypes.doris') }}
                  </div>
                </SelectItem>
                <SelectItem :value="DatabaseType.SELECTDB">
                  <div class="flex gap-2 items-center">
                    <img :src="getDatabaseIcon(DatabaseType.SELECTDB)" alt="" class="h-5 w-5 object-contain">
                    {{ t('components.serverForm.databaseTypes.selectdb') }}
                  </div>
                </SelectItem>
                <SelectItem :value="DatabaseType.STARROCKS">
                  <div class="flex gap-2 items-center">
                    <img :src="getDatabaseIcon(DatabaseType.STARROCKS)" alt="" class="h-5 w-5 object-contain">
                    {{ t('components.serverForm.databaseTypes.starrocks') }}
                  </div>
                </SelectItem>
                <SelectItem :value="DatabaseType.DATABEND">
                  <div class="flex gap-2 items-center">
                    <img :src="getDatabaseIcon(DatabaseType.DATABEND)" alt="" class="h-5 w-5 object-contain">
                    {{ t('components.serverForm.databaseTypes.databend') }}
                  </div>
                </SelectItem>
                <SelectItem :value="DatabaseType.GOLDENDB">
                  <div class="flex gap-2 items-center">
                    <img :src="getDatabaseIcon(DatabaseType.GOLDENDB)" alt="" class="h-5 w-5 object-contain">
                    {{ t('components.serverForm.databaseTypes.goldendb') }}
                  </div>
                </SelectItem>
                <SelectItem :value="DatabaseType.MANTICORESEARCH">
                  <div class="flex gap-2 items-center">
                    <img :src="getDatabaseIcon(DatabaseType.MANTICORESEARCH)" alt="" class="h-5 w-5 object-contain">
                    {{ t('components.serverForm.databaseTypes.manticore') }}
                  </div>
                </SelectItem>
                <SelectItem :value="DatabaseType.KINGBASEES">
                  <div class="flex gap-2 items-center">
                    <img :src="getDatabaseIcon(DatabaseType.KINGBASEES)" alt="" class="h-5 w-5 object-contain">
                    {{ t('components.serverForm.databaseTypes.kingbasees') }}
                  </div>
                </SelectItem>
                <SelectItem :value="DatabaseType.GAUSSDB">
                  <div class="flex gap-2 items-center">
                    <img :src="getDatabaseIcon(DatabaseType.GAUSSDB)" alt="" class="h-5 w-5 object-contain">
                    {{ t('components.serverForm.databaseTypes.gaussdb') }}
                  </div>
                </SelectItem>
                <SelectItem :value="DatabaseType.HIGHGO">
                  <div class="flex gap-2 items-center">
                    <img :src="getDatabaseIcon(DatabaseType.HIGHGO)" alt="" class="h-5 w-5 object-contain">
                    {{ t('components.serverForm.databaseTypes.highgo') }}
                  </div>
                </SelectItem>
                <SelectItem :value="DatabaseType.OPENGAUSS">
                  <div class="flex gap-2 items-center">
                    <img :src="getDatabaseIcon(DatabaseType.OPENGAUSS)" alt="" class="h-5 w-5 object-contain">
                    {{ t('components.serverForm.databaseTypes.opengauss') }}
                  </div>
                </SelectItem>
                <SelectItem :value="DatabaseType.QUESTDB">
                  <div class="flex gap-2 items-center">
                    <img :src="getDatabaseIcon(DatabaseType.QUESTDB)" alt="" class="h-5 w-5 object-contain">
                    {{ t('components.serverForm.databaseTypes.questdb') }}
                  </div>
                </SelectItem>
                <SelectItem :value="DatabaseType.VASTBASE">
                  <div class="flex gap-2 items-center">
                    <img :src="getDatabaseIcon(DatabaseType.VASTBASE)" alt="" class="h-5 w-5 object-contain">
                    {{ t('components.serverForm.databaseTypes.vastbase') }}
                  </div>
                </SelectItem>
                <SelectItem :value="DatabaseType.YASHANDB">
                  <div class="flex gap-2 items-center">
                    <img :src="getDatabaseIcon(DatabaseType.YASHANDB)" alt="" class="h-5 w-5 object-contain">
                    {{ t('components.serverForm.databaseTypes.yashandb') }}
                  </div>
                </SelectItem>
                <SelectItem :value="DatabaseType.GBASE8C">
                  <div class="flex gap-2 items-center">
                    <img :src="getDatabaseIcon(DatabaseType.GBASE8C)" alt="" class="h-5 w-5 object-contain">
                    {{ t('components.serverForm.databaseTypes.gbase8c') }}
                  </div>
                </SelectItem>
                <SelectItem :value="DatabaseType.UXDB">
                  <div class="flex gap-2 items-center">
                    <img :src="getDatabaseIcon(DatabaseType.UXDB)" alt="" class="h-5 w-5 object-contain">
                    {{ t('components.serverForm.databaseTypes.uxdb') }}
                  </div>
                </SelectItem>
                <SelectItem :value="DatabaseType.COCKROACHDB">
                  <div class="flex gap-2 items-center">
                    <img :src="getDatabaseIcon(DatabaseType.COCKROACHDB)" alt="" class="h-5 w-5 object-contain">
                    {{ t('components.serverForm.databaseTypes.cockroachdb') }}
                  </div>
                </SelectItem>
                <SelectItem :value="DatabaseType.REDSHIFT">
                  <div class="flex gap-2 items-center">
                    <img :src="getDatabaseIcon(DatabaseType.REDSHIFT)" alt="" class="h-5 w-5 object-contain">
                    {{ t('components.serverForm.databaseTypes.redshift') }}
                  </div>
                </SelectItem>
                <SelectItem :value="DatabaseType.YUGABYTEDB">
                  <div class="flex gap-2 items-center">
                    <img :src="getDatabaseIcon(DatabaseType.YUGABYTEDB)" alt="" class="h-5 w-5 object-contain">
                    {{ t('components.serverForm.databaseTypes.yugabytedb') }}
                  </div>
                </SelectItem>
                <SelectItem :value="DatabaseType.TIMESCALEDB">
                  <div class="flex gap-2 items-center">
                    <img :src="getDatabaseIcon(DatabaseType.TIMESCALEDB)" alt="" class="h-5 w-5 object-contain">
                    {{ t('components.serverForm.databaseTypes.timescaledb') }}
                  </div>
                </SelectItem>
                <SelectItem :value="DatabaseType.TRINO">
                  <div class="flex gap-2 items-center">
                    <img :src="getDatabaseIcon(DatabaseType.TRINO)" alt="" class="h-5 w-5 object-contain">
                    {{ t('components.serverForm.databaseTypes.trino') }}
                  </div>
                </SelectItem>
                <SelectItem :value="DatabaseType.PRESTO">
                  <div class="flex gap-2 items-center">
                    <img :src="getDatabaseIcon(DatabaseType.PRESTO)" alt="" class="h-5 w-5 object-contain">
                    {{ t('components.serverForm.databaseTypes.presto') }}
                  </div>
                </SelectItem>
              </SelectGroup>

              <SelectGroup>
                <SelectLabel>{{ t('components.serverForm.databaseGroups.jdbc') }}</SelectLabel>
                <SelectItem :value="DatabaseType.ORACLE">
                  <div class="flex gap-2 items-center">
                    <img :src="getDatabaseIcon(DatabaseType.ORACLE)" alt="" class="h-5 w-5 object-contain">
                    {{ t('components.serverForm.databaseTypes.oracle') }}
                    <span class="text-xs text-muted-foreground ml-auto">{{ t('components.serverForm.databaseBadges.jdbc') }}</span>
                  </div>
                </SelectItem>
                <SelectItem :value="DatabaseType.SNOWFLAKE">
                  <div class="flex gap-2 items-center">
                    <img :src="getDatabaseIcon(DatabaseType.SNOWFLAKE)" alt="" class="h-5 w-5 object-contain">
                    {{ t('components.serverForm.databaseTypes.snowflake') }}
                    <span class="text-xs text-muted-foreground ml-auto">{{ t('components.serverForm.databaseBadges.jdbc') }}</span>
                  </div>
                </SelectItem>
                <SelectItem :value="DatabaseType.DB2">
                  <div class="flex gap-2 items-center">
                    <img :src="getDatabaseIcon(DatabaseType.DB2)" alt="" class="h-5 w-5 object-contain">
                    {{ t('components.serverForm.databaseTypes.db2') }}
                    <span class="text-xs text-muted-foreground ml-auto">{{ t('components.serverForm.databaseBadges.jdbc') }}</span>
                  </div>
                </SelectItem>
                <SelectItem :value="DatabaseType.H2">
                  <div class="flex gap-2 items-center">
                    <img :src="getDatabaseIcon(DatabaseType.H2)" alt="" class="h-5 w-5 object-contain">
                    {{ t('components.serverForm.databaseTypes.h2') }}
                    <span class="text-xs text-muted-foreground ml-auto">{{ t('components.serverForm.databaseBadges.jdbc') }}</span>
                  </div>
                </SelectItem>
                <SelectItem :value="DatabaseType.DM8ORACLE">
                  <div class="flex gap-2 items-center">
                    <img :src="getDatabaseIcon(DatabaseType.DM8ORACLE)" alt="" class="h-5 w-5 object-contain">
                    {{ t('components.serverForm.databaseTypes.dm8oracle') }}
                    <span class="text-xs text-muted-foreground ml-auto">{{ t('components.serverForm.databaseBadges.jdbc') }}</span>
                  </div>
                </SelectItem>
                <SelectItem :value="DatabaseType.GBASE8A">
                  <div class="flex gap-2 items-center">
                    <img :src="getDatabaseIcon(DatabaseType.GBASE8A)" alt="" class="h-5 w-5 object-contain">
                    {{ t('components.serverForm.databaseTypes.gbase8a') }}
                    <span class="text-xs text-muted-foreground ml-auto">{{ t('components.serverForm.databaseBadges.jdbc') }}</span>
                  </div>
                </SelectItem>
                <SelectItem :value="DatabaseType.XUGUDB">
                  <div class="flex gap-2 items-center">
                    <img :src="getDatabaseIcon(DatabaseType.XUGUDB)" alt="" class="h-5 w-5 object-contain">
                    {{ t('components.serverForm.databaseTypes.xugudb') }}
                    <span class="text-xs text-muted-foreground ml-auto">{{ t('components.serverForm.databaseBadges.jdbc') }}</span>
                  </div>
                </SelectItem>
                <SelectItem :value="DatabaseType.DERBY">
                  <div class="flex gap-2 items-center">
                    <img :src="getDatabaseIcon(DatabaseType.DERBY)" alt="" class="h-5 w-5 object-contain">
                    {{ t('components.serverForm.databaseTypes.derby') }}
                    <span class="text-xs text-muted-foreground ml-auto">{{ t('components.serverForm.databaseBadges.jdbc') }}</span>
                  </div>
                </SelectItem>
                <SelectItem :value="DatabaseType.HIVE">
                  <div class="flex gap-2 items-center">
                    <img :src="getDatabaseIcon(DatabaseType.HIVE)" alt="" class="h-5 w-5 object-contain">
                    {{ t('components.serverForm.databaseTypes.hive') }}
                    <span class="text-xs text-muted-foreground ml-auto">{{ t('components.serverForm.databaseBadges.jdbc') }}</span>
                  </div>
                </SelectItem>
                <SelectItem :value="DatabaseType.DATABRICKS">
                  <div class="flex gap-2 items-center">
                    <img :src="getDatabaseIcon(DatabaseType.DATABRICKS)" alt="" class="h-5 w-5 object-contain">
                    {{ t('components.serverForm.databaseTypes.databricks') }}
                    <span class="text-xs text-muted-foreground ml-auto">{{ t('components.serverForm.databaseBadges.jdbc') }}</span>
                  </div>
                </SelectItem>
                <SelectItem :value="DatabaseType.HANA">
                  <div class="flex gap-2 items-center">
                    <img :src="getDatabaseIcon(DatabaseType.HANA)" alt="" class="h-5 w-5 object-contain">
                    {{ t('components.serverForm.databaseTypes.hana') }}
                    <span class="text-xs text-muted-foreground ml-auto">{{ t('components.serverForm.databaseBadges.jdbc') }}</span>
                  </div>
                </SelectItem>
                <SelectItem :value="DatabaseType.TERADATA">
                  <div class="flex gap-2 items-center">
                    <img :src="getDatabaseIcon(DatabaseType.TERADATA)" alt="" class="h-5 w-5 object-contain">
                    {{ t('components.serverForm.databaseTypes.teradata') }}
                    <span class="text-xs text-muted-foreground ml-auto">{{ t('components.serverForm.databaseBadges.jdbc') }}</span>
                  </div>
                </SelectItem>
                <SelectItem :value="DatabaseType.VERTICA">
                  <div class="flex gap-2 items-center">
                    <img :src="getDatabaseIcon(DatabaseType.VERTICA)" alt="" class="h-5 w-5 object-contain">
                    {{ t('components.serverForm.databaseTypes.vertica') }}
                    <span class="text-xs text-muted-foreground ml-auto">{{ t('components.serverForm.databaseBadges.jdbc') }}</span>
                  </div>
                </SelectItem>
                <SelectItem :value="DatabaseType.EXASOL">
                  <div class="flex gap-2 items-center">
                    <img :src="getDatabaseIcon(DatabaseType.EXASOL)" alt="" class="h-5 w-5 object-contain">
                    {{ t('components.serverForm.databaseTypes.exasol') }}
                    <span class="text-xs text-muted-foreground ml-auto">{{ t('components.serverForm.databaseBadges.jdbc') }}</span>
                  </div>
                </SelectItem>
                <SelectItem :value="DatabaseType.BIGQUERY">
                  <div class="flex gap-2 items-center">
                    <img :src="getDatabaseIcon(DatabaseType.BIGQUERY)" alt="" class="h-5 w-5 object-contain">
                    {{ t('components.serverForm.databaseTypes.bigquery') }}
                    <span class="text-xs text-muted-foreground ml-auto">{{ t('components.serverForm.databaseBadges.jdbc') }}</span>
                  </div>
                </SelectItem>
                <SelectItem :value="DatabaseType.INFORMIX">
                  <div class="flex gap-2 items-center">
                    <img :src="getDatabaseIcon(DatabaseType.INFORMIX)" alt="" class="h-5 w-5 object-contain">
                    {{ t('components.serverForm.databaseTypes.informix') }}
                    <span class="text-xs text-muted-foreground ml-auto">{{ t('components.serverForm.databaseBadges.jdbc') }}</span>
                  </div>
                </SelectItem>
                <SelectItem :value="DatabaseType.KYLIN">
                  <div class="flex gap-2 items-center">
                    <img :src="getDatabaseIcon(DatabaseType.KYLIN)" alt="" class="h-5 w-5 object-contain">
                    {{ t('components.serverForm.databaseTypes.kylin') }}
                    <span class="text-xs text-muted-foreground ml-auto">{{ t('components.serverForm.databaseBadges.jdbc') }}</span>
                  </div>
                </SelectItem>
                <SelectItem :value="DatabaseType.CASSANDRA">
                  <div class="flex gap-2 items-center">
                    <img :src="getDatabaseIcon(DatabaseType.CASSANDRA)" alt="" class="h-5 w-5 object-contain">
                    {{ t('components.serverForm.databaseTypes.cassandra') }}
                    <span class="text-xs text-muted-foreground ml-auto">{{ t('components.serverForm.databaseBadges.jdbc') }}</span>
                  </div>
                </SelectItem>
                <SelectItem :value="DatabaseType.IRIS">
                  <div class="flex gap-2 items-center">
                    <img :src="getDatabaseIcon(DatabaseType.IRIS)" alt="" class="h-5 w-5 object-contain">
                    {{ t('components.serverForm.databaseTypes.iris') }}
                    <span class="text-xs text-muted-foreground ml-auto">{{ t('components.serverForm.databaseBadges.jdbc') }}</span>
                  </div>
                </SelectItem>
                <SelectItem :value="DatabaseType.ACCESS">
                  <div class="flex gap-2 items-center">
                    <img :src="getDatabaseIcon(DatabaseType.ACCESS)" alt="" class="h-5 w-5 object-contain">
                    {{ t('components.serverForm.databaseTypes.access') }}
                    <span class="text-xs text-muted-foreground ml-auto">{{ t('components.serverForm.databaseBadges.jdbc') }}</span>
                  </div>
                </SelectItem>
              </SelectGroup>
            </SelectContent>
          </Select>
        </div>

        <!-- SQLite-specific fields -->
        <div v-if="isFileBased" class="space-y-4">
          <!-- SQLite Mode Tabs -->
          <Tabs
            v-model="sqliteTab"
          >
            <TabsList class="grid grid-cols-2 w-full">
              <TabsTrigger value="file">
                {{ t('components.serverForm.sqlite.modes.file') }}
              </TabsTrigger>
              <TabsTrigger value="in-memory">
                {{ t('components.serverForm.sqlite.modes.inMemory') }}
              </TabsTrigger>
            </TabsList>

            <!-- File Mode -->
            <TabsContent value="file" class="space-y-4">
              <!-- Database File Path -->
              <div class="space-y-2">
                <Label for="sqlite-path">{{ t('components.serverForm.labels.databaseFilePath') }}</Label>
                <div class="flex gap-2 items-center">
                  <Input
                    id="sqlite-path"
                    v-model="formData.host"
                    :placeholder="t('components.serverForm.placeholders.filePath')"
                    :class="{ 'border-destructive': formErrors.host }"
                    readonly
                    class="flex-1"
                  />
                  <Button
                    variant="outline"
                    size="sm"
                    @click="selectDatabaseFile"
                  >
                    {{ t('common.buttons.browse') }}
                  </Button>
                </div>
                <p v-if="formErrors.host" class="text-sm text-destructive">
                  {{ formErrors.host }}
                </p>
              </div>

              <!-- Recent Databases -->
              <div v-if="recentDatabases.length > 0" class="space-y-2">
                <Label>{{ t('components.serverForm.labels.recentDatabases') }}</Label>
                <div class="p-2 border rounded-md max-h-32 overflow-y-auto space-y-1">
                  <div
                    v-for="db in recentDatabases"
                    :key="db.path"
                    class="text-sm p-1 rounded flex gap-2 cursor-pointer items-center hover:bg-muted"
                    @click="selectRecentDatabase(db.path)"
                  >
                    <svg
                      class="text-muted-foreground h-4 w-4"
                      xmlns="http://www.w3.org/2000/svg"
                      viewBox="0 0 24 24"
                      fill="none"
                      stroke="currentColor"
                      stroke-width="2"
                      stroke-linecap="round"
                      stroke-linejoin="round"
                    >
                      <path d="M14.5 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V7.5L14.5 2z" />
                      <polyline points="14 2 14 8 20 8" />
                    </svg>
                    <span class="flex-1 truncate">{{ db.path }}</span>
                    <span class="text-xs text-muted-foreground">{{ formatTimeAgo(db.timestamp) }}</span>
                  </div>
                </div>
              </div>

              <!-- Empty state for recent databases -->
              <div v-if="recentDatabases.length === 0" class="text-sm text-muted-foreground">
                {{ t('components.serverForm.sqlite.recentEmpty') }}
              </div>
            </TabsContent>

            <!-- In-Memory Mode -->
            <TabsContent value="in-memory">
              <p class="text-sm text-muted-foreground">
                {{ t('components.serverForm.sqlite.inMemoryHint') }}
              </p>
            </TabsContent>
          </Tabs>
        </div>

        <!-- Host field for non-SQLite databases -->
        <div v-if="!isFileBased" class="space-y-2">
          <Label for="host">{{ t('components.serverForm.labels.host') }}</Label>
          <Input
            id="host"
            v-model="formData.host"
            :placeholder="t('components.serverForm.placeholders.host')"
            :class="{ 'border-destructive': formErrors.host }"
          />
          <p v-if="formErrors.host" class="text-sm text-destructive">
            {{ formErrors.host }}
          </p>
        </div>

        <!-- Port and Database (not for SQLite) -->
        <div v-if="!isFileBased" class="gap-4 grid grid-cols-2">
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
        <div v-if="!isFileBased" class="gap-4 grid grid-cols-2">
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
          v-if="!isFileBased"
          v-model="formData.ssl"
          :db-type="formData.type"
          :errors="formErrors"
        />

        <!-- Advanced Configuration: SSH Tunnel -->
        <div v-if="!isFileBased" class="pt-2">
          <Button
            variant="ghost"
            size="sm"
            class="text-muted-foreground w-full justify-between"
            @click="showAdvanced = !showAdvanced"
          >
            Advanced Configuration
            <svg
              class="h-4 w-4 transition-transform" :class="[showAdvanced ? 'rotate-180' : '']"
              xmlns="http://www.w3.org/2000/svg"
              viewBox="0 0 24 24"
              fill="none"
              stroke="currentColor"
              stroke-width="2"
              stroke-linecap="round"
              stroke-linejoin="round"
            >
              <polyline points="6 9 12 15 18 9" />
            </svg>
          </Button>

          <div v-if="showAdvanced" class="mt-3 p-4 border rounded-md space-y-4">
            <div class="flex gap-2 items-center">
              <input
                id="use-ssh"
                type="checkbox"
                class="border-gray-300 rounded h-4 w-4"
                :checked="formData.sshTunnel?.enabled ?? false"
                @change="(e: Event) => toggleSsh((e.target as HTMLInputElement).checked)"
              >
              <Label for="use-ssh">Use SSH Tunnel</Label>
            </div>

            <template v-if="formData.sshTunnel?.enabled">
              <div class="space-y-2">
                <Label for="ssh-host">SSH Host</Label>
                <Input id="ssh-host" v-model="formData.sshTunnel.host" placeholder="ssh.example.com" />
              </div>

              <div class="gap-4 grid grid-cols-2">
                <div class="space-y-2">
                  <Label for="ssh-port">SSH Port</Label>
                  <Input id="ssh-port" v-model.number="formData.sshTunnel.port" type="number" placeholder="22" />
                </div>
                <div class="space-y-2">
                  <Label for="ssh-user">Username</Label>
                  <Input id="ssh-user" v-model="formData.sshTunnel.username" placeholder="username" autocomplete="off" />
                </div>
              </div>

              <div class="space-y-2">
                <Label for="ssh-auth">Auth Method</Label>
                <Select v-model="formData.sshTunnel.authMethod">
                  <SelectTrigger id="ssh-auth">
                    <SelectValue />
                  </SelectTrigger>
                  <SelectContent>
                    <SelectGroup>
                      <SelectItem value="password">
                        Password
                      </SelectItem>
                      <SelectItem value="privateKey">
                        Private Key
                      </SelectItem>
                      <SelectItem value="agent">
                        SSH Agent
                      </SelectItem>
                    </SelectGroup>
                  </SelectContent>
                </Select>
              </div>

              <template v-if="formData.sshTunnel.authMethod === 'password'">
                <div class="space-y-2">
                  <Label for="ssh-password">SSH Password</Label>
                  <Input id="ssh-password" v-model="formData.sshTunnel.password" type="password" placeholder="SSH password" autocomplete="off" />
                </div>
              </template>

              <template v-if="formData.sshTunnel.authMethod === 'privateKey'">
                <div class="space-y-2">
                  <Label for="ssh-key">Private Key Path</Label>
                  <Input id="ssh-key" v-model="formData.sshTunnel.privateKey" placeholder="/path/to/id_rsa" />
                </div>
                <div class="space-y-2">
                  <Label for="ssh-passphrase">Passphrase (optional)</Label>
                  <Input id="ssh-passphrase" v-model="formData.sshTunnel.privateKeyPassphrase" type="password" placeholder="Key passphrase" autocomplete="off" />
                </div>
              </template>

              <template v-if="formData.sshTunnel.authMethod === 'agent'">
                <p class="text-sm text-muted-foreground">
                  Using system SSH agent - no additional configuration needed.
                </p>
              </template>
            </template>
          </div>
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
