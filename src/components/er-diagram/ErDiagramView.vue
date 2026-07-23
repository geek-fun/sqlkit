<script setup lang="ts">
import type { RenderEdge, RenderNode, TableData } from './types'
import type { ForeignKeyInfo } from '@/datasources/erDiagramApi'
import type { ColumnInfo } from '@/types/connection'
import { invoke } from '@tauri-apps/api/core'
import { useElementBounding, useResizeObserver } from '@vueuse/core'
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

import {
  computeCanvasSize,
  computeDagreLayout,
  fitToScreen as fitBboxToScreen,
} from './graph-layout'
import {
  buildTableRectMap,
  computeRelationshipPath,
} from './graph-routing'
import TableCard from './TableCard.vue'
import {
  calcNodeHeight,
  CARD_PADDING,
  HEADER_HEIGHT,
  NODE_WIDTH,
} from './types'

// ─── Props ───────────────────────────────────────────
const props = defineProps<{
  connectionId: string
  database: string
  schema?: string
}>()

const emit = defineEmits<{
  openTable: [tableName: string]
}>()

const { t } = useI18n()

// ─── Types (local) ───────────────────────────────────
type TableInfo = {
  name: string
  schema?: string
  table_type?: string
}

const CANVAS_PADDING = 80

// ─── State ────────────────────────────────────────────
const tables = ref<TableData[]>([])
const foreignKeys = ref<ForeignKeyInfo[]>([])
const dagrePositions = ref<Record<string, { x: number, y: number }>>({})
const manualOverrides = ref<Record<string, { x: number, y: number }>>({})

const selectedTableId = ref<string | null>(null)
const searchQuery = ref('')
const expandedTables = ref<Set<string>>(new Set())

const zoomLevel = ref(1)
const gestureStartZoom = ref(1)

const loading = ref(true)
const error = ref<string | null>(null)
const showWarning = ref(false)

// Viewport refs
const viewportRef = ref<HTMLElement | null>(null)
const viewportBounds = useElementBounding(viewportRef)

// ─── Node drag state ─────────────────────────────
const draggingNodeId = ref<string | null>(null)
const dragStartPos = ref({ x: 0, y: 0 })
const dragNodeStart = ref({ x: 0, y: 0 })

// ─── Drag to pan ────────────────────────────────
const isPanning = ref(false)
const panStart = ref({ mouseX: 0, mouseY: 0, scrollLeft: 0, scrollTop: 0 })

function onViewportMouseDown(e: MouseEvent) {
  // Only left-click triggers pan
  if (e.button !== 0)
    return

  // Don't start pan if clicking on a table card
  const target = e.target as HTMLElement
  if (target.closest('.er-table-wrapper'))
    return

  e.preventDefault()
  const vp = viewportRef.value
  if (!vp)
    return
  isPanning.value = true
  panStart.value = {
    mouseX: e.clientX,
    mouseY: e.clientY,
    scrollLeft: vp.scrollLeft,
    scrollTop: vp.scrollTop,
  }
}

function onViewportMouseMove(e: MouseEvent) {
  if (!isPanning.value)
    return
  const vp = viewportRef.value
  if (!vp)
    return
  const { mouseX, mouseY, scrollLeft, scrollTop } = panStart.value
  vp.scrollLeft = scrollLeft - (e.clientX - mouseX)
  vp.scrollTop = scrollTop - (e.clientY - mouseY)
}

function onViewportMouseUp() {
  isPanning.value = false
}

// ─── Schema selector ──────────────────────────────
const databaseStore = useDatabaseStore()
const availableSchemas = ref<string[]>([])
const localSchema = ref<string>(props.schema ?? '__all__')

const supportsSchemas = computed(() => availableSchemas.value.length > 0)

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

// ─── Combined positions (dagre + manual) ─────────
const nodePositions = computed(() => ({
  ...dagrePositions.value,
  ...manualOverrides.value,
}))

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

// ─── Render nodes ─────────────────────────────────
const renderNodes = computed<RenderNode[]>(() => {
  return displayedTables.value.map((table) => {
    const pos = nodePositions.value[table.name]
    if (!pos)
      return null

    const isExpanded = expandedTables.value.has(table.name)
    const hasMore = table.columns.length > 5
    const height = calcNodeHeight(table.columns.length, hasMore, isExpanded)
    const visibleCols = isExpanded
      ? table.columns
      : table.columns.slice(0, 5)

    return {
      id: table.name,
      x: pos.x - NODE_WIDTH / 2,
      y: pos.y - height / 2,
      width: NODE_WIDTH,
      height,
      table,
      isHighlighted: highlightedTables.value.has(table.name),
      visibleColumns: visibleCols,
      showExpandButton: hasMore,
      isExpanded,
    }
  }).filter((n): n is RenderNode => n !== null)
})

// ─── Canvas size ──────────────────────────────────
const nodeHeights = computed(() => {
  const map: Record<string, number> = {}
  for (const node of renderNodes.value)
    map[node.id] = node.height
  return map
})

