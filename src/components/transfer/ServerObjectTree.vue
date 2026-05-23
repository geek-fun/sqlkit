<script setup lang="ts">
import type { ObjectSelection } from '@/types/transfer'
import { invoke } from '@tauri-apps/api/core'
import { computed, onMounted, ref } from 'vue'
import { useI18n } from 'vue-i18n'
import { Badge } from '@/components/ui/badge'
import { Button } from '@/components/ui/button'
import { Checkbox } from '@/components/ui/checkbox'

const props = defineProps<{
  connectionId: string
  selection: ObjectSelection
  disabled?: boolean
}>()

const emits = defineEmits<{
  (e: 'update:selection', value: ObjectSelection): void
  (e: 'change', value: ObjectSelection): void
}>()

const { t } = useI18n()

// Selection Sets
const selectedDBs = ref(new Set<string>(props.selection.databases))
const selectedSchemas = ref(new Set<string>())
const selectedTables = ref(new Set<string>())

// Initialize from selection
Object.entries(props.selection.schemas).forEach(([db, schemas]) => {
  schemas.forEach(s => selectedSchemas.value.add(`${db}.${s}`))
})
Object.entries(props.selection.tables).forEach(([dbSchema, tables]) => {
  tables.forEach(t => selectedTables.value.add(`${dbSchema}.${t}`))
})

type TreeNode = {
  id: string
  type: 'server' | 'database' | 'schema' | 'table'
  name: string
  db?: string
  schema?: string
  children?: TreeNode[]
  expanded: boolean
  loading: boolean
  loaded: boolean
}

const serverNode = ref<TreeNode>({
  id: props.connectionId,
  type: 'server',
  name: 'Server',
  expanded: true,
  loading: false,
  loaded: false,
  children: [],
})

async function loadDatabases() {
  if (serverNode.value.loaded)
    return
  serverNode.value.loading = true
  try {
    const dbs = await invoke<{ name: string, is_system: boolean }[]>('list_databases', { connectionId: props.connectionId })
    serverNode.value.children = dbs.filter(d => !d.is_system).map(d => ({
      id: d.name,
      type: 'database',
      name: d.name,
      db: d.name,
      expanded: false,
      loading: false,
      loaded: false,
      children: [],
    }))
    serverNode.value.loaded = true
  }
  finally {
    serverNode.value.loading = false
  }
}

async function loadSchemas(node: TreeNode) {
  if (node.loaded || node.type !== 'database')
    return
  node.loading = true
  try {
    const schemas = await invoke<string[]>('list_schemas', { connectionId: props.connectionId, database: node.db! })
    // If no schemas returned, maybe it's MySQL where DB = Schema.
    // In that case, we might need to load tables directly?
    // The spec says list_schemas returns string[]. If empty, we can just show no schemas, or maybe load tables?
    // Let's assume there's always at least a default schema or it returns schemas.
    node.children = schemas.map(s => ({
      id: `${node.db}.${s}`,
      type: 'schema',
      name: s,
      db: node.db,
      schema: s,
      expanded: false,
      loading: false,
      loaded: false,
      children: [],
    }))
    node.loaded = true
  }
  finally {
    node.loading = false
  }
}

async function loadTables(node: TreeNode) {
  if (node.loaded || node.type !== 'schema')
    return
  node.loading = true
  try {
    const tables = await invoke<{ name: string }[]>('list_tables', { connectionId: props.connectionId, database: node.db!, schema: node.schema! })
    node.children = tables.map(t => ({
      id: `${node.db}.${node.schema}.${t.name}`,
      type: 'table',
      name: t.name,
      db: node.db,
      schema: node.schema,
      expanded: false,
      loading: false,
      loaded: false,
    }))
    node.loaded = true
  }
  finally {
    node.loading = false
  }
}

async function toggleExpand(node: TreeNode) {
  node.expanded = !node.expanded
  if (node.expanded && !node.loaded) {
    if (node.type === 'server')
      await loadDatabases()
    else if (node.type === 'database')
      await loadSchemas(node)
    else if (node.type === 'schema')
      await loadTables(node)
  }
}

onMounted(async () => {
  await loadDatabases()
})

