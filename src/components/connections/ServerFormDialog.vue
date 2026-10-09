<script setup lang="ts">
import type { OracleConnectionOptions, ServerConnection } from '@/store'
import type { DatabaseFileAssessment } from '@/utils/databaseFiles'
import { invoke } from '@tauri-apps/api/core'
import { open as openDialog } from '@tauri-apps/plugin-dialog'
import { computed, ref, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import { Button } from '@/components/ui/button'
import { SearchableSelect } from '@/components/ui/combobox'
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
import { useDownloadEvents } from '@/composables/useDownloadEvents'
import { toast } from '@/composables/useNotifications'
import { usePlatform } from '@/composables/usePlatform'
import { jdbcApi } from '@/datasources/jdbcApi'
import { buildOracleOptions, buildTransportLayers, databasePlaceholderFor, DatabaseType, dbTypeToBackend, isDatabaseRequired, isJdbcDatabase, resolveDatabase, useConnectionStore } from '@/store'
import { DEFAULT_SSL_MODE, sslModeToBackend, validateSslConfig } from '@/types/connection'
import { buildMemoryDatabaseHost, createMemoryDatabaseLabel, isMemoryDatabaseHost, isSameDatabaseFile, memoryDatabaseLabel } from '@/utils/databaseFiles'
import { classifyDuckDbFailure } from '@/utils/duckdbErrors'
import DatabaseFileField from './DatabaseFileField.vue'
import SshTunnelConfigPanel from './SshTunnelConfigPanel.vue'
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
const dl = useDownloadEvents()

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
  [DatabaseType.SQLCIPHER]: 0,
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
  [DatabaseType.OCEANBASE_ORACLE]: 2881,
  [DatabaseType.TDSQL]: 3306,
  [DatabaseType.POLARDB]: 3306,
  [DatabaseType.DAMENG]: 5236,
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

const showPasswords = ref<Record<string, boolean>>({})
function togglePassword(key: string) {
  showPasswords.value[key] = !showPasswords.value[key]
}

// Sync download progress events into setup steps
watch(() => ({ ...dl.progress }), (progress) => {
  for (const [id] of Object.entries(progress)) {
    const kind = dl.getKind(id)
    const stepId = kind === 'driver' ? 'driver' : kind // jre→jre, bridge→bridge, driver→driver
    const step = setupSteps.value.find(s => s.id === stepId)
    if (step && step.status === 'running' && dl.getProgress(id)) {
      const p = dl.getProgress(id)!
      step.progress = { downloaded: p.downloaded, total: p.total }
    }
  }
}, { deep: true })

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

// SQLite-specific state
const sqliteTab = ref<'file' | 'in-memory'>('file')
const savedFilePath = ref<string>('') // Preserve file path when switching to in-memory

// DuckDB-specific state. DuckDB mirrors the SQLite File / In-Memory split, but
// in-memory databases are *named* instances (`memory:<label>`): the JDBC pool
// opens several connections, and unnamed in-memory databases are private to a
// single connection, so every pooled connection would otherwise see its own
// empty database.
const duckdbTab = ref<'file' | 'in-memory'>('file')
const savedDuckDbFilePath = ref<string>('')
const duckdbMemoryLabel = ref<string>('')
const duckdbFileStatus = ref<DatabaseFileAssessment | null>(null)

const connectionStore = useConnectionStore()
const { platform } = usePlatform()

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
      // Detect DuckDB File / In-Memory mode from the stored host
      if (props.connection.type === DatabaseType.DUCKDB) {
        if (isMemoryDatabaseHost(props.connection.host)) {
          duckdbTab.value = 'in-memory'
          duckdbMemoryLabel.value = memoryDatabaseLabel(props.connection.host) || createMemoryDatabaseLabel()
        }
        else {
          duckdbTab.value = 'file'
          savedDuckDbFilePath.value = props.connection.host
        }
      }
      // Sync Oracle options when editing
      if (props.connection.type === DatabaseType.ORACLE) {
        syncOracleFromFormData()
        const opts = formData.value.oracleOptions
        if (opts?.connectionMethod !== 'basic' && opts?.tnsAdminDir) {
          loadTnsAliases(opts.tnsAdminDir, true)
        }
      }
    }
    else {
      formData.value = { ...defaultConnection }
      sqliteTab.value = 'file'
      savedFilePath.value = ''
      duckdbTab.value = 'file'
      savedDuckDbFilePath.value = ''
      duckdbMemoryLabel.value = ''
      duckdbFileStatus.value = null
      resetOracleOptions()
    }
    testStatus.value = 'idle'
    testError.value = ''
    formErrors.value = {}
    setupSteps.value.forEach((s) => {
      s.status = 'pending'
      s.progress = undefined
      s.error = undefined
    })
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

