<script setup lang="ts">
import type { ForeignKeyInfo } from '@/datasources/erDiagramApi'
import type { ColumnInfo } from '@/types/connection'
import { invoke } from '@tauri-apps/api/core'
import { useResizeObserver } from '@vueuse/core'
import dagre from 'dagre'
import { computed, onMounted, onUnmounted, ref, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import {
  AlertDialog,
  AlertDialogAction,
  AlertDialogCancel,
  AlertDialogContent,
  AlertDialogFooter,
  AlertDialogHeader,
  AlertDialogTitle,
} from '@/components/ui/alert-dialog'
import { Badge } from '@/components/ui/badge'
import { Button } from '@/components/ui/button'
import { Input } from '@/components/ui/input'
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from '@/components/ui/select'
import { Spinner } from '@/components/ui/spinner'
import { getForeignKeys } from '@/datasources/erDiagramApi'
import { useDatabaseStore } from '@/store/databaseStore'

// ─── Props ───────────────────────────────────────────
const props = defineProps<{
  connectionId: string
  database: string
  schema?: string
}>()

const { t } = useI18n()

// ─── Types ────────────────────────────────────────────
type TableInfo = {
  name: string
  schema?: string
  table_type?: string
}

type TableData = {
  name: string
  schema?: string
  columns: ColumnInfo[]
  foreignKeys: ForeignKeyInfo[]
}

type RenderNode = {
  id: string
  x: number
  y: number
  width: number
  height: number
  table: TableData
  isHighlighted: boolean
  visibleColumns: ColumnInfo[]
  showExpandButton: boolean
  isExpanded: boolean
}

type RenderEdge = {
  from: string
  to: string
  label: string
  path: string
  isHighlighted: boolean
}

// ─── Layout Constants ─────────────────────────────────
const NODE_WIDTH = 220
const COL_HEIGHT = 28
const HEADER_HEIGHT = 36
const EXPAND_BTN_HEIGHT = 30
const CARD_PADDING = 8
const ROUTE_PADDING = 56
const ROUTE_BLOCK_MARGIN = 18

function calcNodeHeight(table: TableData, isExpanded: boolean): number {
  const colCount = isExpanded
    ? table.columns.length
    : Math.min(5, table.columns.length)
  const expandBtn = table.columns.length > 5 ? EXPAND_BTN_HEIGHT : 0
  return HEADER_HEIGHT + colCount * COL_HEIGHT + expandBtn + CARD_PADDING
}

// ─── State ────────────────────────────────────────────
const tables = ref<TableData[]>([])
const foreignKeys = ref<ForeignKeyInfo[]>([])
const nodePositions = ref<Map<string, { x: number, y: number }>>(new Map())
const edgePaths = ref<
  Array<{ from: string, to: string, label: string, path: string }>
>([])

const selectedTableId = ref<string | null>(null)
const searchQuery = ref('')
const expandedTables = ref<Set<string>>(new Set())
const layoutDirection = ref<'TB' | 'LR'>('TB')

const zoomLevel = ref(1)
const panOffset = ref({ x: 0, y: 0 })
const isPanning = ref(false)
const panStart = ref({ x: 0, y: 0 })
const gestureStartZoom = ref(1)

const loading = ref(true)
const error = ref<string | null>(null)
const showWarning = ref(false)
const svgContainerRef = ref<HTMLElement | null>(null)

// ─── Node drag state (delta-offset, no nodePositions update during drag) ───
const draggingNodeId = ref<string | null>(null)
const dragStartPos = ref({ x: 0, y: 0 })
const dragNodeStart = ref({ x: 0, y: 0 })
const dragDelta = ref({ x: 0, y: 0 })

// ─── Schema selector ──────────────────────────────
const databaseStore = useDatabaseStore()
const availableSchemas = ref<string[]>([])
const localSchema = ref<string>(props.schema ?? '__all__')

const supportsSchemas = computed(() =>
  availableSchemas.value.length > 0,
)

const schemaParam = computed(() =>
  localSchema.value === '__all__' ? null : localSchema.value,
)

watch(() => props.schema, (val) => {
  if (val !== undefined)
    localSchema.value = val ?? '__all__'
})

// ─── Relationships from foreign keys ──────────────
const allRelationships = computed(() =>
  foreignKeys.value.map(fk => ({
    sourceTable: fk.source_table,
    targetTable: fk.referenced_table,
  })),
)

// ─── Focus mode ───────────────────────────────────
const focusMode = computed(() => selectedTableId.value !== null)

const displayedTables = computed(() => {
  let result = tables.value
  if (searchQuery.value.trim()) {
    const q = searchQuery.value.toLowerCase()
    result = result.filter(t => t.name.toLowerCase().includes(q))
  }
  if (focusMode.value && selectedTableId.value) {
    const related = new Set<string>([selectedTableId.value])
    for (const rel of allRelationships.value) {
      if (rel.sourceTable === selectedTableId.value)
        related.add(rel.targetTable)
      if (rel.targetTable === selectedTableId.value)
        related.add(rel.sourceTable)
    }
    result = result.filter(t => related.has(t.name))
  }
  return result
})

const displayedTableNames = computed(
  () => new Set(displayedTables.value.map(t => t.name)),
)

const displayedRelationships = computed(() =>
  allRelationships.value.filter(
    rel =>
      displayedTableNames.value.has(rel.sourceTable)
      && displayedTableNames.value.has(rel.targetTable),
  ),
)

const highlightedTables = computed(() => {
  if (!selectedTableId.value)
    return new Set<string>()
  const connected = new Set<string>([selectedTableId.value])
  for (const rel of allRelationships.value) {
    if (rel.sourceTable === selectedTableId.value)
      connected.add(rel.targetTable)
    if (rel.targetTable === selectedTableId.value)
      connected.add(rel.sourceTable)
  }
  return connected
})

const renderNodes = computed<RenderNode[]>(() => {
  return displayedTables.value
    .map((table) => {
      const pos = nodePositions.value.get(table.name)
      if (!pos)
        return null

      const isExpanded = expandedTables.value.has(table.name)
      const visibleCols = isExpanded
        ? table.columns
        : table.columns.slice(0, 5)
      const height = calcNodeHeight(table, isExpanded)

      return {
        id: table.name,
        x: pos.x - NODE_WIDTH / 2,
        y: pos.y - height / 2,
        width: NODE_WIDTH,
        height,
        table,
        isHighlighted: highlightedTables.value.has(table.name),
        visibleColumns: visibleCols,
        showExpandButton: table.columns.length > 5,
        isExpanded,
      }
    })
    .filter((n): n is RenderNode => n !== null)
})

// ─── Orthogonal path routing ──────────────────────
type TableRect = {
  name: string
  x: number
  y: number
  width: number
  height: number
}

function getTableRect(tableName: string): TableRect | null {
  const table = tables.value.find(t => t.name === tableName)
  const pos = nodePositions.value.get(tableName)
  if (!table || !pos)
    return null
  const height = calcNodeHeight(table, expandedTables.value.has(tableName))
  return {
    name: tableName,
    x: pos.x - NODE_WIDTH / 2,
    y: pos.y - height / 2,
    width: NODE_WIDTH,
    height,
  }
}

function rangesOverlap(a1: number, a2: number, b1: number, b2: number): boolean {
  return Math.max(a1, b1) <= Math.min(a2, b2)
}

function routeSideX(rect: TableRect, routeX: number, offset = 0): number {
  if (routeX < rect.x)
    return rect.x - offset
  return rect.x + rect.width + offset
}

function tableRects(): TableRect[] {
  return displayedTables.value
    .map(t => getTableRect(t.name))
    .filter((r): r is TableRect => r !== null)
}

function isVerticalRouteBlocked(routeX: number, y1: number, y2: number, ignored: Set<string>): boolean {
  const top = Math.min(y1, y2)
  const bottom = Math.max(y1, y2)
  return tableRects().some(
    r =>
      !ignored.has(r.name)
      && routeX >= r.x - ROUTE_BLOCK_MARGIN
      && routeX <= r.x + r.width + ROUTE_BLOCK_MARGIN
      && rangesOverlap(top, bottom, r.y - ROUTE_BLOCK_MARGIN, r.y + r.height + ROUTE_BLOCK_MARGIN),
  )
}

function isHorizontalRouteBlocked(y: number, x1: number, x2: number, ignored: Set<string>): boolean {
  const left = Math.min(x1, x2)
  const right = Math.max(x1, x2)
  return tableRects().some(
    r =>
      !ignored.has(r.name)
      && y >= r.y - ROUTE_BLOCK_MARGIN
      && y <= r.y + r.height + ROUTE_BLOCK_MARGIN
      && rangesOverlap(left, right, r.x - ROUTE_BLOCK_MARGIN, r.x + r.width + ROUTE_BLOCK_MARGIN),
  )
}

function candidateRouteXs(source: TableRect, target: TableRect): number[] {
  const candidates = new Set<number>()
  const minLeft = Math.min(source.x, target.x)
  const maxRight = Math.max(source.x + source.width, target.x + target.width)

  candidates.add(minLeft - ROUTE_PADDING)
  candidates.add(maxRight + ROUTE_PADDING)

  if (source.x + source.width + ROUTE_PADDING <= target.x)
    candidates.add((source.x + source.width + target.x) / 2)
  if (target.x + target.width + ROUTE_PADDING <= source.x)
    candidates.add((target.x + target.width + source.x) / 2)

  const columns = [...new Set(tableRects().map(r => r.x))].sort((a, b) => a - b)
  for (let i = 0; i < columns.length - 1; i++) {
    const gap = columns[i + 1] - (columns[i] + NODE_WIDTH)
    if (gap >= ROUTE_PADDING)
      candidates.add((columns[i] + NODE_WIDTH + columns[i + 1]) / 2)
  }

  return [...candidates].sort((a, b) => {
    const sa = routeSideX(source, a)
    const ta = routeSideX(target, a)
    const sb = routeSideX(source, b)
    const tb = routeSideX(target, b)
    return Math.abs(a - sa) + Math.abs(a - ta) - (Math.abs(b - sb) + Math.abs(b - tb))
  })
}

function computeRelationshipPath(sourceTable: string, targetTable: string): string {
  const source = getTableRect(sourceTable)
  const target = getTableRect(targetTable)
  if (!source || !target)
    return ''

  const y1 = source.y + source.height / 2
  const y2 = target.y + target.height / 2
  const ignored = new Set([source.name, target.name])
  const candidates = candidateRouteXs(source, target)

  const routeX
    = candidates.find((c) => {
      const x1 = routeSideX(source, c)
      const x2 = routeSideX(target, c)
      return (
        !isVerticalRouteBlocked(c, y1, y2, ignored)
        && !isHorizontalRouteBlocked(y1, x1, c, ignored)
        && !isHorizontalRouteBlocked(y2, c, x2, ignored)
      )
    })
    ?? candidates[0]
    ?? Math.max(source.x + source.width, target.x + target.width) + ROUTE_PADDING

  const x1 = routeSideX(source, routeX, 2)
  const x2 = routeSideX(target, routeX, 2)
  return `M ${x1} ${y1} L ${routeX} ${y1} L ${routeX} ${y2} L ${x2} ${y2}`
}

const renderEdges = computed<RenderEdge[]>(() => {
  return displayedRelationships.value.map((rel) => {
    const path = computeRelationshipPath(rel.sourceTable, rel.targetTable)
    return {
      from: rel.sourceTable,
      to: rel.targetTable,
      label: '',
      path,
      isHighlighted:
        selectedTableId.value !== null
        && (rel.sourceTable === selectedTableId.value
          || rel.targetTable === selectedTableId.value),
    }
  })
})

// ─── Layout Computation ───────────────────────────────
function computeLayout() {
  if (displayedTables.value.length === 0) {
    nodePositions.value = new Map()
    edgePaths.value = []
    return
  }

  const g = new dagre.graphlib.Graph()
  g.setGraph({
    rankdir: layoutDirection.value,
    nodesep: 80,
    ranksep: 120,
    marginx: 40,
    marginy: 40,
  })
  g.setDefaultEdgeLabel(() => ({}))

  for (const table of displayedTables.value) {
    const isExpanded = expandedTables.value.has(table.name)
    const height = calcNodeHeight(table, isExpanded)
    g.setNode(table.name, {
      label: table.name,
      width: NODE_WIDTH,
      height,
    })
  }

  for (const rel of displayedRelationships.value) {
    g.setEdge(rel.sourceTable, rel.targetTable, { label: '' })
  }

  dagre.layout(g)

  const positions = new Map<string, { x: number, y: number }>()
  g.nodes().forEach((nodeName: string) => {
    const node = g.node(nodeName) as { x: number, y: number }
    positions.set(nodeName, { x: node.x, y: node.y })
  })

  nodePositions.value = positions
  edgePaths.value = []
}

// ─── Data Fetching ────────────────────────────────────
async function fetchAvailableSchemas() {
  if (!props.connectionId)
    return
  try {
    const meta = databaseStore.metadata[props.connectionId]
    const cached = meta?.schemas[props.database]
    if (cached && cached.length > 0) {
      availableSchemas.value = cached
      return
    }
    const result = await invoke<string[]>('list_schemas', {
      connectionId: props.connectionId,
      database: props.database,
    })
    availableSchemas.value = result ?? []
    const m = databaseStore.metadata[props.connectionId]
    if (m && result)
      m.schemas[props.database] = result
  }
  catch {
    availableSchemas.value = []
  }
}

async function fetchSchemaData() {
  loading.value = true
  try {
    const schema = schemaParam.value

    const [tableList, fkList] = await Promise.all([
      invoke<TableInfo[]>('list_tables', {
        connectionId: props.connectionId,
        database: props.database,
        schema,
      }),
      getForeignKeys(props.connectionId, props.database, schema)
        .catch(() => [] as ForeignKeyInfo[]),
    ])

    foreignKeys.value = fkList

    const columnResults = await Promise.all(
      tableList.map(t =>
        invoke<ColumnInfo[]>('list_columns', {
          connectionId: props.connectionId,
          database: props.database,
          schema,
          tableName: t.name,
        }).catch(() => [] as ColumnInfo[]),
      ),
    )

    tables.value = tableList.map((t, i) => ({
      name: t.name,
      schema: t.schema,
      columns: columnResults[i],
      foreignKeys: fkList.filter(
        fk => fk.source_table === t.name || fk.referenced_table === t.name,
      ),
    }))

    if (tables.value.length > 30)
      showWarning.value = true

    computeLayout()
  }
  catch (err) {
    error.value = `Failed to load ER diagram: ${err instanceof Error ? err.message : String(err)}`
    console.error('Failed to fetch ER diagram data:', err)
  }
  finally {
    loading.value = false
  }
}

// ─── Watchers ─────────────────────────────────────────
watch(
  [displayedTables, layoutDirection, expandedTables],
  () => {
    computeLayout()
  },
  { immediate: false },
)

watch(localSchema, () => {
  fetchSchemaData()
})

// ─── Interaction Handlers ─────────────────────────────
function startPan(e: MouseEvent) {
  const target = e.target as HTMLElement
  const isCanvasClick
    = target === svgContainerRef.value
      || target.tagName === 'svg'
  if (!isCanvasClick)
    return

  isPanning.value = true
  panStart.value = {
    x: e.clientX - panOffset.value.x,
    y: e.clientY - panOffset.value.y,
  }
}

function onPan(e: MouseEvent) {
  if (!isPanning.value)
    return
  panOffset.value = {
    x: e.clientX - panStart.value.x,
    y: e.clientY - panStart.value.y,
  }
}

function endPan() {
  isPanning.value = false
}

// ─── Node Drag Handlers (delta-offset: nodePositions only updated on drop) ──
function onNodeMouseDown(e: MouseEvent, nodeId: string) {
  if (e.button !== 0)
    return
  const pos = nodePositions.value.get(nodeId)
  if (!pos)
    return
  draggingNodeId.value = nodeId
  dragStartPos.value = { x: e.clientX, y: e.clientY }
  dragNodeStart.value = { x: pos.x, y: pos.y }
  dragDelta.value = { x: 0, y: 0 }
  document.addEventListener('mousemove', onNodeMouseMove)
  document.addEventListener('mouseup', onNodeMouseUp)
}

function onNodeMouseMove(e: MouseEvent) {
  if (!draggingNodeId.value)
    return
  dragDelta.value = {
    x: (e.clientX - dragStartPos.value.x) / zoomLevel.value,
    y: (e.clientY - dragStartPos.value.y) / zoomLevel.value,
  }
}

function onNodeMouseUp() {
  if (draggingNodeId.value) {
    const finalX = dragNodeStart.value.x + dragDelta.value.x
    const finalY = dragNodeStart.value.y + dragDelta.value.y
    const positions = new Map(nodePositions.value)
    positions.set(draggingNodeId.value, { x: finalX, y: finalY })
    nodePositions.value = positions
  }
  draggingNodeId.value = null
  dragDelta.value = { x: 0, y: 0 }
  document.removeEventListener('mousemove', onNodeMouseMove)
  document.removeEventListener('mouseup', onNodeMouseUp)
}

// ─── Zoom ─────────────────────────────────────────────
function onWheel(e: WheelEvent) {
  if (!e.ctrlKey && !e.metaKey) {
    // Without ctrl/meta: pan vertically
    panOffset.value = { ...panOffset.value, y: panOffset.value.y - e.deltaY }
    return
  }
  e.preventDefault()
  const delta = -e.deltaY / 500
  const nextZoom = Math.max(0.1, Math.min(3, +(zoomLevel.value * (1 + delta)).toFixed(2)))

  // Zoom centered on mouse position
  const rect = svgContainerRef.value?.getBoundingClientRect()
  if (rect) {
    const originX = e.clientX - rect.left
    const originY = e.clientY - rect.top
    const contentX = (originX - panOffset.value.x) / zoomLevel.value
    const contentY = (originY - panOffset.value.y) / zoomLevel.value
    zoomLevel.value = nextZoom
    panOffset.value = {
      x: originX - contentX * nextZoom,
      y: originY - contentY * nextZoom,
    }
  }
  else {
    zoomLevel.value = nextZoom
  }
}

function onGestureStart(e: Event) {
  e.preventDefault()
  gestureStartZoom.value = zoomLevel.value
}

function onGestureChange(e: Event) {
  const ge = e as WheelEvent & { scale?: number }
  if (typeof ge.scale !== 'number')
    return
  e.preventDefault()
  zoomLevel.value = Math.max(0.1, Math.min(3, +(gestureStartZoom.value * ge.scale).toFixed(2)))
}

function fitToScreen() {
  zoomLevel.value = 1
  panOffset.value = { x: 0, y: 0 }
}

function selectTable(tableName: string) {
  selectedTableId.value
    = selectedTableId.value === tableName ? null : tableName
}

function deselectAll() {
  selectedTableId.value = null
}

function toggleExpand(tableName: string) {
  const next = new Set(expandedTables.value)
  if (next.has(tableName))
    next.delete(tableName)
  else
    next.add(tableName)
  expandedTables.value = next
}

function toggleLayout() {
  layoutDirection.value
    = layoutDirection.value === 'TB' ? 'LR' : 'TB'
}

// ─── Resize Observer ──────────────────────────────────
useResizeObserver(svgContainerRef, () => {
  // Dagre layout is independent of container size
})

// ─── Helpers ──────────────────────────────────────────
function isFKColumn(table: TableData, colName: string): boolean {
  return table.foreignKeys.some(
    fk => fk.columns.includes(colName) || fk.referenced_columns.includes(colName),
  )
}

// ─── Lifecycle ────────────────────────────────────────
onMounted(async () => {
  await fetchAvailableSchemas()
  fetchSchemaData()
})

onUnmounted(() => {
  document.removeEventListener('mousemove', onNodeMouseMove)
  document.removeEventListener('mouseup', onNodeMouseUp)
})
</script>

<template>
  <div class="er-diagram-view bg-background flex flex-col h-full">
    <!-- ── Toolbar ──────────────────────────────────── -->
    <div class="px-3 py-2 border-b bg-muted/30 flex gap-2 items-center">
      <!-- Schema selector -->
      <div v-if="supportsSchemas" class="w-44">
        <Select v-model="localSchema">
          <SelectTrigger class="text-xs h-8">
            <SelectValue :placeholder="t('components.databaseBrowser.erDiagram.allSchemas')" />
          </SelectTrigger>
          <SelectContent :side-offset="4" align="start" class="z-[100]">
            <SelectItem value="__all__" class="text-xs">
              {{ t('components.databaseBrowser.erDiagram.allSchemas') }}
            </SelectItem>
            <SelectItem
              v-for="s in availableSchemas"
              :key="s"
              :value="s"
              class="text-xs font-mono"
            >
              {{ s }}
            </SelectItem>
          </SelectContent>
        </Select>
      </div>

      <!-- Search -->
      <div class="flex-1 max-w-xs relative">
        <svg
          xmlns="http://www.w3.org/2000/svg" width="14" height="14" viewBox="0 0 24 24"
          fill="none" stroke="currentColor" stroke-width="2"
          stroke-linecap="round" stroke-linejoin="round"
          class="text-muted-foreground left-2 top-1/2 absolute -translate-y-1/2"
        >
          <circle cx="11" cy="11" r="8" /><path d="m21 21-4.3-4.3" />
        </svg>
        <Input
          v-model="searchQuery"
          :placeholder="t('components.databaseBrowser.erDiagram.searchPlaceholder')"
          class="text-xs pl-7 h-8"
        />
      </div>

      <!-- Info badges -->
      <Badge variant="secondary" class="text-[11px] shrink-0 gap-1 h-6" :title="t('components.databaseBrowser.erDiagram.tablesCountBadge', { count: displayedTables.length })">
        <svg xmlns="http://www.w3.org/2000/svg" width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M3 3h18v18H3z" /><path d="M21 9H3" /><path d="M9 21V9" /></svg>
        {{ displayedTables.length }}
      </Badge>
      <Badge variant="secondary" class="text-[11px] shrink-0 gap-1 h-6" :title="t('components.databaseBrowser.erDiagram.relationshipsCountBadge', { count: displayedRelationships.length })">
        <svg xmlns="http://www.w3.org/2000/svg" width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M10 13a5 5 0 0 0 7.54.54l3-3a5 5 0 0 0-7.07-7.07l-1.72 1.71" /><path d="M14 11a5 5 0 0 0-7.54-.54l-3 3a5 5 0 0 0 7.07 7.07l1.71-1.71" /></svg>
        {{ displayedRelationships.length }}
      </Badge>
      <div class="flex-1" />

      <!-- Layout direction toggle -->
      <Button
        variant="ghost" size="sm"
        class="text-xs gap-1 h-7"
        :title="t('components.databaseBrowser.erDiagram.layoutDirection')"
        @click="toggleLayout"
      >
        <svg xmlns="http://www.w3.org/2000/svg" width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><rect x="3" y="3" width="7" height="7" /><rect x="14" y="3" width="7" height="7" /><rect x="14" y="14" width="7" height="7" /><rect x="3" y="14" width="7" height="7" /></svg>
        {{
          layoutDirection === 'TB'
            ? t('components.databaseBrowser.erDiagram.layoutLeftRight')
            : t('components.databaseBrowser.erDiagram.layoutTopBottom')
        }}
      </Button>

      <!-- Fit to screen -->
      <Button
        variant="ghost" size="icon" class="h-7 w-7"
        :title="t('components.databaseBrowser.erDiagram.fitToScreen')"
        @click="fitToScreen"
      >
        <svg xmlns="http://www.w3.org/2000/svg" width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M8 3H5a2 2 0 0 0-2 2v3" /><path d="M21 8V5a2 2 0 0 0-2-2h-3" /><path d="M16 21h3a2 2 0 0 0 2-2v-3" /><path d="M3 16v3a2 2 0 0 0 2 2h3" /></svg>
      </Button>

      <!-- Refresh -->
      <Button
        variant="ghost" size="icon" class="h-7 w-7"
        :title="t('common.buttons.refresh')"
        :disabled="loading"
        @click="fetchSchemaData"
      >
        <svg xmlns="http://www.w3.org/2000/svg" width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" :class="{ 'animate-spin': loading }"><path d="M3 12a9 9 0 0 1 9-9 9.75 9.75 0 0 1 6.74 2.74L21 8" /><path d="M21 3v5h-5" /><path d="M21 12a9 9 0 0 1-9 9 9.75 9.75 0 0 1-6.74-2.74L3 16" /><path d="M8 16H3v5" /></svg>
      </Button>

      <!-- Zoom controls -->
      <div class="flex gap-0.5 items-center">
        <Button variant="ghost" size="icon" class="h-7 w-7" :disabled="zoomLevel <= 0.1" :title="t('components.databaseBrowser.erDiagram.zoomOut')" @click="zoomLevel = Math.max(0.1, +(zoomLevel / 1.3).toFixed(2))">
          <svg xmlns="http://www.w3.org/2000/svg" width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><circle cx="11" cy="11" r="8" /><line x1="21" y1="21" x2="16.65" y2="16.65" /><line x1="8" y1="11" x2="14" y2="11" /></svg>
        </Button>
        <Button variant="ghost" size="icon" class="h-7 w-7" :disabled="zoomLevel >= 3" :title="t('components.databaseBrowser.erDiagram.zoomIn')" @click="zoomLevel = Math.min(3, +(zoomLevel * 1.3).toFixed(2))">
          <svg xmlns="http://www.w3.org/2000/svg" width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><circle cx="11" cy="11" r="8" /><line x1="21" y1="21" x2="16.65" y2="16.65" /><line x1="11" y1="8" x2="11" y2="14" /><line x1="8" y1="11" x2="14" y2="11" /></svg>
        </Button>
        <span class="text-xs text-muted-foreground text-center min-w-[3rem] tabular-nums">
          {{ t('components.databaseBrowser.erDiagram.zoom', { percentage: Math.round(zoomLevel * 100) }) }}
        </span>
      </div>
    </div>

    <!-- ── Large Schema Warning ─────────────────────── -->
    <AlertDialog v-model:open="showWarning">
      <AlertDialogContent>
        <AlertDialogHeader>
          <AlertDialogTitle>
            {{ t('components.databaseBrowser.erDiagram.largeSchemaWarning', { count: tables.length }) }}
          </AlertDialogTitle>
        </AlertDialogHeader>
        <AlertDialogFooter>
          <AlertDialogCancel @click="showWarning = false">
            {{ t('common.buttons.cancel') }}
          </AlertDialogCancel>
          <AlertDialogAction @click="showWarning = false">
            {{ t('common.buttons.continue') }}
          </AlertDialogAction>
        </AlertDialogFooter>
      </AlertDialogContent>
    </AlertDialog>

    <!-- ── Canvas Area ──────────────────────────────── -->
    <div
      ref="svgContainerRef"
      class="flex-1 cursor-grab relative overflow-hidden"
      @mousedown="startPan"
      @mousemove="onPan"
      @mouseup="endPan"
      @mouseleave="endPan"
      @wheel.prevent="onWheel"
      @gesturestart="onGestureStart"
      @gesturechange="onGestureChange"
    >
      <!-- Loading -->
      <div v-if="loading" class="flex gap-2 items-center inset-0 justify-center absolute">
        <Spinner class="h-6 w-6" />
        <span class="text-sm text-muted-foreground">{{ t('components.databaseBrowser.erDiagram.loading') }}</span>
      </div>

      <!-- Error state -->
      <div v-else-if="error" class="flex flex-col gap-2 items-center inset-0 justify-center absolute">
        <p class="text-sm text-destructive">
          {{ error }}
        </p>
        <Button variant="outline" size="sm" @click="fetchSchemaData">
          {{ t('common.buttons.retry') }}
        </Button>
      </div>

      <!-- No tables -->
      <div v-else-if="tables.length === 0" class="flex items-center inset-0 justify-center absolute">
        <p class="text-sm text-muted-foreground">
          {{ t('components.databaseBrowser.erDiagram.noForeignKeys') }}
        </p>
      </div>

      <!-- Empty search / focus results -->
      <div v-else-if="displayedTables.length === 0" class="flex items-center inset-0 justify-center absolute">
        <p class="text-sm text-muted-foreground">
          {{ t('components.databaseBrowser.erDiagram.searchPlaceholder') }}
        </p>
      </div>

      <!-- SVG Canvas -->
      <svg
        v-show="!loading && displayedTables.length > 0"
        class="h-full w-full"
        :style="{
          transform: `scale(${zoomLevel}) translate(${panOffset.x}px, ${panOffset.y}px)`,
          transformOrigin: '0 0',
          overflow: 'visible',
        }"
        @click.self="deselectAll"
        @dblclick="fitToScreen"
      >
        <defs>
          <marker id="er-arrowhead" markerWidth="10" markerHeight="7" refX="9" refY="3.5" orient="auto">
            <polygon points="0 0, 10 3.5, 0 7" fill="hsl(var(--muted-foreground))" />
          </marker>
        </defs>

        <!-- Relationship lines -->
        <g v-for="(edge, idx) in renderEdges" :key="`edge-${idx}`">
          <path
            :d="edge.path"
            fill="none"
            :stroke="edge.isHighlighted ? 'hsl(var(--primary))' : 'hsl(var(--muted-foreground))'"
            :stroke-width="edge.isHighlighted ? 2 : 1"
            :stroke-dasharray="edge.label ? '4 3' : 'none'"
            marker-end="url(#er-arrowhead)"
          />
        </g>

        <!-- Table cards -->
        <g
          v-for="node in renderNodes"
          :key="node.id"
          :transform="`translate(${node.x + (draggingNodeId === node.id ? dragDelta.x : 0)}, ${node.y + (draggingNodeId === node.id ? dragDelta.y : 0)})`"
          class="er-table-group"
          :class="{ 'er-table-group--dragging': draggingNodeId === node.id }"
          @mousedown.prevent="onNodeMouseDown($event, node.id)"
          @click.stop="selectTable(node.id)"
        >
          <foreignObject :width="node.width" :height="node.height">
            <div
              class="er-table-card"
              :class="{
                'er-table-card--selected': selectedTableId === node.id,
              }"
            >
              <!-- Table header -->
              <div class="er-table-header">{{ node.id }}</div>

              <!-- Columns -->
              <div class="er-table-columns">
                <div
                  v-for="(col, colIdx) in node.visibleColumns"
                  :key="colIdx"
                  class="er-table-column"
                >
                  <span class="er-column-markers">
                    <span v-if="col.is_primary_key" class="er-pk">
                      <svg xmlns="http://www.w3.org/2000/svg" width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M2.586 17.414A2 2 0 0 0 2 18.828V21a1 1 0 0 0 1 1h3a1 1 0 0 0 1-1v-1a1 1 0 0 1 1-1h1a1 1 0 0 0 1-1v-1a1 1 0 0 1 1-1h.172a2 2 0 0 0 1.414-.586l.814-.814a6.5 6.5 0 1 0-4-4z" /><circle cx="16.5" cy="7.5" r=".5" fill="currentColor" /></svg>
                    </span>
                    <span v-else-if="isFKColumn(node.table, col.name)" class="er-fk">
                      <svg xmlns="http://www.w3.org/2000/svg" width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M10 13a5 5 0 0 0 7.54.54l3-3a5 5 0 0 0-7.07-7.07l-1.72 1.71" /><path d="M14 11a5 5 0 0 0-7.54-.54l-3 3a5 5 0 0 0 7.07 7.07l1.71-1.71" /></svg>
                    </span>
                    <span v-else class="er-col-spacer" />
                  </span>
                  <span class="er-column-name">{{ col.name }}</span>
                  <span class="er-column-type">{{ col.data_type }}</span>
                </div>
              </div>

              <!-- Show more / less toggle -->
              <button
                v-if="node.showExpandButton"
                class="er-expand-btn whitespace-nowrap"
                @click.stop="toggleExpand(node.id)"
              >
                {{
                  node.isExpanded
                    ? t('components.databaseBrowser.erDiagram.hideExtraColumns')
                    : t('components.databaseBrowser.erDiagram.showAllColumns', { count: node.table.columns.length })
                }}
              </button>
            </div>
          </foreignObject>
        </g>
      </svg>
    </div>
  </div>
