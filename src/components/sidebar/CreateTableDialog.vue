<script setup lang="ts">
import { DatabaseType } from '@/store/connectionStore'
import { computed, ref, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import { Button } from '@/components/ui/button'
import {
  Dialog,
  DialogContent,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog'
import { Input } from '@/components/ui/input'
import { Label } from '@/components/ui/label'

type ColumnDef = {
  id: number
  name: string
  type: string
  length: string
  nullable: boolean
  defaultValue: string
  autoIncrement: boolean
  primaryKey: boolean
}

type Props = {
  open: boolean
  /** Current database context */
  database: string | null
  /** Optional schema context */
  schema?: string | null
  /** Database type for engine/options */
  databaseType?: DatabaseType
}

const props = withDefaults(defineProps<Props>(), {
  schema: null,
  databaseType: undefined,
})

const emit = defineEmits<{
  (e: 'update:open', value: boolean): void
  (e: 'create', tableName: string, sql: string): void
}>()

const { t } = useI18n()

const MYSQL_COMPAT = new Set<DatabaseType>([
  DatabaseType.MYSQL,
  DatabaseType.MARIADB,
  DatabaseType.TIDB,
  DatabaseType.OCEANBASE,
  DatabaseType.TDSQL,
  DatabaseType.POLARDB,
  DatabaseType.DORIS,
  DatabaseType.SELECTDB,
  DatabaseType.STARROCKS,
  DatabaseType.DATABEND,
  DatabaseType.GOLDENDB,
  DatabaseType.MANTICORESEARCH,
  DatabaseType.SINGLESTOREMEMSQL,
  DatabaseType.CLOUDSQLMYSQL,
])

const MYSQL_ENGINES = ['InnoDB', 'MyISAM', 'MEMORY', 'CSV', 'ARCHIVE', 'BLACKHOLE']

const isMySQL = computed(() => props.databaseType && MYSQL_COMPAT.has(props.databaseType))

const COMMON_TYPES = [
  'INTEGER',
  'BIGINT',
  'SMALLINT',
  'TINYINT',
  'SERIAL',
  'BIGSERIAL',
  'VARCHAR',
  'CHAR',
  'TEXT',
  'CLOB',
  'BOOLEAN',
  'FLOAT',
  'DOUBLE',
  'DECIMAL',
  'NUMERIC',
  'REAL',
  'DATE',
  'TIME',
  'TIMESTAMP',
  'TIMESTAMPTZ',
  'BLOB',
  'BYTEA',
  'UUID',
  'JSON',
  'JSONB',
  'ENUM',
  'ARRAY',
]

const tableName = ref('')
const tableEngine = ref('InnoDB')
const columns = ref<ColumnDef[]>([])
let nextColId = 1

watch(() => props.open, (open) => {
  if (open) {
    tableName.value = ''
    tableEngine.value = 'InnoDB'
    columns.value = []
    nextColId = 1
    addColumn()
  }
})

function addColumn() {
  columns.value = [
    ...columns.value,
    { id: nextColId++, name: '', type: 'VARCHAR', length: '255', nullable: true, defaultValue: '', autoIncrement: false, primaryKey: false },
  ]
}

function removeColumn(id: number) {
  if (columns.value.length <= 1)
    return
  columns.value = columns.value.filter(c => c.id !== id)
}

function moveColumn(fromIndex: number, direction: -1 | 1) {
  const toIndex = fromIndex + direction
  if (toIndex < 0 || toIndex >= columns.value.length)
    return
  const arr = [...columns.value];
  [arr[fromIndex], arr[toIndex]] = [arr[toIndex], arr[fromIndex]]
  columns.value = arr
}

const ddlPreview = computed(() => {
  if (!tableName.value.trim())
    return ''

  const name = tableName.value.trim()
  const schemaPrefix = props.schema ? `${props.schema}.` : ''
  const fullName = `${schemaPrefix}${name}`

  const colLines = columns.value
    .filter(c => c.name.trim())
    .map((c) => {
      let colDef = `  ${c.name.trim()} ${c.type}`
      if (c.length && ['VARCHAR', 'CHAR', 'DECIMAL', 'NUMERIC'].includes(c.type))
        colDef += `(${c.length})`
      if (c.autoIncrement && ['INTEGER', 'BIGINT', 'SMALLINT'].includes(c.type))
        colDef += ' GENERATED ALWAYS AS IDENTITY'
      if (!c.nullable)
        colDef += ' NOT NULL'
      if (c.defaultValue)
        colDef += ` DEFAULT ${c.defaultValue}`
      return colDef
    })

  const pkCols = columns.value.filter(c => c.primaryKey && c.name.trim()).map(c => c.name.trim())
  if (pkCols.length > 0)
    colLines.push(`  PRIMARY KEY (${pkCols.join(', ')})`)

  let ddl = `CREATE TABLE ${fullName} (\n${colLines.join(',\n')}\n)`
  if (isMySQL.value)
    ddl += ` ENGINE=${tableEngine.value}`
  ddl += ';'

  return ddl
})

function handleCreate() {
  if (!tableName.value.trim() || !columns.value.some(c => c.name.trim()))
    return
  emit('create', tableName.value.trim(), ddlPreview.value)
}
</script>

<template>
  <Dialog :open="open" @update:open="(v: boolean) => emit('update:open', v)">
    <DialogContent class="max-h-[85vh] overflow-y-auto sm:max-w-[700px]">
      <DialogHeader>
        <DialogTitle>{{ t('sidebar.databases.actions.createTable.title') }}</DialogTitle>
      </DialogHeader>

      <div class="space-y-4">
        <!-- Table name + Engine for MySQL -->
        <div class="flex gap-2 items-center">
          <Label class="text-xs shrink-0 w-24">{{ t('sidebar.databases.actions.createTable.tableName') }}</Label>
          <Input v-model="tableName" :placeholder="t('sidebar.databases.actions.createTable.tableNamePlaceholder')" class="text-xs flex-1" />
          <template v-if="isMySQL">
            <Label class="text-xs shrink-0">{{ t('sidebar.databases.actions.createTable.engine') }}</Label>
            <select
              v-model="tableEngine"
              class="text-xs px-2 border border-input rounded-md bg-background h-7 w-24"
            >
              <option v-for="eng in MYSQL_ENGINES" :key="eng" :value="eng">
                {{ eng }}
              </option>
            </select>
          </template>
        </div>

        <!-- Column grid -->
        <div class="space-y-1">
          <div class="flex items-center justify-between">
            <Label class="text-xs font-medium">{{ t('sidebar.databases.actions.createTable.columns') }}</Label>
            <Button variant="outline" size="sm" class="text-xs h-6" @click="addColumn">
              + {{ t('sidebar.databases.actions.createTable.addColumn') }}
            </Button>
          </div>

          <!-- Header row -->
          <div class="text-xs text-muted-foreground font-medium px-1 py-0.5 gap-1 grid grid-cols-12">
            <span class="col-span-1" />
            <span class="col-span-2">{{ t('sidebar.databases.actions.createTable.colName') }}</span>
            <span class="col-span-2">{{ t('sidebar.databases.actions.createTable.colType') }}</span>
            <span class="col-span-1">{{ t('sidebar.databases.actions.createTable.colLength') }}</span>
            <span class="text-center col-span-1">NOT NULL</span>
            <span class="text-center col-span-1">{{ t('sidebar.databases.actions.createTable.colAutoInc') }}</span>
            <span class="text-center col-span-1">PK</span>
            <span class="col-span-2">{{ t('sidebar.databases.actions.createTable.colDefault') }}</span>
            <span class="col-span-1" />
          </div>

          <!-- Column rows -->
          <div
            v-for="(col, index) in columns"
            :key="col.id"
            class="px-1 py-0.5 rounded gap-1 grid grid-cols-12 items-center hover:bg-accent/20"
          >
            <!-- Move buttons -->
            <div class="flex gap-0.5 col-span-1">
              <button
                class="text-muted-foreground flex h-4 w-4 items-center justify-center hover:text-foreground disabled:opacity-20"
                :disabled="index === 0"
                @click="moveColumn(index, -1)"
              >
                <span class="i-lucide-chevron-up h-3 w-3" />
              </button>
              <button
                class="text-muted-foreground flex h-4 w-4 items-center justify-center hover:text-foreground disabled:opacity-20"
                :disabled="index === columns.length - 1"
                @click="moveColumn(index, 1)"
              >
                <span class="i-lucide-chevron-down h-3 w-3" />
              </button>
            </div>

            <!-- Name -->
            <Input
              v-model="col.name"
              :placeholder="`col_${index + 1}`"
              class="text-xs col-span-2 h-7"
            />

            <!-- Type -->
            <select
              v-model="col.type"
              class="text-xs px-2 border border-input rounded-md bg-background col-span-2 h-7"
            >
              <option v-for="colType in COMMON_TYPES" :key="colType" :value="colType">
                {{ colType.toLowerCase() }}
              </option>
            </select>

            <!-- Length -->
            <Input
              v-model="col.length"
              :placeholder="t('sidebar.databases.actions.createTable.lengthPlaceholder')"
              class="text-xs col-span-1 h-7"
              :disabled="!['VARCHAR', 'CHAR', 'DECIMAL', 'NUMERIC'].includes(col.type)"
            />

            <!-- Not null -->
            <div class="flex col-span-1 justify-center">
              <input
                :checked="!col.nullable"
                type="checkbox"
                class="accent-primary h-3.5 w-3.5"
                @change="col.nullable = !($event.target as HTMLInputElement).checked"
              >
            </div>
            <div class="flex col-span-1 justify-center">
              <input
                v-model="col.autoIncrement"
                type="checkbox"
                class="accent-primary h-3.5 w-3.5"
              >
            </div>
            <div class="flex col-span-1 justify-center">
              <input
                v-model="col.primaryKey"
                type="checkbox"
                class="accent-primary h-3.5 w-3.5"
              >
            </div>

            <!-- Default -->
            <Input
              v-model="col.defaultValue"
              :placeholder="t('sidebar.databases.actions.createTable.defaultPlaceholder')"
              class="text-xs col-span-2 h-7"
            />

            <!-- Remove -->
            <div class="flex col-span-1 justify-center">
              <button
                class="text-muted-foreground flex h-5 w-5 items-center justify-center hover:text-destructive disabled:opacity-20"
                :disabled="columns.length <= 1"
                @click="removeColumn(col.id)"
              >
                <span class="i-lucide-x h-3.5 w-3.5" />
              </button>
            </div>
          </div>
        </div>

        <!-- SQL Preview -->
        <div v-if="ddlPreview" class="space-y-1">
          <Label class="text-xs font-medium">{{ t('sidebar.databases.actions.createTable.preview') }}</Label>
          <pre class="text-xs font-mono p-2 border rounded-md bg-muted/50 whitespace-pre-wrap overflow-x-auto">{{ ddlPreview }}</pre>
        </div>
      </div>

      <DialogFooter>
        <Button variant="outline" size="sm" @click="emit('update:open', false)">
          {{ t('sidebar.databases.actions.createTable.cancel') }}
        </Button>
        <Button
          size="sm"
          :disabled="!tableName.trim() || !columns.some(c => c.name.trim())"
          @click="handleCreate"
        >
          {{ t('sidebar.databases.actions.createTable.create') }}
        </Button>
      </DialogFooter>
    </DialogContent>
  </Dialog>
</template>
