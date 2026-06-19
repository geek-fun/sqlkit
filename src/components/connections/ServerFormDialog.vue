<script setup lang="ts">
import type { OracleConnectionOptions, ServerConnection } from '@/store'
import { invoke } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'
import { open as openDialog } from '@tauri-apps/plugin-dialog'
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
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select'
import { Tabs, TabsContent, TabsList, TabsTrigger } from '@/components/ui/tabs'
import { useDatabaseIcon } from '@/composables/useDatabaseIcon'
import { toast } from '@/composables/useNotifications'
import { jdbcApi } from '@/datasources/jdbcApi'
import { buildOracleOptions, buildTransportLayers, DatabaseType, dbTypeToBackend, resolveDatabase } from '@/store'
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
  [DatabaseType.FIREBIRD]: 3050,
  [DatabaseType.RQLITE]: 4001,
  [DatabaseType.TURSO]: 443,
  [DatabaseType.TDENGINE]: 6030,
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
const isPickingFile = ref(false)

// Step tracking for setup progress
// Each step has its own status; clicking retry re-runs that individual step.
type StepDef = {
  id: string
  label: string
  status: 'pending' | 'running' | 'done' | 'error'
  progress?: { downloaded: number, total: number }
  error?: string
}
const setupSteps = ref<StepDef[]>([
  { id: 'jre', label: 'Java Runtime', status: 'pending' },
  { id: 'bridge', label: 'Bridge JAR', status: 'pending' },
  { id: 'driver', label: 'JDBC Driver', status: 'pending' },
])

// Listen for progress events from any download command
const progressUnlisten = ref<(() => void) | null>(null)

// TNS alias handling: strip level suffixes for display, combine for connection
const LEVEL_SUFFIXES = ['_low', '_medium', '_high', '_tp', '_tpurgent']
function stripLevelSuffix(name: string): string {
  for (const suffix of LEVEL_SUFFIXES) {
    if (name.endsWith(suffix)) return name.slice(0, -suffix.length)
  }
  return name
}
const tnsFullAlias = computed(() => {
  if (!oracleTnsAlias.value || oracleMethod.value === 'basic') return undefined
  if (oracleMethod.value === 'cloud_wallet' && oracleServiceLevel.value) {
    return `${oracleTnsAlias.value}_${oracleServiceLevel.value}`
  }
  return oracleTnsAlias.value
})

async function startProgressListener() {
  // Clean up previous listener
  if (progressUnlisten.value) progressUnlisten.value()
  const unlisten = await listen<{ step: string, downloaded: number, total: number, message?: string }>('connection-progress', (event) => {
    const stepMap: Record<string, string> = { jre_download: 'jre', bridge_jar: 'bridge' }
    const stepId = stepMap[event.payload.step]
    const step = setupSteps.value.find(s => s.id === stepId)
    if (step && event.payload.step !== 'retry') {
      step.status = 'running'
      step.progress = { downloaded: event.payload.downloaded, total: event.payload.total }
    }
  })
  progressUnlisten.value = unlisten
}