const canvasSize = computed(() => {
  const positions = nodePositions.value
  if (Object.keys(positions).length === 0)
    return { width: 800, height: 600 }

  return computeCanvasSize(positions, NODE_WIDTH, (name) => {
    const h = nodeHeights.value[name]
    return h ?? HEADER_HEIGHT + 8
  }, CANVAS_PADDING)
})

// ─── Render edges (orthogonal routing) ────────────
const tableRectMap = computed(() => {
  return buildTableRectMap(
    displayedTables.value.map(t => t.name),
    nodePositions.value,
    (name) => {
      const h = nodeHeights.value[name]
      return h ?? HEADER_HEIGHT + CARD_PADDING
    },
  )
})

const renderEdges = computed<RenderEdge[]>(() => {
  return displayedRelationships.value.map((rel) => {
    const path = computeRelationshipPath(
      rel.sourceTable,
      rel.targetTable,
      tableRectMap.value,
    )
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

// ─── Layout Computation (dagre only, does NOT touch manualOverrides) ─
function computeLayout() {
  if (displayedTables.value.length === 0) {
    dagrePositions.value = {}
    return
  }

  const dagreTables = displayedTables.value.map((table) => {
    const isExpanded = expandedTables.value.has(table.name)
    const hasMore = table.columns.length > 5
    const h = calcNodeHeight(table.columns.length, hasMore, isExpanded)
    return { name: table.name, width: NODE_WIDTH, height: h }
  })

  const dagreRels = displayedRelationships.value.map(rel => ({
    sourceTable: rel.sourceTable,
    targetTable: rel.targetTable,
  }))

  dagrePositions.value = computeDagreLayout(dagreTables, dagreRels)
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
// Only recompute layout when column visibility (expand) changes — NOT on search/focus (displayedTables)
watch(expandedTables, () => {
  computeLayout()
})

watch(localSchema, () => {
  fetchSchemaData()
})

// ─── Interaction Handlers ─────────────────────────────
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

// ─── Header-only drag handlers ───────────────────
function onHeaderMousedown(e: MouseEvent, nodeId: string) {
  if (e.button !== 0)
    return
  const pos = nodePositions.value[nodeId]
  if (!pos)
    return
  draggingNodeId.value = nodeId
  dragStartPos.value = { x: e.clientX, y: e.clientY }
  dragNodeStart.value = { x: pos.x, y: pos.y }
  document.addEventListener('mousemove', onHeaderMouseMove)
  document.addEventListener('mouseup', onHeaderMouseUp)
}

function onHeaderMouseMove(e: MouseEvent) {
  if (!draggingNodeId.value)
    return
  const dx = (e.clientX - dragStartPos.value.x) / zoomLevel.value
  const dy = (e.clientY - dragStartPos.value.y) / zoomLevel.value
  manualOverrides.value = {
    ...manualOverrides.value,
    [draggingNodeId.value]: {
      x: Math.max(16, dragNodeStart.value.x + dx),
      y: Math.max(16, dragNodeStart.value.y + dy),
    },
  }
}

function onHeaderMouseUp() {
  draggingNodeId.value = null
  document.removeEventListener('mousemove', onHeaderMouseMove)
  document.removeEventListener('mouseup', onHeaderMouseUp)
}

// ─── Zoom ─────────────────────────────────────────────
function onWheel(e: WheelEvent) {
  if (!e.ctrlKey && !e.metaKey)
    return // native scroll
  e.preventDefault()
  const delta = -e.deltaY / 500
  const nextZoom = Math.max(0.1, Math.min(3, +(zoomLevel.value * (1 + delta)).toFixed(2)))

  // Zoom centered on mouse position relative to content
  const vp = viewportRef.value
  if (vp) {
    const vpRect = vp.getBoundingClientRect()
    const originX = e.clientX - vpRect.left
    const originY = e.clientY - vpRect.top
    const contentX = (vp.scrollLeft + originX) / zoomLevel.value
    const contentY = (vp.scrollTop + originY) / zoomLevel.value
    zoomLevel.value = nextZoom
    vp.scrollLeft = contentX * nextZoom - originX
    vp.scrollTop = contentY * nextZoom - originY
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
  if (!viewportRef.value)
    return
  const vpW = viewportBounds.width.value || 800
  const vpH = viewportBounds.height.value || 600
  const result = fitBboxToScreen(canvasSize.value, vpW, vpH)
  zoomLevel.value = result.zoom
  viewportRef.value.scrollLeft = result.scrollX
  viewportRef.value.scrollTop = result.scrollY
}

function resetLayout() {
  manualOverrides.value = {}
  computeLayout()
  fitToScreen()
}

// ─── Resize Observer ──────────────────────────────────
useResizeObserver(viewportRef, () => {
  // Viewport size changes may affect fit-to-screen but we don't auto-fit
})

// ─── Lifecycle ────────────────────────────────────────
onMounted(async () => {
  await fetchAvailableSchemas()
  fetchSchemaData()
})

onUnmounted(() => {
  document.removeEventListener('mousemove', onHeaderMouseMove)
  document.removeEventListener('mouseup', onHeaderMouseUp)
})
</script>

<template>
  <div class="er-diagram-view bg-background flex flex-col h-full">
    <!-- ── Toolbar ──────────────────────────────────── -->
    <div class="px-3 py-2 border-b bg-muted/30 flex shrink-0 gap-2 items-center">
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
        <svg xmlns="http://www.w3.org/2000/svg" width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" class="text-muted-foreground left-2 top-1/2 absolute -translate-y-1/2"><circle cx="11" cy="11" r="8" /><path d="m21 21-4.3-4.3" /></svg>
        <Input v-model="searchQuery" :placeholder="t('components.databaseBrowser.erDiagram.searchPlaceholder')" class="text-xs pl-7 h-8" />
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

      <!-- Reset Layout -->
      <Button
        variant="ghost" size="icon" class="h-7 w-7"
        :title="t('components.databaseBrowser.erDiagram.resetLayout')"
        @click="resetLayout"
      >
        <svg xmlns="http://www.w3.org/2000/svg" width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><polyline points="1 4 1 10 7 10" /><path d="M3.51 15a9 9 0 1 0 2.13-9.36L1 10" /></svg>
      </Button>

      <!-- Fit to screen -->
      <Button variant="ghost" size="icon" class="h-7 w-7" :title="t('components.databaseBrowser.erDiagram.fitToScreen')" @click="fitToScreen">
        <svg xmlns="http://www.w3.org/2000/svg" width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M8 3H5a2 2 0 0 0-2 2v3" /><path d="M21 8V5a2 2 0 0 0-2-2h-3" /><path d="M16 21h3a2 2 0 0 0 2-2v-3" /><path d="M3 16v3a2 2 0 0 0 2 2h3" /></svg>
      </Button>

      <!-- Refresh -->
      <Button variant="ghost" size="icon" class="h-7 w-7" :title="t('common.buttons.refresh')" :disabled="loading" @click="fetchSchemaData">
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
          <AlertDialogTitle>{{ t('components.databaseBrowser.erDiagram.largeSchemaWarning', { count: tables.length }) }}</AlertDialogTitle>
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

    <!-- ── Canvas Area (pure HTML, no foreignObject) ── -->
    <div
      ref="viewportRef"
      class="bg-muted/5 flex-1 relative overflow-auto"
      :class="{ 'cursor-grab': !isPanning && !draggingNodeId, 'cursor-grabbing': isPanning || !!draggingNodeId }"
      @wheel.prevent="onWheel"
      @gesturestart="onGestureStart"
      @gesturechange="onGestureChange"
      @mousedown="onViewportMouseDown"
      @mousemove="onViewportMouseMove"
      @mouseup="onViewportMouseUp"
      @mouseleave="onViewportMouseUp"
      @click.self="deselectAll"
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

      <!-- Content area (scaled) -->
      <div
        v-show="!loading && displayedTables.length > 0"
        class="relative"
        :style="{
          width: `${canvasSize.width * zoomLevel}px`,
          height: `${canvasSize.height * zoomLevel}px`,
        }"
        @mousedown="onViewportMouseDown"
      >
        <div
          class="origin-top-left left-0 top-0 absolute"
          :style="{
            width: `${canvasSize.width}px`,
            height: `${canvasSize.height}px`,
            transform: `scale(${zoomLevel})`,
          }"
          @mousedown="onViewportMouseDown"
          @click.self="deselectAll"
        >
          <!-- SVG overlay for relationship lines -->
          <svg class="h-full w-full pointer-events-none inset-0 absolute overflow-visible" style="z-index: 0;">
            <defs>
              <marker id="er-arrowhead" markerWidth="10" markerHeight="7" refX="9" refY="3.5" orient="auto" markerUnits="strokeWidth">
                <polygon points="0 0, 10 3.5, 0 7" fill="hsl(var(--muted-foreground))" />
              </marker>
            </defs>
            <path
              v-for="(edge, idx) in renderEdges"
              :key="`edge-${idx}`"
              :d="edge.path"
              fill="none"
              :stroke="edge.isHighlighted ? 'hsl(var(--primary))' : 'hsl(var(--muted-foreground))'"
              :stroke-width="edge.isHighlighted ? 2 : 1"
              marker-end="url(#er-arrowhead)"
            />
          </svg>

          <!-- Table cards -->
          <div
            v-for="node in renderNodes"
            :key="node.id"
            class="er-table-wrapper"
            :style="{
              position: 'absolute',
              left: 0,
              top: 0,
              transform: `translate(${node.x}px, ${node.y}px)`,
              willChange: draggingNodeId === node.id ? 'transform' : 'auto',
            }"
          >
            <TableCard
              :node="node"
              :is-selected="selectedTableId === node.id"
              :is-highlighted="node.isHighlighted"
              :header-dragging="draggingNodeId === node.id"
              @header-mousedown="onHeaderMousedown($event, node.id)"
              @card-dblclick="emit('openTable', node.id)"
              @card-click="selectTable(node.id)"
              @toggle-expand="toggleExpand(node.id)"
            />
          </div>
        </div>
      </div>
    </div>
  </div>
</template>

<style scoped>
/* Table view styles are in TableCard.vue */
</style>