</template>

<style scoped>
.er-table-group {
  cursor: grab;
}
.er-table-group--dragging {
  cursor: grabbing;
}
.er-table-card {
  @apply bg-card border border-border rounded-lg shadow-sm;
  width: 220px;
  font-size: 12px;
  user-select: none;
}
.er-table-card--selected {
  @apply border-primary ring-1 ring-primary shadow-md;
}
.er-table-header {
  @apply bg-muted px-3 py-2 font-semibold text-sm border-b border-border;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}
.er-table-columns {
  @apply divide-y divide-border/50;
}
.er-table-column {
  @apply px-3 py-1.5 flex gap-1 items-center text-xs;
}
.er-column-name {
  @apply font-mono flex-1 truncate;
}
.er-column-type {
  @apply text-muted-foreground text-[10px] flex-shrink-0;
  max-width: 80px;
  overflow: hidden;
  text-overflow: ellipsis;
}
.er-column-markers {
  @apply flex gap-0.5 flex-shrink-0 w-4 items-center justify-center;
}
.er-pk {
  @apply text-amber-500 flex-shrink-0;
}
.er-fk {
  @apply text-blue-500 flex-shrink-0;
}
.er-col-spacer {
  @apply block w-3;
}
.er-expand-btn {
  @apply w-full text-xs text-muted-foreground hover:text-foreground py-1.5 border-t border-border bg-muted/30 hover:bg-muted/50 transition-colors;
}
</style>