// Run a single setup step (JRE or Bridge) with its own invoke
async function runStep(step: StepDef): Promise<boolean> {
  step.status = 'running'
  step.progress = undefined
  step.error = undefined
  try {
    if (step.id === 'jre') {
      await invoke('download_jre')
    }
    else if (step.id === 'bridge') {
      await invoke('download_bridge_jar')
    }
    step.status = 'done'
    return true
  }
  catch (e) {
    step.status = 'error'
    step.error = e instanceof Error ? e.message : String(e)
    return false
  }
}

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
      // Sync Oracle options when editing
      if (props.connection.type === DatabaseType.ORACLE) {
        syncOracleFromFormData()
      }
    }
    else {
      formData.value = { ...defaultConnection }
      sqliteTab.value = 'file'
      savedFilePath.value = ''
      resetOracleOptions()
    }
    testStatus.value = 'idle'
    testError.value = ''
    formErrors.value = {}
    setupSteps.value.forEach(s => { s.status = 'pending'; s.progress = undefined; s.error = undefined })
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
  if (type === DatabaseType.ORACLE) {
    resetOracleOptions()
    formData.value.host = formData.value.host || 'localhost'
    formData.value.port = formData.value.port || 1521
    formData.value.oracleOptions = buildOracleOptionsFromRefs()
  }
  else {
    formData.value.oracleOptions = undefined
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

// ── Oracle-specific state ──
const isOracle = computed(() => formData.value.type === DatabaseType.ORACLE)
const oracleMethod = ref<'basic' | 'tns' | 'cloud_wallet'>('basic')
const oracleSidOrService = ref<'sid' | 'service_name'>('service_name')
const oracleRole = ref<'NORMAL' | 'SYSDBA' | 'SYSOPER'>('NORMAL')
const oracleTnsAdminDir = ref('')
const oracleTnsAlias = ref('')
const oracleWalletPassword = ref('')
const oracleServiceLevel = ref<'low' | 'medium' | 'high' | 'tp' | 'tpurgent'>('medium')
const tnsAliases = ref<string[]>([])
const loadingAliases = ref(false)

// When Oracle is TNS or Cloud Wallet, standard host/port/database fields are hidden
const showsStandardHostFields = computed(() =>
  !isOracle.value
  || oracleMethod.value === 'basic',
)

const databaseLabel = computed(() => {
  if (isOracle.value && oracleMethod.value === 'basic') {
    return oracleSidOrService.value === 'sid'
      ? t('components.serverForm.oracle.sid')
      : t('components.serverForm.oracle.serviceName')
  }
  return t('components.serverForm.labels.database')
})

// Reset Oracle options when toggling method
function resetOracleOptions() {
  oracleMethod.value = 'basic'
  oracleSidOrService.value = 'service_name'
  oracleRole.value = 'NORMAL'
  oracleTnsAdminDir.value = ''
  oracleTnsAlias.value = ''
  oracleWalletPassword.value = ''
  oracleServiceLevel.value = 'medium'
  tnsAliases.value = []
}

// Sync oracle options from formData when editing existing connection
function syncOracleFromFormData() {
  if (formData.value.oracleOptions) {
    const o = formData.value.oracleOptions
    oracleMethod.value = o.connectionMethod
    oracleSidOrService.value = o.sidOrService ?? 'service_name'
    oracleRole.value = o.role ?? 'NORMAL'
    oracleTnsAdminDir.value = o.tnsAdminDir ?? ''
    oracleTnsAlias.value = o.tnsAlias ?? ''
    oracleWalletPassword.value = o.walletPassword ?? ''
    oracleServiceLevel.value = o.serviceLevel ?? 'medium'
  }
}

// Build oracleOptions from reactive refs
function buildOracleOptionsFromRefs(): OracleConnectionOptions | undefined {
  if (!isOracle.value) return undefined
  const hasTnsAlias = oracleMethod.value !== 'basic' && !!tnsFullAlias.value
  return {
    connectionMethod: oracleMethod.value,
    sidOrService: oracleMethod.value === 'basic' ? oracleSidOrService.value : undefined,
    role: oracleRole.value,
    tnsAdminDir: oracleMethod.value !== 'basic' ? oracleTnsAdminDir.value || undefined : undefined,
    tnsAlias: hasTnsAlias ? tnsFullAlias.value : undefined,
    walletPassword: oracleMethod.value === 'cloud_wallet' ? oracleWalletPassword.value || undefined : undefined,
    serviceLevel: oracleMethod.value === 'cloud_wallet' ? oracleServiceLevel.value : undefined,
  }
}

function onOracleMethodChange(method: string | number) {
  oracleMethod.value = method as 'basic' | 'tns' | 'cloud_wallet'
  oracleTnsAlias.value = ''
  if (method !== 'basic') {
    formData.value.host = ''
    formData.value.port = 0
  }
  else {
    formData.value.host = formData.value.host || 'localhost'
    formData.value.port = formData.value.port || 1521
  }
}

function onServiceLevelChange(val: string) {
  oracleServiceLevel.value = val as 'low' | 'medium' | 'high' | 'tp' | 'tpurgent'
}

function onTnsAliasChange(val: string) {
  oracleTnsAlias.value = val
}

// Sync formData.oracleOptions whenever relevant refs change
watch([oracleMethod, oracleSidOrService, oracleRole, oracleTnsAdminDir, oracleTnsAlias, oracleWalletPassword, oracleServiceLevel], () => {
  formData.value.oracleOptions = buildOracleOptionsFromRefs()
})

// Load TNS aliases when directory changes (TNS or Cloud Wallet)
async function loadTnsAliases(dir: string) {
  if (!dir) {
    tnsAliases.value = []
    return
  }
  loadingAliases.value = true
  try {
    const raw = await jdbcApi.listTnsAliases(dir)
    // Strip level suffixes and deduplicate to show clean base names
    const baseNames = new Set(raw.map(stripLevelSuffix))
    tnsAliases.value = [...baseNames].sort()
    if (tnsAliases.value.length === 0 && oracleTnsAlias.value) {
      oracleTnsAlias.value = ''
    }
  }
  catch (e) {
    console.error('Failed to list TNS aliases:', e)
    tnsAliases.value = []
    if (oracleTnsAlias.value) oracleTnsAlias.value = ''
  }
  finally {
    loadingAliases.value = false
  }
}

async function browseDirectory(target: 'tns' | 'wallet') {
  isPickingFile.value = true
  try {
    const selected = await openDialog({ directory: true, multiple: false })
    if (typeof selected === 'string') {
      if (target === 'tns') {
        oracleTnsAdminDir.value = selected
        await loadTnsAliases(selected)
      }
      else {
        // For cloud wallet, tnsAdminDir is the wallet dir
        oracleTnsAdminDir.value = selected
        oracleTnsAlias.value = ''
        await loadTnsAliases(selected)
      }
    }
  }
  catch (error) {
    toast.error(t('components.serverForm.errors.filePickerFailed'), {
      description: error instanceof Error ? error.message : String(error),
    })
  }
  finally {
    isPickingFile.value = false
  }
}

// SQLite file picker function - handles both open existing and create new
async function selectDatabaseFile() {
  isPickingFile.value = true
  try {
    const selected = await openDialog({
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
  finally {
    isPickingFile.value = false
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
  else if (isOracle.value && !showsStandardHostFields.value) {
    // Oracle TNS / Cloud Wallet: TNS admin dir and alias required
    if (!oracleTnsAdminDir.value.trim()) {
      errors.tnsAdminDir = t('components.serverForm.errors.hostRequired')
    }
    if (!oracleTnsAlias.value.trim()) {
      errors.tnsAlias = t('components.serverForm.errors.hostRequired')
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
  if (!validateForm()) return

  testStatus.value = 'testing'
  testError.value = ''

  // Reset steps to pending — checks will promote to done or running
  setupSteps.value.forEach(s => { s.status = 'pending'; s.progress = undefined; s.error = undefined })

  await startProgressListener()

  try {
    // Run all three downloads in parallel — none depend on each other
    const jreStep = setupSteps.value.find(s => s.id === 'jre')!
    const bridgeStep = setupSteps.value.find(s => s.id === 'bridge')!
    const driverStep = setupSteps.value.find(s => s.id === 'driver')!

    const dbType = mapDatabaseTypeToBackend(formData.value.type)

    const results = await Promise.allSettled([
      // JRE download (skips if already installed)
      invoke('check_jre_status').then(async (status: any) => {
        if (!status.installed) {
          const ok = await runStep(jreStep)
          if (!ok) throw new Error('JRE download failed')
        }
        else { jreStep.status = 'done' }
      }),
      // Bridge download (skips if already installed)
      invoke('check_bridge_status').then(async (status: any) => {
        if (!status.installed) {
          const ok = await runStep(bridgeStep)
          if (!ok) throw new Error('Bridge download failed')
        }
        else { bridgeStep.status = 'done' }
      }),
      // JDBC driver download directly from Maven Central (no Java needed)
      (async () => {
        try {
          await invoke('download_jdbc_driver_direct', { dbType })
          driverStep.status = 'done'
        }
        catch {
          driverStep.status = 'done' // Non-fatal
        }
      })(),
    ])

    // If JRE or Bridge failed, stop (keep steps visible with error state)
    if (results[0].status === 'rejected' || results[1].status === 'rejected') {
      const firstError = setupSteps.value.find(s => s.status === 'error')
      if (firstError?.error) testError.value = firstError.error
      return
    }

    // Step 4: Test the actual connection
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
      oracle_options: buildOracleOptions(formData.value.oracleOptions),
    }

    // Small yield to let Vue render the loading state before the blocking invoke
    await new Promise(r => setTimeout(r, 50))

    const result = await invoke<{ is_connected: boolean, server_version?: string }>('test_connection', { config })

    if (result.is_connected) {
      driverStep.status = 'done'
      testStatus.value = 'success'
    }
    else {
      driverStep.status = 'error'
      const msg = 'Connection returned not connected'
      driverStep.error = msg
      testError.value = msg
      testStatus.value = 'error'
    }
  }
  catch (error) {
    testStatus.value = 'error'
    const msg = error instanceof Error ? error.message : String(error)
    testError.value = msg
    // If the error is Java-related, also mark JRE as failed so user can retry it
    if (msg.includes('Unable to locate a Java Runtime') || msg.includes('Java not found')) {
      const jreStep = setupSteps.value.find(s => s.id === 'jre')
      if (jreStep) { jreStep.status = 'error'; jreStep.error = 'JRE is missing or broken' }
    }
    const running = setupSteps.value.find(s => s.status === 'running')
    if (running) { running.status = 'error'; running.error = msg }
  }

  // If testStatus is error but testError is empty, collect from first errored step
  if (testStatus.value === 'error' && !testError.value) {
    const firstError = setupSteps.value.find(s => s.status === 'error')
    if (firstError?.error) testError.value = firstError.error
  }
}

/** Retry an individual setup step by its id */
async function retryStep(stepId: string) {
  const step = setupSteps.value.find(s => s.id === stepId)
  if (!step || step.status !== 'error') return

  if (stepId === 'driver') {
    // For driver/connection, re-run full test
    handleTestConnection()
    return
  }

  await startProgressListener()
  step.status = 'running'
  step.error = undefined
  step.progress = undefined

  try {
    if (stepId === 'jre') {
      await invoke('download_jre')
    }
    else if (stepId === 'bridge') {
      await invoke('download_bridge_jar')
    }
    step.status = 'done'

    // If all dependencies are now done, try connecting
    const allReady = setupSteps.value.every(s => s.status === 'done')
    if (allReady && stepId !== 'driver') {
      handleTestConnection()
    }
  }
  catch (e) {
    step.status = 'error'
    step.error = e instanceof Error ? e.message : String(e)
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
    <DialogContent class="sm:max-w-xl" @interact-outside="(e: Event) => { if (isPickingFile) e.preventDefault() }">
      <DialogTitle>
        {{ isEditing ? t('components.serverForm.title.edit') : t('components.serverForm.title.new') }}
      </DialogTitle>
      <DialogDescription>
        {{ isEditing ? t('components.serverForm.description.edit') : t('components.serverForm.description.new') }}
      </DialogDescription>

      <form class="space-y-4" @submit.prevent="handleSave">
        <!-- Connection Name -->
        <div class="space-y-2">
          <Label for="name">{{ t('components.serverForm.labels.connectionName') }}<span class="text-destructive ml-0.5">*</span></Label>
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
                <!-- Sorted by DB-Engines June 2026 rank; 信创 databases grouped together -->
                <!-- Global databases -->
                <SelectItem :value="DatabaseType.ORACLE">
                  <div class="flex gap-2 items-center">
                    <img :src="getDatabaseIcon(DatabaseType.ORACLE)" alt="" class="h-5 w-5 object-contain">
                    {{ t('components.serverForm.databaseTypes.oracle') }}
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
                <SelectItem :value="DatabaseType.POSTGRESQL">
                  <div class="flex gap-2 items-center">
                    <img :src="getDatabaseIcon(DatabaseType.POSTGRESQL)" alt="" class="h-5 w-5 object-contain">
                    {{ t('components.serverForm.databaseTypes.postgresql') }}
                  </div>
                </SelectItem>
                <SelectItem :value="DatabaseType.SNOWFLAKE">
                  <div class="flex gap-2 items-center">
                    <img :src="getDatabaseIcon(DatabaseType.SNOWFLAKE)" alt="" class="h-5 w-5 object-contain">
                    {{ t('components.serverForm.databaseTypes.snowflake') }}
                  </div>
                </SelectItem>
                <SelectItem :value="DatabaseType.DATABRICKS">
                  <div class="flex gap-2 items-center">
                    <img :src="getDatabaseIcon(DatabaseType.DATABRICKS)" alt="" class="h-5 w-5 object-contain">
                    {{ t('components.serverForm.databaseTypes.databricks') }}
                  </div>
                </SelectItem>
                <SelectItem :value="DatabaseType.DB2">
                  <div class="flex gap-2 items-center">
                    <img :src="getDatabaseIcon(DatabaseType.DB2)" alt="" class="h-5 w-5 object-contain">
                    {{ t('components.serverForm.databaseTypes.db2') }}
                  </div>
                </SelectItem>
                <SelectItem :value="DatabaseType.CASSANDRA">
                  <div class="flex gap-2 items-center">
                    <img :src="getDatabaseIcon(DatabaseType.CASSANDRA)" alt="" class="h-5 w-5 object-contain">
                    {{ t('components.serverForm.databaseTypes.cassandra') }}
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
                <SelectItem :value="DatabaseType.HIVE">
                  <div class="flex gap-2 items-center">
                    <img :src="getDatabaseIcon(DatabaseType.HIVE)" alt="" class="h-5 w-5 object-contain">
                    {{ t('components.serverForm.databaseTypes.hive') }}
                  </div>
                </SelectItem>
                <SelectItem :value="DatabaseType.ACCESS">
                  <div class="flex gap-2 items-center">
                    <img :src="getDatabaseIcon(DatabaseType.ACCESS)" alt="" class="h-5 w-5 object-contain">
                    {{ t('components.serverForm.databaseTypes.access') }}
                  </div>
                </SelectItem>
                <SelectItem :value="DatabaseType.BIGQUERY">
                  <div class="flex gap-2 items-center">
                    <img :src="getDatabaseIcon(DatabaseType.BIGQUERY)" alt="" class="h-5 w-5 object-contain">
                    {{ t('components.serverForm.databaseTypes.bigquery') }}
                  </div>
                </SelectItem>
                <SelectItem :value="DatabaseType.HANA">
                  <div class="flex gap-2 items-center">
                    <img :src="getDatabaseIcon(DatabaseType.HANA)" alt="" class="h-5 w-5 object-contain">
                    {{ t('components.serverForm.databaseTypes.hana') }}
                  </div>
                </SelectItem>
                <SelectItem :value="DatabaseType.TERADATA">
                  <div class="flex gap-2 items-center">
                    <img :src="getDatabaseIcon(DatabaseType.TERADATA)" alt="" class="h-5 w-5 object-contain">
                    {{ t('components.serverForm.databaseTypes.teradata') }}
                  </div>
                </SelectItem>
                <SelectItem :value="DatabaseType.CLICKHOUSE">
                  <div class="flex gap-2 items-center">
                    <img :src="getDatabaseIcon(DatabaseType.CLICKHOUSE)" alt="" class="h-5 w-5 object-contain">
                    {{ t('components.serverForm.databaseTypes.clickhouse') }}
                  </div>
                </SelectItem>
                <SelectItem :value="DatabaseType.FIREBIRD">
                  <div class="flex gap-2 items-center">
                    <img :src="getDatabaseIcon(DatabaseType.FIREBIRD)" alt="" class="h-5 w-5 object-contain">
                    {{ t('components.serverForm.databaseTypes.firebird') }}
                  </div>
                </SelectItem>
                <SelectItem :value="DatabaseType.REDSHIFT">
                  <div class="flex gap-2 items-center">
                    <img :src="getDatabaseIcon(DatabaseType.REDSHIFT)" alt="" class="h-5 w-5 object-contain">
                    {{ t('components.serverForm.databaseTypes.redshift') }}
                  </div>
                </SelectItem>
                <SelectItem :value="DatabaseType.INFORMIX">
                  <div class="flex gap-2 items-center">
                    <img :src="getDatabaseIcon(DatabaseType.INFORMIX)" alt="" class="h-5 w-5 object-contain">
                    {{ t('components.serverForm.databaseTypes.informix') }}
                  </div>
                </SelectItem>
                <SelectItem :value="DatabaseType.DUCKDB">
                  <div class="flex gap-2 items-center">
                    <img :src="getDatabaseIcon(DatabaseType.DUCKDB)" alt="" class="h-5 w-5 object-contain">
                    {{ t('components.serverForm.databaseTypes.duckdb') }}
                  </div>
                </SelectItem>
                <SelectItem :value="DatabaseType.VERTICA">
                  <div class="flex gap-2 items-center">
                    <img :src="getDatabaseIcon(DatabaseType.VERTICA)" alt="" class="h-5 w-5 object-contain">
                    {{ t('components.serverForm.databaseTypes.vertica') }}
                  </div>
                </SelectItem>
                <SelectItem :value="DatabaseType.H2">
                  <div class="flex gap-2 items-center">
                    <img :src="getDatabaseIcon(DatabaseType.H2)" alt="" class="h-5 w-5 object-contain">
                    {{ t('components.serverForm.databaseTypes.h2') }}
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
                <SelectItem :value="DatabaseType.TIMESCALEDB">
                  <div class="flex gap-2 items-center">
                    <img :src="getDatabaseIcon(DatabaseType.TIMESCALEDB)" alt="" class="h-5 w-5 object-contain">
                    {{ t('components.serverForm.databaseTypes.timescaledb') }}
                  </div>
                </SelectItem>
                <SelectItem :value="DatabaseType.COCKROACHDB">
                  <div class="flex gap-2 items-center">
                    <img :src="getDatabaseIcon(DatabaseType.COCKROACHDB)" alt="" class="h-5 w-5 object-contain">
                    {{ t('components.serverForm.databaseTypes.cockroachdb') }}
                  </div>
                </SelectItem>
                <SelectItem :value="DatabaseType.QUESTDB">
                  <div class="flex gap-2 items-center">
                    <img :src="getDatabaseIcon(DatabaseType.QUESTDB)" alt="" class="h-5 w-5 object-contain">
                    {{ t('components.serverForm.databaseTypes.questdb') }}
                  </div>
                </SelectItem>
                <SelectItem :value="DatabaseType.DERBY">
                  <div class="flex gap-2 items-center">
                    <img :src="getDatabaseIcon(DatabaseType.DERBY)" alt="" class="h-5 w-5 object-contain">
                    {{ t('components.serverForm.databaseTypes.derby') }}
                  </div>
                </SelectItem>
                <SelectItem :value="DatabaseType.IRIS">
                  <div class="flex gap-2 items-center">
                    <img :src="getDatabaseIcon(DatabaseType.IRIS)" alt="" class="h-5 w-5 object-contain">
                    {{ t('components.serverForm.databaseTypes.iris') }}
                  </div>
                </SelectItem>
                <SelectItem :value="DatabaseType.YUGABYTEDB">
                  <div class="flex gap-2 items-center">
                    <img :src="getDatabaseIcon(DatabaseType.YUGABYTEDB)" alt="" class="h-5 w-5 object-contain">
                    {{ t('components.serverForm.databaseTypes.yugabytedb') }}
                  </div>
                </SelectItem>
                <SelectItem :value="DatabaseType.EXASOL">
                  <div class="flex gap-2 items-center">
                    <img :src="getDatabaseIcon(DatabaseType.EXASOL)" alt="" class="h-5 w-5 object-contain">
                    {{ t('components.serverForm.databaseTypes.exasol') }}
                  </div>
                </SelectItem>
                <SelectItem :value="DatabaseType.KYLIN">
                  <div class="flex gap-2 items-center">
                    <img :src="getDatabaseIcon(DatabaseType.KYLIN)" alt="" class="h-5 w-5 object-contain">
                    {{ t('components.serverForm.databaseTypes.kylin') }}
                  </div>
                </SelectItem>
                <SelectItem :value="DatabaseType.MANTICORESEARCH">
                  <div class="flex gap-2 items-center">
                    <img :src="getDatabaseIcon(DatabaseType.MANTICORESEARCH)" alt="" class="h-5 w-5 object-contain">
                    {{ t('components.serverForm.databaseTypes.manticore') }}
                  </div>
                </SelectItem>
                <SelectItem :value="DatabaseType.RQLITE">
                  <div class="flex gap-2 items-center">
                    <img :src="getDatabaseIcon(DatabaseType.RQLITE)" alt="" class="h-5 w-5 object-contain">
                    {{ t('components.serverForm.databaseTypes.rqlite') }}
                  </div>
                </SelectItem>
                <SelectItem :value="DatabaseType.TURSO">
                  <div class="flex gap-2 items-center">
                    <img :src="getDatabaseIcon(DatabaseType.TURSO)" alt="" class="h-5 w-5 object-contain">
                    {{ t('components.serverForm.databaseTypes.turso') }}
                  </div>
                </SelectItem>
                <!-- 信创 databases (grouped) -->
                <SelectItem :value="DatabaseType.TIDB">
                  <div class="flex gap-2 items-center">
                    <img :src="getDatabaseIcon(DatabaseType.TIDB)" alt="" class="h-5 w-5 object-contain">
                    {{ t('components.serverForm.databaseTypes.tidb') }}
                  </div>
                </SelectItem>
                <SelectItem :value="DatabaseType.POLARDB">
                  <div class="flex gap-2 items-center">
                    <img :src="getDatabaseIcon(DatabaseType.POLARDB)" alt="" class="h-5 w-5 object-contain">
                    {{ t('components.serverForm.databaseTypes.polardb') }}
                  </div>
                </SelectItem>
                <SelectItem :value="DatabaseType.TDENGINE">
                  <div class="flex gap-2 items-center">
                    <img :src="getDatabaseIcon(DatabaseType.TDENGINE)" alt="" class="h-5 w-5 object-contain">
                    {{ t('components.serverForm.databaseTypes.tdengine') }}
                  </div>
                </SelectItem>
                <SelectItem :value="DatabaseType.OCEANBASE">
                  <div class="flex gap-2 items-center">
                    <img :src="getDatabaseIcon(DatabaseType.OCEANBASE)" alt="" class="h-5 w-5 object-contain">
                    {{ t('components.serverForm.databaseTypes.oceanbase') }}
                  </div>
                </SelectItem>
                <SelectItem :value="DatabaseType.GBASE8C">
                  <div class="flex gap-2 items-center">
                    <img :src="getDatabaseIcon(DatabaseType.GBASE8C)" alt="" class="h-5 w-5 object-contain">
                    {{ t('components.serverForm.databaseTypes.gbase8c') }}
                  </div>
                </SelectItem>
                <SelectItem :value="DatabaseType.GBASE8A">
                  <div class="flex gap-2 items-center">
                    <img :src="getDatabaseIcon(DatabaseType.GBASE8A)" alt="" class="h-5 w-5 object-contain">
                    {{ t('components.serverForm.databaseTypes.gbase8a') }}
                  </div>
                </SelectItem>
                <SelectItem :value="DatabaseType.STARROCKS">
                  <div class="flex gap-2 items-center">
                    <img :src="getDatabaseIcon(DatabaseType.STARROCKS)" alt="" class="h-5 w-5 object-contain">
                    {{ t('components.serverForm.databaseTypes.starrocks') }}
                  </div>
                </SelectItem>
                <SelectItem :value="DatabaseType.TDSQL">
                  <div class="flex gap-2 items-center">
                    <img :src="getDatabaseIcon(DatabaseType.TDSQL)" alt="" class="h-5 w-5 object-contain">
                    {{ t('components.serverForm.databaseTypes.tdsql') }}
                  </div>
                </SelectItem>
                <SelectItem :value="DatabaseType.OPENGAUSS">
                  <div class="flex gap-2 items-center">
                    <img :src="getDatabaseIcon(DatabaseType.OPENGAUSS)" alt="" class="h-5 w-5 object-contain">
                    {{ t('components.serverForm.databaseTypes.opengauss') }}
                  </div>
                </SelectItem>
                <SelectItem :value="DatabaseType.KINGBASEES">
                  <div class="flex gap-2 items-center">
                    <img :src="getDatabaseIcon(DatabaseType.KINGBASEES)" alt="" class="h-5 w-5 object-contain">
                    {{ t('components.serverForm.databaseTypes.kingbasees') }}
                  </div>
                </SelectItem>
                <SelectItem :value="DatabaseType.DORIS">
                  <div class="flex gap-2 items-center">
                    <img :src="getDatabaseIcon(DatabaseType.DORIS)" alt="" class="h-5 w-5 object-contain">
                    {{ t('components.serverForm.databaseTypes.doris') }}
                  </div>
                </SelectItem>
                <SelectItem :value="DatabaseType.DATABEND">
                  <div class="flex gap-2 items-center">
                    <img :src="getDatabaseIcon(DatabaseType.DATABEND)" alt="" class="h-5 w-5 object-contain">
                    {{ t('components.serverForm.databaseTypes.databend') }}
                  </div>
                </SelectItem>
                <SelectItem :value="DatabaseType.DM8">
                  <div class="flex gap-2 items-center">
                    <img :src="getDatabaseIcon(DatabaseType.DM8)" alt="" class="h-5 w-5 object-contain">
                    {{ t('components.serverForm.databaseTypes.dm8') }}
                  </div>
                </SelectItem>
                <SelectItem :value="DatabaseType.DM8ORACLE">
                  <div class="flex gap-2 items-center">
                    <img :src="getDatabaseIcon(DatabaseType.DM8ORACLE)" alt="" class="h-5 w-5 object-contain">
                    {{ t('components.serverForm.databaseTypes.dm8oracle') }}
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
                <SelectItem :value="DatabaseType.UXDB">
                  <div class="flex gap-2 items-center">
                    <img :src="getDatabaseIcon(DatabaseType.UXDB)" alt="" class="h-5 w-5 object-contain">
                    {{ t('components.serverForm.databaseTypes.uxdb') }}
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
                <SelectItem :value="DatabaseType.SELECTDB">
                  <div class="flex gap-2 items-center">
                    <img :src="getDatabaseIcon(DatabaseType.SELECTDB)" alt="" class="h-5 w-5 object-contain">
                    {{ t('components.serverForm.databaseTypes.selectdb') }}
                  </div>
                </SelectItem>
                <SelectItem :value="DatabaseType.GOLDENDB">
                  <div class="flex gap-2 items-center">
                    <img :src="getDatabaseIcon(DatabaseType.GOLDENDB)" alt="" class="h-5 w-5 object-contain">
                    {{ t('components.serverForm.databaseTypes.goldendb') }}
                  </div>
                </SelectItem>
                <SelectItem :value="DatabaseType.XUGUDB">
                  <div class="flex gap-2 items-center">
                    <img :src="getDatabaseIcon(DatabaseType.XUGUDB)" alt="" class="h-5 w-5 object-contain">
                    {{ t('components.serverForm.databaseTypes.xugudb') }}
                  </div>
                </SelectItem>
              </SelectGroup>
            </SelectContent>
          </Select>
        </div>

        <!-- Host and Port on same row; Database below (non-Oracle, non-file-based) -->
        <div v-if="!isFileBased && !isOracle" class="space-y-4">
          <div class="gap-4 grid grid-cols-4">
            <div class="col-span-3 space-y-2">
              <Label for="host">{{ t('components.serverForm.labels.host') }}<span class="text-destructive ml-0.5">*</span></Label>
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
            <div class="space-y-2">
              <Label for="port">{{ t('components.serverForm.labels.port') }}<span class="text-destructive ml-0.5">*</span></Label>
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

        <!-- Oracle-specific fields -->
        <div v-if="isOracle" class="space-y-4">
          <!-- Connection Method Selector -->
          <div class="space-y-2">
            <Label>{{ t('components.serverForm.oracle.connectionMethod') }}</Label>
            <Tabs :model-value="oracleMethod" @update:model-value="onOracleMethodChange">
              <TabsList class="grid grid-cols-3 w-full">
                <TabsTrigger value="basic">
                  {{ t('components.serverForm.oracle.methods.basic') }}
                </TabsTrigger>
                <TabsTrigger value="tns">
                  {{ t('components.serverForm.oracle.methods.tns') }}
                </TabsTrigger>
                <TabsTrigger value="cloud_wallet">
                  {{ t('components.serverForm.oracle.methods.cloudWallet') }}
                </TabsTrigger>
              </TabsList>

              <!-- Basic Method -->
              <TabsContent value="basic" class="mt-4 space-y-4">
                <div class="gap-4 grid grid-cols-4">
                  <div class="col-span-3 space-y-2">
                    <Label for="oracle-host">{{ t('components.serverForm.labels.host') }}<span class="text-destructive ml-0.5">*</span></Label>
                    <Input
                      id="oracle-host"
                      v-model="formData.host"
                      :placeholder="t('components.serverForm.placeholders.host')"
                      :class="{ 'border-destructive': formErrors.host }"
                    />
                    <p v-if="formErrors.host" class="text-sm text-destructive">
                      {{ formErrors.host }}
                    </p>
                  </div>
                  <div class="space-y-2">
                    <Label for="oracle-port">{{ t('components.serverForm.labels.port') }}<span class="text-destructive ml-0.5">*</span></Label>
                    <Input
                      id="oracle-port"
                      v-model.number="formData.port"
                      type="number"
                      :class="{ 'border-destructive': formErrors.port }"
                    />
                    <p v-if="formErrors.port" class="text-sm text-destructive">
                      {{ formErrors.port }}
                    </p>
                  </div>
                </div>
                <div class="flex flex-wrap gap-4 items-center">
                  <div class="flex gap-2 items-center">
                    <Label class="min-w-fit whitespace-nowrap">{{ t('components.serverForm.oracle.identifyBy') }}</Label>
                    <div class="flex flex-wrap gap-1.5">
                      <button
                        type="button"
                        class="text-xs leading-none px-3 py-1 rounded-full cursor-pointer transition-colors" :class="[
                          oracleSidOrService === 'service_name'
                            ? 'bg-primary/10 text-primary font-medium'
                            : 'text-muted-foreground hover:text-foreground hover:bg-accent/50',
                        ]"
                        @click="oracleSidOrService = 'service_name'"
                      >
                        {{ t('components.serverForm.oracle.serviceName') }}
                      </button>
                      <button
                        type="button"
                        class="text-xs leading-none px-3 py-1 rounded-full cursor-pointer transition-colors" :class="[
                          oracleSidOrService === 'sid'
                            ? 'bg-primary/10 text-primary font-medium'
                            : 'text-muted-foreground hover:text-foreground hover:bg-accent/50',
                        ]"
                        @click="oracleSidOrService = 'sid'"
                      >
                        {{ t('components.serverForm.oracle.sid') }}
                      </button>
                    </div>
                  </div>
                  <div class="flex gap-2 items-center">
                    <Label class="min-w-fit whitespace-nowrap">{{ t('components.serverForm.oracle.role') }}</Label>
                    <div class="flex flex-wrap gap-1.5">
                      <button
                        v-for="r in ([{ v: 'NORMAL', k: 'normal' }, { v: 'SYSDBA', k: 'sysdba' }, { v: 'SYSOPER', k: 'sysoper' }] as const)"
                        :key="r.v"
                        type="button"
                        class="text-xs leading-none px-3 py-1 rounded-full cursor-pointer transition-colors" :class="[
                          oracleRole === r.v
                            ? 'bg-primary/10 text-primary font-medium'
                            : 'text-muted-foreground hover:text-foreground hover:bg-accent/50',
                        ]"
                        @click="oracleRole = r.v"
                      >
                        {{ t(`components.serverForm.oracle.roles.${r.k}`) }}
                      </button>
                    </div>
                  </div>
                </div>
                <div class="space-y-2">
                  <Label for="oracle-svc">{{ databaseLabel }}</Label>
                  <Input
                    id="oracle-svc"
                    v-model="formData.database"
                    :placeholder="databaseLabel"
                  />
                </div>
              </TabsContent>

              <!-- TNS Method -->
              <TabsContent value="tns" class="mt-4 space-y-4">
                <div class="space-y-2">
                  <Label>{{ t('components.serverForm.oracle.tnsAdminDir') }}<span class="text-destructive ml-0.5">*</span></Label>
                  <div class="flex gap-2 items-center">
                    <Input
                      v-model="oracleTnsAdminDir"
                      :placeholder="t('components.serverForm.oracle.tnsAdminDir')"
                      class="flex-1"
                    />
                    <Button type="button" variant="outline" size="sm" @click="browseDirectory('tns')">
                      {{ t('components.serverForm.oracle.browse') }}
                    </Button>
                  </div>
                </div>
                <div class="space-y-2">
                  <Label>{{ t('components.serverForm.oracle.tnsAlias') }}<span class="text-destructive ml-0.5">*</span></Label>
                  <p v-if="loadingAliases" class="text-sm text-muted-foreground">
                    {{ t('components.serverForm.oracle.loadingAliases') }}
                  </p>
                  <p v-else-if="tnsAliases.length === 0 && oracleTnsAdminDir" class="text-sm text-muted-foreground">
                    No aliases found in selected directory
                  </p>
                  <p v-else-if="tnsAliases.length === 0" class="text-sm text-muted-foreground">
                    Select a TNS directory to load aliases
                  </p>
                  <Select v-else :model-value="oracleTnsAlias" @update:model-value="onTnsAliasChange">
                    <SelectTrigger>
                      <SelectValue :placeholder="t('components.serverForm.oracle.tnsAlias')" />
                    </SelectTrigger>
                    <SelectContent>
                      <SelectGroup>
                        <SelectItem v-for="alias in tnsAliases" :key="alias" :value="alias">
                          {{ alias }}
                        </SelectItem>
                      </SelectGroup>
                    </SelectContent>
                  </Select>
                </div>
              </TabsContent>

              <!-- Cloud Wallet Method -->
              <TabsContent value="cloud_wallet" class="mt-4 space-y-4">
                <div class="space-y-2">
                  <Label>{{ t('components.serverForm.oracle.walletDir') }}<span class="text-destructive ml-0.5">*</span></Label>
                  <div class="flex gap-2 items-center">
                    <Input
                      v-model="oracleTnsAdminDir"
                      :placeholder="t('components.serverForm.oracle.walletDir')"
                      class="flex-1"
                    />
                    <Button type="button" variant="outline" size="sm" @click="browseDirectory('wallet')">
                      {{ t('components.serverForm.oracle.browse') }}
                    </Button>
                  </div>
                </div>
                <div class="gap-4 grid grid-cols-2">
                  <div class="space-y-2">
                    <Label>{{ t('components.serverForm.oracle.walletPassword') }}</Label>
                    <Input
                      v-model="oracleWalletPassword"
                      type="password"
                      :placeholder="t('components.serverForm.oracle.walletPassword')"
                    />
                  </div>
                  <div class="space-y-2">
                    <Label>{{ t('components.serverForm.oracle.serviceLevel') }}<span class="text-destructive ml-0.5">*</span></Label>
                    <Select :model-value="oracleServiceLevel" @update:model-value="onServiceLevelChange">
                      <SelectTrigger>
                        <SelectValue />
                      </SelectTrigger>
                      <SelectContent>
                        <SelectGroup>
                          <SelectItem value="low">
                            {{ t('components.serverForm.oracle.serviceLevels.low') }}
                          </SelectItem>
                          <SelectItem value="medium">
                            {{ t('components.serverForm.oracle.serviceLevels.medium') }}
                          </SelectItem>
                          <SelectItem value="high">
                            {{ t('components.serverForm.oracle.serviceLevels.high') }}
                          </SelectItem>
                          <SelectItem value="tp">
                            {{ t('components.serverForm.oracle.serviceLevels.tp') }}
                          </SelectItem>
                          <SelectItem value="tpurgent">
                            {{ t('components.serverForm.oracle.serviceLevels.tpurgent') }}
                          </SelectItem>
                        </SelectGroup>
                      </SelectContent>
                    </Select>
                  </div>
                </div>
                <div class="space-y-2">
                  <Label>{{ t('components.serverForm.oracle.tnsAlias') }}<span class="text-destructive ml-0.5">*</span></Label>
                  <p v-if="loadingAliases" class="text-sm text-muted-foreground">
                    {{ t('components.serverForm.oracle.loadingAliases') }}
                  </p>
                  <p v-else-if="tnsAliases.length === 0 && oracleTnsAdminDir" class="text-sm text-muted-foreground">
                    No aliases found in selected wallet directory
                  </p>
                  <p v-else-if="tnsAliases.length === 0" class="text-sm text-muted-foreground">
                    Select a wallet directory to load aliases
                  </p>
                  <Select v-else :model-value="oracleTnsAlias" @update:model-value="onTnsAliasChange">
                    <SelectTrigger>
                      <SelectValue :placeholder="t('components.serverForm.oracle.tnsAlias')" />
                    </SelectTrigger>
                    <SelectContent>
                      <SelectGroup>
                        <SelectItem v-for="alias in tnsAliases" :key="alias" :value="alias">
                          {{ alias }}
                        </SelectItem>
                      </SelectGroup>
                    </SelectContent>
                  </Select>
                </div>
              </TabsContent>
            </Tabs>
          </div>
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
                <Label for="sqlite-path">{{ t('components.serverForm.labels.databaseFilePath') }}<span class="text-destructive ml-0.5">*</span></Label>
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
                    type="button"
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
                    <span class="i-carbon-document text-muted-foreground h-4 w-4" />
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

        <!-- SSL Configuration (not for SQLite, basic-only for Oracle) -->
        <SslConfigSection
          v-if="!isFileBased && showsStandardHostFields"
          v-model="formData.ssl"
          :db-type="formData.type"
          :errors="formErrors"
        />

        <!-- Advanced Configuration: SSH Tunnel -->
        <div v-if="!isFileBased" class="pt-2">
          <Button
            type="button"
            variant="ghost"
            size="sm"
            class="text-muted-foreground w-full justify-between"
            @click="showAdvanced = !showAdvanced"
          >
            Advanced Configuration
            <span class="i-carbon-chevron-down h-4 w-4 transition-transform" :class="[showAdvanced ? 'rotate-180' : '']" />
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
              <Label for="use-ssh">{{ t('components.serverForm.ssh.useSshTunnel') }}</Label>
            </div>

            <template v-if="formData.sshTunnel?.enabled">
              <div class="space-y-2">
                <Label for="ssh-host">{{ t('components.serverForm.ssh.sshHost') }}</Label>
                <Input id="ssh-host" v-model="formData.sshTunnel.host" placeholder="ssh.example.com" />
              </div>

              <div class="gap-4 grid grid-cols-2">
                <div class="space-y-2">
                  <Label for="ssh-port">{{ t('components.serverForm.ssh.sshPort') }}</Label>
                  <Input id="ssh-port" v-model.number="formData.sshTunnel.port" type="number" placeholder="22" />
                </div>
                <div class="space-y-2">
                  <Label for="ssh-user">{{ t('components.serverForm.ssh.username') }}</Label>
                  <Input id="ssh-user" v-model="formData.sshTunnel.username" placeholder="username" autocomplete="off" />
                </div>
              </div>

              <div class="space-y-2">
                <Label for="ssh-auth">{{ t('components.serverForm.ssh.authMethod') }}</Label>
                <Select v-model="formData.sshTunnel.authMethod">
                  <SelectTrigger id="ssh-auth">
                    <SelectValue />
                  </SelectTrigger>
                  <SelectContent>
                    <SelectGroup>
                      <SelectItem value="password">
                        {{ t('components.serverForm.ssh.password') }}
                      </SelectItem>
                      <SelectItem value="privateKey">
                        {{ t('components.serverForm.ssh.privateKey') }}
                      </SelectItem>
                      <SelectItem value="agent">
                        {{ t('components.serverForm.ssh.sshAgent') }}
                      </SelectItem>
                    </SelectGroup>
                  </SelectContent>
                </Select>
              </div>

              <template v-if="formData.sshTunnel.authMethod === 'password'">
                <div class="space-y-2">
                  <Label for="ssh-password">{{ t('components.serverForm.ssh.sshPassword') }}</Label>
                  <Input id="ssh-password" v-model="formData.sshTunnel.password" type="password" :placeholder="t('components.serverForm.ssh.sshPasswordPlaceholder')" autocomplete="off" />
                </div>
              </template>

              <template v-if="formData.sshTunnel.authMethod === 'privateKey'">
                <div class="space-y-2">
                  <Label for="ssh-key">{{ t('components.serverForm.ssh.privateKeyPath') }}</Label>
                  <Input id="ssh-key" v-model="formData.sshTunnel.privateKey" placeholder="/path/to/id_rsa" />
                </div>
                <div class="space-y-2">
                  <Label for="ssh-passphrase">{{ t('components.serverForm.ssh.passphraseOptional') }}</Label>
                  <Input id="ssh-passphrase" v-model="formData.sshTunnel.privateKeyPassphrase" type="password" :placeholder="t('components.serverForm.ssh.keyPassphrasePlaceholder')" autocomplete="off" />
                </div>
              </template>

              <template v-if="formData.sshTunnel.authMethod === 'agent'">
                <p class="text-sm text-muted-foreground">
                  {{ t('components.serverForm.ssh.agentHelpText') }}
                </p>
              </template>
            </template>
          </div>
        </div>

        <!-- Test Connection Status -->
        <div
          v-if="testStatus !== 'idle' || setupSteps.some(s => s.status !== 'pending')" class="p-3 rounded-md" :class="{
            'bg-blue-50 dark:bg-blue-900/10': testStatus === 'testing',
            'bg-green-50 dark:bg-green-900/10': testStatus === 'success',
            'bg-red-50 dark:bg-red-900/10': testStatus === 'error',
          }"
        >
          <!-- Per-step status list (shown during and after setup) -->
          <div class="space-y-1.5">
            <div
              v-for="step in setupSteps"
              :key="step.id"
              class="flex items-center gap-2 text-xs min-w-0"
              :class="step.status === 'done' ? 'text-green-600 dark:text-green-400' : step.status === 'running' ? 'text-foreground' : step.status === 'error' ? 'text-red-600 dark:text-red-400' : 'text-muted-foreground'"
            >
              <span v-if="step.status === 'done'" class="i-carbon-checkmark h-3.5 w-3.5 shrink-0" />
              <span v-else-if="step.status === 'running' && !step.progress" class="i-carbon-loading h-3.5 w-3.5 animate-spin shrink-0" />
              <span v-else-if="step.status === 'running'" class="i-carbon-loading h-3.5 w-3.5 animate-spin shrink-0 text-blue-500" />
              <span v-else-if="step.status === 'error'" class="i-carbon-close h-3.5 w-3.5 shrink-0" />
              <span v-else class="i-carbon-circle-dash h-3.5 w-3.5 shrink-0" />
              <span class="font-medium whitespace-nowrap">{{ step.label }}</span>
              <!-- Spacer + progress bar: fill remaining space -->
              <span v-if="step.status === 'running' && step.progress" class="flex-1 flex items-center gap-1.5 min-w-0">
                <span class="flex-1 bg-muted rounded-full h-1 min-w-[40px]">
                  <span
                    class="block bg-primary rounded-full h-1 transition-all duration-300"
                    :style="{ width: `${Math.min(100, (step.progress.downloaded / step.progress.total) * 100)}%` }"
                  />
                </span>
                <span class="tabular-nums text-muted-foreground shrink-0 w-[3.5ch] text-right">{{ Math.round(step.progress.downloaded / 1024 / 1024 * 10) / 10 }} MB</span>
              </span>
              <!-- Error + retry button (pushed right) -->
              <span v-if="step.status === 'error'" class="flex-1 flex items-center justify-end gap-1.5 min-w-0">
                <span class="text-red-500 truncate text-right max-w-[200px]">{{ step.error || 'Failed' }}</span>
                <button
                  type="button"
                  class="inline-flex items-center gap-0.5 text-muted-foreground hover:text-foreground transition-colors cursor-pointer shrink-0"
                  :title="`Retry ${step.label}`"
                  @click="retryStep(step.id)"
                >
                  <span class="i-carbon-renew h-3.5 w-3.5" />
                </button>
              </span>
            </div>
            <!-- Loading state during connection test (always visible when testing after steps) -->
            <div v-if="testStatus === 'testing'" class="flex items-center gap-1.5 pt-1 text-xs text-blue-600 dark:text-blue-400 border-t border-border/50 mt-1.5">
              <span class="i-carbon-loading h-3.5 w-3.5 animate-spin shrink-0" />
              <span class="font-medium">{{ t('common.status.testing') }}</span>
            </div>
            <!-- Summary result -->
            <div v-if="testStatus === 'success'" class="flex items-center gap-1.5 pt-1 text-xs text-green-600 dark:text-green-400 border-t border-border/50 mt-1.5">
              <span class="i-carbon-checkmark h-3.5 w-3.5 shrink-0" />
              <span class="font-medium">{{ t('common.status.success') }}</span>
            </div>
            <div v-else-if="testStatus === 'error'" class="pt-1 text-xs text-red-600 dark:text-red-400 border-t border-border/50 mt-1.5 space-y-0.5">
              <div class="flex items-center gap-1.5">
                <span class="i-carbon-close h-3.5 w-3.5 shrink-0" />
                <span class="font-medium">{{ t('common.status.failed') }}</span>
                <button
                  type="button"
                  class="ml-1 inline-flex items-center gap-0.5 text-muted-foreground hover:text-foreground transition-colors cursor-pointer"
                  @click="handleTestConnection"
                >
                  <span class="i-carbon-renew h-3 w-3" />
                  <span class="text-xs">Retry</span>
                </button>
              </div>
              <p v-if="testError" class="pl-5 text-red-500">{{ testError }}</p>
            </div>
          </div>
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
          <Button type="submit" :disabled="testStatus === 'testing'">
            {{ isEditing ? t('common.buttons.saveChanges') : t('common.buttons.createConnection') }}
          </Button>
        </div>
      </form>
    </DialogContent>
  </Dialog>
</template>