function getCheckState(node: TreeNode): 'checked' | 'unchecked' | 'indeterminate' {
  if (node.type === 'server') {
    if (serverNode.value.children?.length === 0)
      return 'unchecked'
    const allChecked = serverNode.value.children?.every(c => getCheckState(c) === 'checked')
    const anyChecked = serverNode.value.children?.some(c => getCheckState(c) !== 'unchecked')
    return allChecked ? 'checked' : anyChecked ? 'indeterminate' : 'unchecked'
  }

  if (node.type === 'database') {
    if (selectedDBs.value.has(node.name))
      return 'checked'
    // check if any schema/table under this DB is selected
    const hasSchema = Array.from(selectedSchemas.value).some(id => id.startsWith(`${node.name}.`))
    const hasTable = Array.from(selectedTables.value).some(id => id.startsWith(`${node.name}.`))
    if (hasSchema || hasTable)
      return 'indeterminate'
    return 'unchecked'
  }

  if (node.type === 'schema') {
    if (selectedDBs.value.has(node.db!))
      return 'checked'
    if (selectedSchemas.value.has(node.id))
      return 'checked'
    const hasTable = Array.from(selectedTables.value).some(id => id.startsWith(`${node.id}.`))
    if (hasTable)
      return 'indeterminate'
    return 'unchecked'
  }

  if (node.type === 'table') {
    if (selectedDBs.value.has(node.db!))
      return 'checked'
    if (selectedSchemas.value.has(`${node.db}.${node.schema}`))
      return 'checked'
    if (selectedTables.value.has(node.id))
      return 'checked'
    return 'unchecked'
  }

  return 'unchecked'
}

function updateSelection() {
  const selection: ObjectSelection = {
    serverId: props.connectionId,
    databases: Array.from(selectedDBs.value),
    schemas: {},
    tables: {},
  }

  Array.from(selectedSchemas.value).forEach((id) => {
    const [db, schema] = id.split('.')
    if (!selection.schemas[db])
      selection.schemas[db] = []
    selection.schemas[db].push(schema)
  })

  Array.from(selectedTables.value).forEach((id) => {
    const parts = id.split('.')
    const dbSchema = `${parts[0]}.${parts[1]}`
    const table = parts.slice(2).join('.')
    if (!selection.tables[dbSchema])
      selection.tables[dbSchema] = []
    selection.tables[dbSchema].push(table)
  })

  emits('update:selection', selection)
  emits('change', selection)
}

async function toggleCheck(node: TreeNode) {
  if (props.disabled)
    return

  const state = getCheckState(node)
  const isChecked = state === 'checked'

  if (node.type === 'database') {
    if (isChecked) {
      selectedDBs.value.delete(node.name)
      // Remove all descendants explicitly from partial sets
      Array.from(selectedSchemas.value).filter(id => id.startsWith(`${node.name}.`)).forEach(id => selectedSchemas.value.delete(id))
      Array.from(selectedTables.value).filter(id => id.startsWith(`${node.name}.`)).forEach(id => selectedTables.value.delete(id))
    }
    else {
      selectedDBs.value.add(node.name)
      // Cleanup redundant partial descendants
      Array.from(selectedSchemas.value).filter(id => id.startsWith(`${node.name}.`)).forEach(id => selectedSchemas.value.delete(id))
      Array.from(selectedTables.value).filter(id => id.startsWith(`${node.name}.`)).forEach(id => selectedTables.value.delete(id))
    }
  }
  else if (node.type === 'schema') {
    if (isChecked) {
      if (selectedDBs.value.has(node.db!)) {
        // Unchecking a schema when parent DB is fully checked
        selectedDBs.value.delete(node.db!)
        // Add all OTHER schemas to selectedSchemas
        if (!node.loaded)
          await loadSchemas(node)
        const parentDb = serverNode.value.children?.find(c => c.name === node.db!)
        parentDb?.children?.forEach((s) => {
          if (s.id !== node.id)
            selectedSchemas.value.add(s.id)
        })
      }
      else {
        selectedSchemas.value.delete(node.id)
        Array.from(selectedTables.value).filter(id => id.startsWith(`${node.id}.`)).forEach(id => selectedTables.value.delete(id))
      }
    }
    else {
      selectedSchemas.value.add(node.id)
      Array.from(selectedTables.value).filter(id => id.startsWith(`${node.id}.`)).forEach(id => selectedTables.value.delete(id))
      rollupSchema(node.db!)
    }
  }
  else if (node.type === 'table') {
    if (isChecked) {
      if (selectedDBs.value.has(node.db!)) {
        // Unchecking table when DB is checked
        selectedDBs.value.delete(node.db!)
        // Add all schemas except this one, and for this schema add all tables except this one
        const parentDb = serverNode.value.children?.find(c => c.name === node.db!)
        if (parentDb && !parentDb.loaded)
          await loadSchemas(parentDb)
        parentDb?.children?.forEach((s) => {
          if (s.id !== `${node.db}.${node.schema}`) {
            selectedSchemas.value.add(s.id)
          }
        })
        const parentSchema = parentDb?.children?.find(s => s.id === `${node.db}.${node.schema}`)
        if (parentSchema && !parentSchema.loaded)
          await loadTables(parentSchema)
        parentSchema?.children?.forEach((t) => {
          if (t.id !== node.id)
            selectedTables.value.add(t.id)
        })
      }
      else if (selectedSchemas.value.has(`${node.db}.${node.schema}`)) {
        // Unchecking table when schema is checked
        selectedSchemas.value.delete(`${node.db}.${node.schema}`)
        const parentDb = serverNode.value.children?.find(c => c.name === node.db!)
        const parentSchema = parentDb?.children?.find(s => s.id === `${node.db}.${node.schema}`)
        if (parentSchema && !parentSchema.loaded)
          await loadTables(parentSchema)
        parentSchema?.children?.forEach((t) => {
          if (t.id !== node.id)
            selectedTables.value.add(t.id)
        })
      }
      else {
        selectedTables.value.delete(node.id)
      }
    }
    else {
      selectedTables.value.add(node.id)
      rollupTable(node.db!, node.schema!)
    }
  }
  else if (node.type === 'server') {
    if (isChecked) {
      selectedDBs.value.clear()
      selectedSchemas.value.clear()
      selectedTables.value.clear()
    }
    else {
      serverNode.value.children?.forEach((db) => {
        selectedDBs.value.add(db.name)
      })
      selectedSchemas.value.clear()
      selectedTables.value.clear()
    }
  }

  updateSelection()
}