// Watch for DuckDB tab changes: in-memory mode stores a named instance host
watch(duckdbTab, (tab) => {
  if (formData.value.type !== DatabaseType.DUCKDB)
    return

  if (tab === 'in-memory') {
    if (!isMemoryDatabaseHost(formData.value.host))
      savedDuckDbFilePath.value = formData.value.host
    if (!duckdbMemoryLabel.value)
      duckdbMemoryLabel.value = createMemoryDatabaseLabel()
    formData.value.host = buildMemoryDatabaseHost(duckdbMemoryLabel.value)
    formData.value.readOnly = false
    duckdbFileStatus.value = null
  }
  else {
    formData.value.host = savedDuckDbFilePath.value
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
  if (type === DatabaseType.SQLITE || type === DatabaseType.SQLCIPHER || type === DatabaseType.DUCKDB) {
    formData.value.host = ''
    formData.value.port = 0
    if (type === DatabaseType.SQLITE || type === DatabaseType.DUCKDB) {
      formData.value.username = ''
      formData.value.password = ''
    }
    sqliteTab.value = 'file'
    savedFilePath.value = ''
  }
  if (type === DatabaseType.DUCKDB) {
    duckdbTab.value = 'file'
    savedDuckDbFilePath.value = ''
    duckdbMemoryLabel.value = ''
    duckdbFileStatus.value = null
    formData.value.readOnly = false
  }
  if (type !== DatabaseType.DUCKDB && type !== DatabaseType.SQLITE) {
    formData.value.readOnly = false
  }
}

const isFileBased = computed(() =>
  formData.value.type === DatabaseType.SQLITE
  || formData.value.type === DatabaseType.SQLCIPHER
  || formData.value.type === DatabaseType.DUCKDB,
)

// SQLCipher is file-based but also needs a password (encryption key)
const isEncryptedFileDb = computed(() =>
  formData.value.type === DatabaseType.SQLCIPHER,
)

const isJdbcDb = computed(() => isJdbcDatabase(formData.value.type))

const isDuckDb = computed(() => formData.value.type === DatabaseType.DUCKDB)

const isDuckDbFileMode = computed(() =>
  isDuckDb.value
  && duckdbTab.value === 'file'
  && !isMemoryDatabaseHost(formData.value.host),
)

// DuckDB caches one database instance per file inside the JDBC process, and
// options are instance-scoped: a second connection to the same file with a
// different configuration (read-only vs read-write) is rejected by the driver.
// Block that combination up front instead of surfacing the raw engine error.
const duckDbFileConnections = computed(() => {
  if (!isDuckDbFileMode.value || !formData.value.host.trim())
    return []
  return connectionStore.connections.filter(connection =>
    connection.id !== formData.value.id
    && connection.type === DatabaseType.DUCKDB
    && !isMemoryDatabaseHost(connection.host)
    && isSameDatabaseFile(connection.host, formData.value.host, platform.value ?? 'unknown'),
  )
})

const conflictingDuckDbConnection = computed(() =>
  duckDbFileConnections.value.find(connection =>
    (connection.readOnly ?? false) !== (formData.value.readOnly ?? false),
  ),
)

const sharedDuckDbConnection = computed(() =>
  duckDbFileConnections.value.find(connection =>
    (connection.readOnly ?? false) === (formData.value.readOnly ?? false),
  ),
)

function handleDuckDbFileStatus(status: DatabaseFileAssessment) {
  duckdbFileStatus.value = status
}

const testFailureHint = computed(() =>
  isDuckDb.value ? classifyDuckDbFailure(testError.value) : null,
)

async function retryAsReadOnly() {
  formData.value.readOnly = true
  testStatus.value = 'idle'
  await handleTestConnection()
}

async function retryAsReadWrite() {
  formData.value.readOnly = false
  testStatus.value = 'idle'
  await handleTestConnection()
}

function switchToConnection(connection: ServerConnection) {
  if (connection.id)
    connectionStore.setActiveConnection(connection.id)
  isOpen.value = false
}

const dbTypeI18nKeyMap: Record<string, string> = {
  MANTICORESEARCH: 'manticore',
}

const databaseTypeOptions = computed(() =>
  Object.values(DatabaseType).map((value) => {
    const i18nKey = dbTypeI18nKeyMap[value] || value.toLowerCase()
    const fullKey = `components.serverForm.databaseTypes.${i18nKey}`
    const label = t(fullKey)
    return { label: label === fullKey ? value : label, value }
  }),
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

// TNS alias handling: return the selected alias as-is for TNS/Cloud Wallet
const tnsFullAlias = computed(() => {
  if (!oracleTnsAlias.value || oracleMethod.value === 'basic')
    return undefined
  return oracleTnsAlias.value
})

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

const isDbNameRequired = computed(() => isDatabaseRequired(formData.value.type))

const databasePlaceholder = computed(() =>
  databasePlaceholderFor[formData.value.type] || t('components.serverForm.placeholders.database'),
)

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
  if (!isOracle.value)
    return undefined
  const hasTnsAlias = oracleMethod.value !== 'basic' && !!tnsFullAlias.value
  return {
    connectionMethod: oracleMethod.value,
    sidOrService: oracleMethod.value === 'basic' ? oracleSidOrService.value : undefined,
    role: oracleRole.value,
    tnsAdminDir: oracleMethod.value !== 'basic' ? oracleTnsAdminDir.value || undefined : undefined,
    tnsAlias: hasTnsAlias ? tnsFullAlias.value : undefined,
    walletPassword: oracleMethod.value === 'cloud_wallet' ? oracleWalletPassword.value || undefined : undefined,
    // Service level is encoded in the TNS alias suffix (e.g. dbname_low, dbname_medium)
    serviceLevel: undefined,
  }
}

function onOracleMethodChange(method: string | number) {
  oracleMethod.value = method as 'basic' | 'tns' | 'cloud_wallet'
  oracleTnsAlias.value = ''
  // Preserve host/port across method switches — don't clear user-entered values
}

function onTnsAliasChange(val: string) {
  oracleTnsAlias.value = val
}

// Sync formData.oracleOptions whenever relevant refs change
watch([oracleMethod, oracleSidOrService, oracleRole, oracleTnsAdminDir, oracleTnsAlias, oracleWalletPassword, oracleServiceLevel], () => {
  formData.value.oracleOptions = buildOracleOptionsFromRefs()
})

// Load TNS aliases when directory changes (TNS or Cloud Wallet)
// Set preserveAlias=true when editing to avoid clearing a previously saved alias
async function loadTnsAliases(dir: string, preserveAlias = false) {
  if (!dir) {
    tnsAliases.value = []
    return
  }
  loadingAliases.value = true
  try {
    const raw = await jdbcApi.listTnsAliases(dir)
    tnsAliases.value = raw.sort()
    if (!preserveAlias && tnsAliases.value.length === 0 && oracleTnsAlias.value) {
      oracleTnsAlias.value = ''
    }
    // In edit mode, drop the alias if it no longer exists in the loaded list
    // (handles corrupted saved data, e.g. alias with repeated _medium suffix)
    if (preserveAlias && oracleTnsAlias.value && tnsAliases.value.length > 0 && !tnsAliases.value.includes(oracleTnsAlias.value)) {
      oracleTnsAlias.value = ''
    }
  }
  catch (e) {
    console.error('Failed to list TNS aliases:', e)
    tnsAliases.value = []
    if (!preserveAlias && oracleTnsAlias.value)
      oracleTnsAlias.value = ''
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

function validateForm(): boolean {
  const errors: Record<string, string> = {}

  if (!formData.value.name.trim()) {
    errors.name = t('components.serverForm.errors.nameRequired')
  }

  if (isFileBased.value) {
    if (!isMemoryDatabaseHost(formData.value.host) && !formData.value.host.trim()) {
      errors.host = t('components.serverForm.errors.filePathRequired')
    }
    else if (conflictingDuckDbConnection.value) {
      errors.host = t('components.serverForm.duckdb.conflictingConnection', {
        name: conflictingDuckDbConnection.value.name,
      })
    }
    else if (isDuckDbFileMode.value && duckdbFileStatus.value?.status === 'directory') {
      errors.host = t('components.serverForm.fileStatus.directory')
    }
    else if (isDuckDbFileMode.value && duckdbFileStatus.value?.status === 'unsupported-path') {
      errors.host = t('components.serverForm.fileStatus.unsupportedPath')
    }
  }
  else if (isOracle.value && !showsStandardHostFields.value) {
    // Oracle TNS / Cloud Wallet: TNS admin dir and alias required
    if (!oracleTnsAdminDir.value.trim()) {
      errors.tnsAdminDir = t('components.serverForm.errors.tnsAdminDirRequired')
    }
    if (!oracleTnsAlias.value.trim()) {
      errors.tnsAlias = t('components.serverForm.errors.tnsAliasRequired')
    }
  }
  else {
    // Non-Oracle OR Oracle Basic: host, port, database (SID/Service Name) required
    if (!formData.value.host.trim()) {
      errors.host = t('components.serverForm.errors.hostRequired')
    }
    if (!formData.value.port || formData.value.port <= 0) {
      errors.port = t('components.serverForm.errors.portInvalid')
    }
    // For Oracle Basic, database field IS the SID/Service Name — required
    if (isOracle.value && !formData.value.database?.trim()) {
      errors.database = t('components.serverForm.errors.databaseRequired')
    }
    // For engines with mode-dependent or no default database (EnterpriseDB, TimescaleDB, Redshift)
    if (isDbNameRequired.value && !formData.value.database?.trim()) {
      errors.database = t('components.serverForm.errors.databaseNameRequired')
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
  if (testStatus.value === 'testing')
    return
  if (!validateForm())
    return

  testStatus.value = 'testing'
  testError.value = ''

  // Reset steps to pending — checks will promote to done or running
  setupSteps.value.forEach((s) => {
    s.status = 'pending'
    s.progress = undefined
    s.error = undefined
  })

  try {
    const dbType = mapDatabaseTypeToBackend(formData.value.type)

    // For JDBC bridge databases, ensure JRE/bridge/driver are ready first
    if (isJdbcDb.value) {
      const jreStep = setupSteps.value.find(s => s.id === 'jre')!
      const bridgeStep = setupSteps.value.find(s => s.id === 'bridge')!
      const driverStep = setupSteps.value.find(s => s.id === 'driver')!

      const results = await Promise.allSettled([
        // JRE download (skips if already installed)
        invoke('check_jre_status').then(async (status: any) => {
          if (!status.installed) {
            const ok = await runStep(jreStep)
            if (!ok)
              throw new Error('JRE download failed')
          }
          else { jreStep.status = 'done' }
        }),
        // Bridge download (skips if already installed)
        invoke('check_bridge_status').then(async (status: any) => {
          if (!status.installed) {
            const ok = await runStep(bridgeStep)
            if (!ok)
              throw new Error('Bridge download failed')
          }
          else { bridgeStep.status = 'done' }
        }),
        // JDBC driver download directly from Maven Central (no Java needed)
        (async () => {
          driverStep.status = 'running'
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
        if (firstError?.error)
          testError.value = firstError.error
        return
      }
    }
    const driverStep = isJdbcDb.value ? setupSteps.value.find(s => s.id === 'driver') : undefined

    // Test the actual connection
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
      transport_layers: await buildTransportLayers(formData.value.sshTunnel),
      oracle_options: buildOracleOptions(formData.value.oracleOptions),
      read_only: formData.value.readOnly ?? false,
    }

    // Small yield to let Vue render the loading state before the blocking invoke
    await new Promise(r => setTimeout(r, 50))

    const result = await invoke<{ is_connected: boolean, server_version?: string }>('test_connection', { config })

    if (result.is_connected) {
      if (result.server_version) {
        formData.value.serverVersion = result.server_version
      }
      if (driverStep)
        driverStep.status = 'done'
      testStatus.value = 'success'
    }
    else {
      if (driverStep)
        driverStep.status = 'error'
      const msg = 'Connection returned not connected'
      if (driverStep)
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
      const jreStepFound = setupSteps.value.find(s => s.id === 'jre')
      if (jreStepFound) {
        jreStepFound.status = 'error'
        jreStepFound.error = 'JRE is missing or broken'
      }
    }
    const running = setupSteps.value.find(s => s.status === 'running')
    if (running) {
      running.status = 'error'
      running.error = msg
    }
  }

  // If testStatus is error but testError is empty, collect from first errored step
  if (testStatus.value === 'error' && !testError.value) {
    const firstError = setupSteps.value.find(s => s.status === 'error')
    if (firstError?.error)
      testError.value = firstError.error
  }
}

/** Retry an individual setup step by its id */
async function retryStep(stepId: string) {
  const step = setupSteps.value.find(s => s.id === stepId)
  if (!step || step.status !== 'error')
    return

  if (stepId === 'driver') {
    // For driver/connection, re-run full test
    await handleTestConnection()
    return
  }

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
      await handleTestConnection()
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
          <SearchableSelect
            :model-value="formData.type"
            :options="databaseTypeOptions"
            :search-threshold="5"
            :placeholder="t('components.serverForm.placeholders.selectType')"
            class="w-full"
            @update:model-value="handleDatabaseTypeChange"
          >
            <template #selected-prepend>
              <img
                v-if="formData.type"
                :src="getDatabaseIcon(formData.type as DatabaseType)"
                alt=""
                class="mr-2 h-5 w-5 object-contain"
              >
            </template>
            <template #option="{ option }">
              <div class="flex gap-2 items-center">
                <img
                  :src="getDatabaseIcon(option.value as DatabaseType)"
                  alt=""
                  class="h-5 w-5 object-contain"
                >
                {{ option.label }}
              </div>
            </template>
          </SearchableSelect>
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
            <Label for="database">
              {{ t('components.serverForm.labels.database') }}
              <span v-if="isDbNameRequired" class="text-destructive ml-0.5">*</span>
            </Label>
            <Input
              id="database"
              v-model="formData.database"
              :placeholder="databasePlaceholder"
              :class="{ 'border-destructive': formErrors.database }"
            />
            <p v-if="formErrors.database" class="text-sm text-destructive">
              {{ formErrors.database }}
            </p>
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
                  <Label for="oracle-svc">{{ databaseLabel }}<span class="text-destructive ml-0.5">*</span></Label>
                  <Input
                    id="oracle-svc"
                    v-model="formData.database"
                    :placeholder="databaseLabel"
                    :class="{ 'border-destructive': formErrors.database }"
                  />
                  <p v-if="formErrors.database" class="text-sm text-destructive">
                    {{ formErrors.database }}
                  </p>
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
                      :class="{ 'border-destructive': formErrors.tnsAdminDir }"
                      class="flex-1"
                    />
                    <Button type="button" variant="outline" size="sm" @click="browseDirectory('tns')">
                      {{ t('components.serverForm.oracle.browse') }}
                    </Button>
                  </div>
                  <p v-if="formErrors.tnsAdminDir" class="text-sm text-destructive">
                    {{ formErrors.tnsAdminDir }}
                  </p>
                </div>
                <div class="space-y-2">
                  <Label>{{ t('components.serverForm.oracle.tnsAlias') }}<span class="text-destructive ml-0.5">*</span></Label>
                  <p v-if="loadingAliases" class="text-sm text-muted-foreground">
                    {{ t('components.serverForm.oracle.loadingAliases') }}
                  </p>
                  <!-- Show saved alias value when aliases haven't loaded yet (edit mode) -->
                  <p v-else-if="tnsAliases.length === 0 && oracleTnsAdminDir && oracleTnsAlias" class="text-sm text-muted-foreground">
                    {{ oracleTnsAlias }}
                  </p>
                  <p v-else-if="tnsAliases.length === 0 && oracleTnsAdminDir" class="text-sm text-muted-foreground">
                    No aliases found in selected directory
                  </p>
                  <p v-else-if="tnsAliases.length === 0" class="text-sm text-muted-foreground">
                    Select a TNS directory to load aliases
                  </p>
                  <Select v-else :model-value="oracleTnsAlias" @update:model-value="onTnsAliasChange">
                    <SelectTrigger :class="{ 'border-destructive': formErrors.tnsAlias }">
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
                  <p v-if="formErrors.tnsAlias" class="text-sm text-destructive">
                    {{ formErrors.tnsAlias }}
                  </p>
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
                      :class="{ 'border-destructive': formErrors.tnsAdminDir }"
                      class="flex-1"
                    />
                    <Button type="button" variant="outline" size="sm" @click="browseDirectory('wallet')">
                      {{ t('components.serverForm.oracle.browse') }}
                    </Button>
                  </div>
                  <p v-if="formErrors.tnsAdminDir" class="text-sm text-destructive">
                    {{ formErrors.tnsAdminDir }}
                  </p>
                </div>
                <div class="space-y-2">
                  <Label>{{ t('components.serverForm.oracle.walletPassword') }}</Label>
                  <div class="relative">
                    <Input
                      v-model="oracleWalletPassword"
                      :type="showPasswords.wallet ? 'text' : 'password'"
                      :placeholder="t('components.serverForm.oracle.walletPassword')"
                      class="pr-8"
                    />
                    <button type="button" class="text-muted-foreground right-2 top-1/2 absolute hover:text-foreground -translate-y-1/2" @click="togglePassword('wallet')">
                      <span class="i-carbon-view h-4 w-4 block" />
                    </button>
                  </div>
                </div>
                <div class="space-y-2">
                  <Label>{{ t('components.serverForm.oracle.tnsAlias') }}<span class="text-destructive ml-0.5">*</span></Label>
                  <p v-if="loadingAliases" class="text-sm text-muted-foreground">
                    {{ t('components.serverForm.oracle.loadingAliases') }}
                  </p>
                  <!-- Show saved alias value when aliases haven't loaded yet (edit mode) -->
                  <p v-else-if="tnsAliases.length === 0 && oracleTnsAdminDir && oracleTnsAlias" class="text-sm text-muted-foreground">
                    {{ oracleTnsAlias }}
                  </p>
                  <p v-else-if="tnsAliases.length === 0 && oracleTnsAdminDir" class="text-sm text-muted-foreground">
                    No aliases found in selected wallet directory
                  </p>
                  <p v-else-if="tnsAliases.length === 0" class="text-sm text-muted-foreground">
                    Select a wallet directory to load aliases
                  </p>
                  <Select v-else :model-value="oracleTnsAlias" @update:model-value="onTnsAliasChange">
                    <SelectTrigger :class="{ 'border-destructive': formErrors.tnsAlias }">
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
                  <p v-if="formErrors.tnsAlias" class="text-sm text-destructive">
                    {{ formErrors.tnsAlias }}
                  </p>
                </div>
              </TabsContent>
            </Tabs>
          </div>
        </div>

        <!-- SQLite/SQLCipher/DuckDB-specific fields -->
        <div v-if="isFileBased" class="space-y-4">
          <!-- SQLite with file/in-memory tabs -->
          <template v-if="formData.type === DatabaseType.SQLITE">
            <Tabs v-model="sqliteTab">
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
                <DatabaseFileField
                  v-model="formData.host"
                  engine="sqlite"
                  required
                  :label="t('components.serverForm.labels.databaseFilePath')"
                  :placeholder="t('components.serverForm.placeholders.filePath')"
                  :error-message="formErrors.host"
                  @picking="isPickingFile = $event"
                />
              </TabsContent>

              <!-- In-Memory Mode -->
              <TabsContent value="in-memory">
                <p class="text-sm text-muted-foreground">
                  {{ t('components.serverForm.sqlite.inMemoryHint') }}
                </p>
              </TabsContent>
            </Tabs>
          </template>

          <!-- SQLCipher: file picker + encryption key -->
          <template v-else-if="formData.type === DatabaseType.SQLCIPHER">
            <DatabaseFileField
              v-model="formData.host"
              engine="sqlite"
              required
              :label="t('components.serverForm.labels.databaseFilePath')"
              :placeholder="t('components.serverForm.placeholders.filePath')"
              :error-message="formErrors.host"
              @picking="isPickingFile = $event"
            />
          </template>

          <!-- DuckDB: same File / In-Memory split as SQLite -->
          <template v-else>
            <Tabs v-model="duckdbTab">
              <TabsList class="grid grid-cols-2 w-full">
                <TabsTrigger value="file">
                  {{ t('components.serverForm.sqlite.modes.file') }}
                </TabsTrigger>
                <TabsTrigger value="in-memory">
                  {{ t('components.serverForm.sqlite.modes.inMemory') }}
                </TabsTrigger>
              </TabsList>

              <TabsContent value="file" class="space-y-4">
                <DatabaseFileField
                  v-model="formData.host"
                  engine="duckdb"
                  required
                  :label="t('components.serverForm.labels.databaseFilePath')"
                  :placeholder="t('components.serverForm.placeholders.duckdbFilePath')"
                  :error-message="formErrors.host"
                  :suggested-name="formData.name ? `${formData.name}.duckdb` : undefined"
                  @status="handleDuckDbFileStatus"
                  @picking="isPickingFile = $event"
                />

                <p v-if="formData.serverVersion" class="text-xs text-muted-foreground">
                  {{ t('components.serverForm.duckdb.engineVersion', { version: formData.serverVersion }) }}
                </p>

                <div class="space-y-2">
                  <Label>{{ t('components.serverForm.duckdb.accessMode') }}</Label>
                  <div class="gap-3 grid grid-cols-2" role="radiogroup">
                    <button
                      type="button"
                      role="radio"
                      :aria-checked="!formData.readOnly"
                      class="p-3 text-left border-2 rounded-lg transition-colors" :class="[
                        !formData.readOnly
                          ? 'border-primary bg-primary/5'
                          : 'border-border bg-card hover:border-muted-foreground/50',
                      ]"
                      @click="formData.readOnly = false"
                    >
                      <div class="mb-1 flex gap-2 items-center">
                        <div
                          class="border-2 rounded-full flex shrink-0 h-3.5 w-3.5 items-center justify-center" :class="[
                            !formData.readOnly ? 'border-primary' : 'border-muted-foreground',
                          ]"
                        >
                          <div v-if="!formData.readOnly" class="rounded-full bg-primary h-1.5 w-1.5" />
                        </div>
                        <span class="text-sm font-medium">
                          {{ t('components.serverForm.duckdb.readWrite') }}
                        </span>
                      </div>
                      <p class="text-xs text-muted-foreground leading-relaxed pl-5">
                        {{ t('components.serverForm.duckdb.readWriteHint') }}
                      </p>
                    </button>
                    <button
                      type="button"
                      role="radio"
                      :aria-checked="formData.readOnly === true"
                      class="p-3 text-left border-2 rounded-lg transition-colors" :class="[
                        formData.readOnly
                          ? 'border-primary bg-primary/5'
                          : 'border-border bg-card hover:border-muted-foreground/50',
                      ]"
                      @click="formData.readOnly = true"
                    >
                      <div class="mb-1 flex gap-2 items-center">
                        <div
                          class="border-2 rounded-full flex shrink-0 h-3.5 w-3.5 items-center justify-center" :class="[
                            formData.readOnly ? 'border-primary' : 'border-muted-foreground',
                          ]"
                        >
                          <div v-if="formData.readOnly" class="rounded-full bg-primary h-1.5 w-1.5" />
                        </div>
                        <span class="text-sm font-medium">
                          {{ t('components.serverForm.duckdb.readOnly') }}
                        </span>
                      </div>
                      <p class="text-xs text-muted-foreground leading-relaxed pl-5">
                        {{ t('components.serverForm.duckdb.readOnlyHint') }}
                      </p>
                    </button>
                  </div>
                </div>

                <div
                  v-if="conflictingDuckDbConnection"
                  class="text-xs text-amber-600 p-2 border border-amber-500/50 rounded-md flex gap-2 items-start dark:text-amber-500"
                >
                  <span class="i-carbon-warning mt-0.5 shrink-0 h-4 w-4" />
                  <div class="flex-1 space-y-1">
                    <p>{{ t('components.serverForm.duckdb.conflictingConnection', { name: conflictingDuckDbConnection.name }) }}</p>
                    <div class="flex gap-2">
                      <Button type="button" variant="outline" size="sm" class="text-xs px-2 h-6" @click="formData.readOnly = true">
                        {{ t('components.serverForm.duckdb.openReadOnly') }}
                      </Button>
                      <Button type="button" variant="ghost" size="sm" class="text-xs px-2 h-6" @click="switchToConnection(conflictingDuckDbConnection)">
                        {{ t('components.serverForm.duckdb.switchToConnection') }}
                      </Button>
                    </div>
                  </div>
                </div>

                <p v-else-if="sharedDuckDbConnection" class="text-xs text-muted-foreground">
                  {{ t('components.serverForm.duckdb.sharedConnection', { name: sharedDuckDbConnection.name }) }}
                </p>
              </TabsContent>

              <!-- In-Memory Mode -->
              <TabsContent value="in-memory" class="space-y-3">
                <p class="text-sm text-muted-foreground">
                  {{ t('components.serverForm.duckdb.inMemoryHint') }}
                </p>
                <div class="text-xs p-2 border rounded-md flex gap-2 items-center">
                  <span class="i-carbon-data-table text-muted-foreground shrink-0 h-4 w-4" />
                  <span class="text-muted-foreground">{{ t('components.serverForm.duckdb.inMemoryInstance') }}</span>
                  <span class="font-mono flex-1 truncate" :title="formData.host">{{ formData.host || '-' }}</span>
                </div>
                <p class="text-xs text-muted-foreground">
                  {{ t('components.serverForm.duckdb.inMemorySharedHint') }}
                </p>
                <p v-if="formData.serverVersion" class="text-xs text-muted-foreground">
                  {{ t('components.serverForm.duckdb.engineVersion', { version: formData.serverVersion }) }}
                </p>
              </TabsContent>
            </Tabs>
          </template>
        </div>

        <!-- Password (encryption key) for SQLCipher; Username and Password for others -->
        <div v-if="isEncryptedFileDb" class="space-y-2">
          <Label for="sqlcipher-key">{{ t('components.serverForm.labels.encryptionKey') }}<span class="text-destructive ml-0.5">*</span></Label>
          <div class="relative">
            <Input
              id="sqlcipher-key"
              v-model="formData.password"
              :type="showPasswords.db ? 'text' : 'password'"
              :placeholder="t('components.serverForm.placeholders.encryptionKey')"
              autocomplete="new-password"
              class="pr-8"
            />
            <button type="button" class="text-muted-foreground right-2 top-1/2 absolute hover:text-foreground -translate-y-1/2" @click="togglePassword('db')">
              <span class="i-carbon-view h-4 w-4 block" />
            </button>
          </div>
        </div>
        <div v-if="!isFileBased && !isEncryptedFileDb" class="gap-4 grid grid-cols-2">
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
            <div class="relative">
              <Input
                id="password"
                v-model="formData.password"
                :type="showPasswords.db ? 'text' : 'password'"
                :placeholder="t('components.serverForm.placeholders.password')"
                autocomplete="new-password"
                class="pr-8"
              />
              <button type="button" class="text-muted-foreground right-2 top-1/2 absolute hover:text-foreground -translate-y-1/2" @click="togglePassword('db')">
                <span class="i-carbon-view h-4 w-4 block" />
              </button>
            </div>
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
        <SshTunnelConfigPanel
          v-if="!isFileBased"
          :model-value="formData.sshTunnel"
          :remote-host="formData.host"
          :remote-port="formData.port"
          @update:model-value="formData.sshTunnel = $event"
        />

        <!-- Test Connection Status -->
        <div
          v-if="testStatus !== 'idle' || (isJdbcDb && setupSteps.some(s => s.status !== 'pending'))" class="p-3 rounded-md" :class="{
            'bg-blue-50 dark:bg-blue-900/10': testStatus === 'testing',
            'bg-green-50 dark:bg-green-900/10': testStatus === 'success',
            'bg-red-50 dark:bg-red-900/10': testStatus === 'error',
          }"
        >
          <!-- Per-step status list (shown during and after setup for JDBC databases) -->
          <div v-if="isJdbcDb" class="space-y-1.5">
            <div
              v-for="step in setupSteps"
              :key="step.id"
              class="text-xs flex gap-2 min-w-0 items-center"
              :class="step.status === 'done' ? 'text-green-600 dark:text-green-400' : step.status === 'running' ? 'text-foreground' : step.status === 'error' ? 'text-red-600 dark:text-red-400' : 'text-muted-foreground'"
            >
              <span v-if="step.status === 'done'" class="i-carbon-checkmark shrink-0 h-3.5 w-3.5" />
              <span v-else-if="step.status === 'running' && !step.progress" class="i-carbon-circle-dash shrink-0 h-3.5 w-3.5 animate-spin" />
              <span v-else-if="step.status === 'running'" class="i-carbon-circle-dash text-blue-500 shrink-0 h-3.5 w-3.5 animate-spin" />
              <span v-else-if="step.status === 'error'" class="i-carbon-close shrink-0 h-3.5 w-3.5" />
              <span v-else class="i-carbon-circle-dash shrink-0 h-3.5 w-3.5" />
              <span class="font-medium whitespace-nowrap">{{ step.label }}</span>
              <!-- Spacer + progress bar: fill remaining space -->
              <span v-if="step.status === 'running' && step.progress" class="flex flex-1 gap-1.5 min-w-0 items-center">
                <span class="rounded-full bg-muted flex-1 h-1 min-w-[40px]">
                  <span
                    class="rounded-full bg-primary h-1 block transition-all duration-300"
                    :style="{ width: `${Math.min(100, (step.progress.downloaded / step.progress.total) * 100)}%` }"
                  />
                </span>
                <span class="text-muted-foreground text-right shrink-0 w-[3.5ch] tabular-nums">{{ Math.round(step.progress.downloaded / 1024 / 1024 * 10) / 10 }} MB</span>
              </span>
              <!-- Error + retry button (pushed right) -->
              <span v-if="step.status === 'error'" class="flex flex-1 gap-1.5 min-w-0 items-center justify-end">
                <span class="text-red-500 text-right max-w-[200px] truncate">{{ step.error || 'Failed' }}</span>
                <button
                  type="button"
                  class="text-muted-foreground inline-flex shrink-0 gap-0.5 cursor-pointer transition-colors items-center hover:text-foreground"
                  :title="`Retry ${step.label}`"
                  @click="retryStep(step.id)"
                >
                  <span class="i-carbon-renew h-3.5 w-3.5" />
                </button>
              </span>
            </div>
          </div>
          <div class="space-y-1.5">
            <!-- Loading state during connection test (always visible when testing after steps) -->
            <div v-if="testStatus === 'testing'" class="text-xs text-blue-600 mt-1.5 pt-1 flex gap-1.5 items-center dark:text-blue-400" :class="{ 'border-t border-border/50': isJdbcDb }">
              <span class="i-carbon-circle-dash shrink-0 h-3.5 w-3.5 animate-spin" />
              <span class="font-medium">{{ t('common.status.testing') }}</span>
            </div>
            <!-- Summary result -->
            <div v-if="testStatus === 'success'" class="text-xs text-green-600 mt-1.5 pt-1 flex gap-1.5 items-center dark:text-green-400" :class="{ 'border-t border-border/50': isJdbcDb }">
              <span class="i-carbon-checkmark shrink-0 h-3.5 w-3.5" />
              <span class="font-medium">{{ t('common.status.success') }}</span>
            </div>
            <div v-else-if="testStatus === 'error'" class="text-xs text-red-600 mt-1.5 pt-1 space-y-0.5 dark:text-red-400" :class="{ 'border-t border-border/50': isJdbcDb }">
              <div class="flex gap-1.5 items-center">
                <span class="i-carbon-close shrink-0 h-3.5 w-3.5" />
                <span class="font-medium">{{ t('common.status.failed') }}</span>
                <button
                  type="button"
                  class="text-muted-foreground ml-1 inline-flex gap-0.5 cursor-pointer transition-colors items-center hover:text-foreground"
                  @click="handleTestConnection"
                >
                  <span class="i-carbon-renew h-3 w-3" />
                  <span class="text-xs">Retry</span>
                </button>
              </div>
              <p v-if="testError" class="text-red-500 pl-5">
                {{ testError }}
              </p>
              <div
                v-if="testFailureHint"
                class="text-xs text-amber-600 pl-5 flex gap-2 items-start dark:text-amber-500"
              >
                <span class="i-carbon-warning mt-0.5 shrink-0 h-4 w-4" />
                <div class="flex-1 space-y-1">
                  <p>{{ t(testFailureHint.messageKey) }}</p>
                  <Button
                    v-if="testFailureHint.action === 'use-read-only'"
                    type="button"
                    variant="outline"
                    size="sm"
                    class="text-xs px-2 h-6"
                    @click="retryAsReadOnly"
                  >
                    {{ t('components.serverForm.duckdb.retryReadOnly') }}
                  </Button>
                  <Button
                    v-else-if="testFailureHint.action === 'use-read-write'"
                    type="button"
                    variant="outline"
                    size="sm"
                    class="text-xs px-2 h-6"
                    @click="retryAsReadWrite"
                  >
                    {{ t('components.serverForm.duckdb.retryReadWrite') }}
                  </Button>
                </div>
              </div>
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
