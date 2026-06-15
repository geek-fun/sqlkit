// ── Sort Types ──

export type SortDirection = 'ASC' | 'DESC'

export type SortColumn = {
  column: string
  direction: SortDirection
}

export type SortState = SortColumn[]

// ── Filter Types ──

export type FilterOperator
  = | 'eq'
    | 'neq'
    | 'like'
    | 'gt'
    | 'lt'
    | 'gte'
    | 'lte'
    | 'between'

export type ColumnFilter = {
  column: string
  operator: FilterOperator
  value: string
  value2?: string
}

export type FilterState = ColumnFilter[]

// ── Copy/Export ──

export type CopyFormat = 'csv' | 'json' | 'insert'

// ── Column Type Info ──

export type ColumnTypeMap = Record<string, string>

// ── Grid Component Events ──

export type DataGridEmits = {
  (e: 'sortChange', state: SortColumn[]): void
  (e: 'filterChange', state: ColumnFilter[]): void
  (e: 'refresh'): void
}

// ── Filter Bar ──

export type FilterBarEmits = {
  (e: 'removeFilter', column: string): void
  (e: 'clearAll'): void
}

// ── Cell Context Menu ──

export type CellContextMenuEmits = {
  (e: 'close'): void
  (e: 'filter', filter: ColumnFilter): void
}

// ── Column Header Context Menu ──

export type ColumnHeaderContextMenuEmits = {
  (e: 'close'): void
  (e: 'sort', column: string, direction: SortDirection): void
  (e: 'clearSort'): void
  (e: 'filter', filter: ColumnFilter): void
  (e: 'clearFilter', column: string): void
}

// ── Edit Row Dialog ──

export type EditRowDialogEmits = {
  (e: 'update:open', open: boolean): void
  (e: 'saved'): void
  (e: 'close'): void
}

// ── Batch Action Bar ──

export type BatchActionBarEmits = {
  (e: 'editSelected'): void
  (e: 'deleteSelected'): void
  (e: 'exportSelected', format: CopyFormat): void
}