function rollupSchema(db: string) {
  const dbNode = serverNode.value.children?.find(c => c.name === db)
  if (!dbNode || !dbNode.loaded)
    return
  const allSchemasChecked = dbNode.children?.every(s => selectedSchemas.value.has(s.id))
  if (allSchemasChecked && (dbNode.children?.length ?? 0) > 0) {
    dbNode.children?.forEach(s => selectedSchemas.value.delete(s.id))
    selectedDBs.value.add(db)
  }
}

function rollupTable(db: string, schema: string) {
  const dbNode = serverNode.value.children?.find(c => c.name === db)
  const schemaNode = dbNode?.children?.find(c => c.schema === schema)
  if (!schemaNode || !schemaNode.loaded)
    return
  const allTablesChecked = schemaNode.children?.every(t => selectedTables.value.has(t.id))
  if (allTablesChecked && (schemaNode.children?.length ?? 0) > 0) {
    schemaNode.children?.forEach(t => selectedTables.value.delete(t.id))
    selectedSchemas.value.add(schemaNode.id)
    rollupSchema(db)
  }
}

function selectAll() {
  if (props.disabled)
    return
  serverNode.value.children?.forEach(db => selectedDBs.value.add(db.name))
  selectedSchemas.value.clear()
  selectedTables.value.clear()
  updateSelection()
}

function clearAll() {
  if (props.disabled)
    return
  selectedDBs.value.clear()
  selectedSchemas.value.clear()
  selectedTables.value.clear()
  updateSelection()
}

const totalSelected = computed(() => {
  // A naive count of explicit rules + implicit tables? The badge just needs an approximate or exact number
  // Let's sum explicit items:
  return selectedDBs.value.size + selectedSchemas.value.size + selectedTables.value.size
})
</script>

