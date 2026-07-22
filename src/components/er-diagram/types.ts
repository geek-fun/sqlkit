import type { ForeignKeyInfo } from '@/datasources/erDiagramApi'
import type { ColumnInfo } from '@/types/connection'

export type TableInfo = {
  name: string
  schema?: string
  table_type?: string
}

export type TableData = {
  name: string
  schema?: string
  columns: ColumnInfo[]
  foreignKeys: ForeignKeyInfo[]
}

export type RenderNode = {
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

export type RenderEdge = {
  from: string
  to: string
  label: string
  path: string
  isHighlighted: boolean
}

export type TableRect = {
  name: string
  x: number
  y: number
  width: number
  height: number
}

// Layout constants
export const NODE_WIDTH = 220
export const COL_HEIGHT = 28
export const HEADER_HEIGHT = 36
export const EXPAND_BTN_HEIGHT = 30
export const CARD_PADDING = 8
export const ROUTE_PADDING = 56
export const ROUTE_BLOCK_MARGIN = 18

export function calcNodeHeight(columnsCount: number, hasMore: boolean, isExpanded: boolean): number {
  const colCount = isExpanded ? columnsCount : Math.min(5, columnsCount)
  const expandBtn = hasMore ? EXPAND_BTN_HEIGHT : 0
  return HEADER_HEIGHT + colCount * COL_HEIGHT + expandBtn + CARD_PADDING
}