<template>
  <div class="text-sm text-foreground flex flex-col h-full space-y-2">
    <div class="pb-2 border-b border-border/40 flex items-center space-x-4">
      <Badge variant="secondary">
        {{ t('transfer.tree.nSelected', { n: totalSelected }) }}
      </Badge>
      <div class="flex-1" />
      <Button variant="ghost" size="sm" :disabled="disabled" @click="selectAll">
        {{ t('transfer.tree.selectAll') }}
      </Button>
      <Button variant="ghost" size="sm" :disabled="disabled" @click="clearAll">
        {{ t('transfer.tree.clearAll') }}
      </Button>
    </div>

    <div class="flex-1 min-h-0 overflow-y-auto" role="tree">
      <div v-if="!serverNode.loaded" class="py-2">
        <svg class="text-muted-foreground h-4 w-4 animate-spin" xmlns="http://www.w3.org/2000/svg" width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M21 12a9 9 0 1 1-6.219-8.56" /></svg>
      </div>
      <div v-else-if="serverNode.children?.length === 0" class="text-muted-foreground py-2 text-center">
        {{ t('transfer.tree.empty') }}
      </div>
      <div v-else>
        <!-- Recursive-like rendering using nested loops for simplicity and explicit control -->
        <div v-for="db in serverNode.children" :key="db.id" class="flex flex-col">
          <div class="group px-2 py-1.5 rounded-sm flex items-center hover:bg-muted/50">
            <button class="text-muted-foreground mr-1 p-0.5 hover:text-foreground" @click="toggleExpand(db)">
              <svg v-if="db.expanded" class="h-4 w-4" xmlns="http://www.w3.org/2000/svg" width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><polyline points="6 9 12 15 18 9" /></svg>
              <svg v-else class="h-4 w-4" xmlns="http://www.w3.org/2000/svg" width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><polyline points="9 18 15 12 9 6" /></svg>
            </button>
            <Checkbox
              :checked="getCheckState(db) === 'checked' ? true : getCheckState(db) === 'indeterminate' ? 'indeterminate' : false"
              :disabled="disabled"
              @update:checked="toggleCheck(db)"
            />
            <svg class="text-primary ml-2 mr-2 h-4 w-4" xmlns="http://www.w3.org/2000/svg" width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><ellipse cx="12" cy="5" rx="9" ry="3" /><path d="M21 12c0 1.66-4 3-9 3s-9-1.34-9-3" /><path d="M3 5v14c0 1.66 4 3 9 3s9-1.34 9-3V5" /></svg>
            <span class="flex-1 select-none truncate">{{ db.name }}</span>
            <svg v-if="db.loading" class="text-muted-foreground ml-2 h-3 w-3 animate-spin" xmlns="http://www.w3.org/2000/svg" width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M21 12a9 9 0 1 1-6.219-8.56" /></svg>
          </div>

          <div v-if="db.expanded" class="ml-6 pl-2 border-l border-border/40">
            <div v-if="db.children?.length === 0 && db.loaded" class="text-xs text-muted-foreground py-1">
              {{ t('transfer.tree.empty') }}
            </div>
            <div v-for="schema in db.children" :key="schema.id" class="flex flex-col">
              <div class="group px-2 py-1.5 rounded-sm flex items-center hover:bg-muted/50">
                <button class="text-muted-foreground mr-1 p-0.5 hover:text-foreground" @click="toggleExpand(schema)">
                  <svg v-if="schema.expanded" class="h-4 w-4" xmlns="http://www.w3.org/2000/svg" width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><polyline points="6 9 12 15 18 9" /></svg>
                  <svg v-else class="h-4 w-4" xmlns="http://www.w3.org/2000/svg" width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><polyline points="9 18 15 12 9 6" /></svg>
                </button>
                <Checkbox
                  :checked="getCheckState(schema) === 'checked' ? true : getCheckState(schema) === 'indeterminate' ? 'indeterminate' : false"
                  :disabled="disabled"
                  @update:checked="toggleCheck(schema)"
                />
                <svg class="text-primary/80 ml-2 mr-2 h-4 w-4" xmlns="http://www.w3.org/2000/svg" width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M22 19a2 2 0 0 1-2 2H4a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h5l2 3h9a2 2 0 0 1 2 2z" /></svg>
                <span class="flex-1 select-none truncate">{{ schema.name }}</span>
                <svg v-if="schema.loading" class="text-muted-foreground ml-2 h-3 w-3 animate-spin" xmlns="http://www.w3.org/2000/svg" width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M21 12a9 9 0 1 1-6.219-8.56" /></svg>
              </div>

              <div v-if="schema.expanded" class="ml-6 pl-2 border-l border-border/40">
                <div v-if="schema.children?.length === 0 && schema.loaded" class="text-xs text-muted-foreground py-1">
                  {{ t('transfer.tree.empty') }}
                </div>
                <!-- VIRTUALIZED TABLES LIST -->
                <div v-if="schema.children && schema.children.length > 0" class="max-h-[300px] overflow-y-auto">
                  <div v-for="table in schema.children" :key="table.id" class="px-2 py-1 rounded-sm flex items-center hover:bg-muted/50">
                    <div class="mr-1 w-5" /> <!-- Spacer for alignment -->
                    <Checkbox
                      :checked="getCheckState(table) === 'checked'"
                      :disabled="disabled"
                      @update:checked="toggleCheck(table)"
                    />
                    <svg class="text-muted-foreground ml-2 mr-2 h-4 w-4" xmlns="http://www.w3.org/2000/svg" width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><rect x="3" y="3" width="18" height="18" rx="2" ry="2" /><line x1="3" y1="9" x2="21" y2="9" /><line x1="9" y1="21" x2="9" y2="9" /></svg>
                    <span class="text-xs flex-1 select-none truncate">{{ table.name }}</span>
                  </div>
                </div>
              </div>
            </div>
          </div>
        </div>
      </div>
    </div>
  </div>
</template>
