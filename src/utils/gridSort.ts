import type { ColumnTypeMap, SortState } from '@/types/grid'

// ---------------------------------------------------------------------------
// Primitive-only client-side sorting for returned grid rows.
//
// Rows are sorted in the frontend, so only values that have a total ordering
// can participate: strings, numbers, booleans, dates and NULL. JSON documents,
// arrays, BLOBs and other object-shaped values are excluded, both by declared
// column type and by inspecting the actual cell values.
// ---------------------------------------------------------------------------

const NON_PRIMITIVE_TYPE_PATTERN
  = /^(?:JSONB?|ARRAY|BYTEA|BLOB|BINARY|VARBINARY|IMAGE|GEOMETRY|GEOGRAPHY|HSTORE|XML)/i

const NUMERIC_TYPE_PATTERN
  = /^(?:INT|BIGINT|SMALLINT|TINYINT|MEDIUMINT|SERIAL|BIGSERIAL|FLOAT|DOUBLE|REAL|DECIMAL|NUMERIC|MONEY|NUMBER)/i

const BOOLEAN_TYPE_PATTERN = /^(?:BOOL|BIT)/i

const DATE_TYPE_PATTERN = /^(?:DATE|TIMESTAMP|TIME)/i

/** SQL types whose values have no meaningful client-side ordering. */
function isNonPrimitiveColumnType(columnType?: string): boolean {
  return columnType !== undefined && NON_PRIMITIVE_TYPE_PATTERN.test(columnType.trim())
}

/** A value that can be compared client-side (NULL included). */
export function isPrimitiveValue(value: unknown): boolean {
  return value === null || value === undefined || typeof value !== 'object'
}

/**
 * A column is sortable when its declared type is primitive and every returned
 * cell is primitive. A single JSON/array cell disqualifies the whole column.
 */
export function isSortableColumn(
  column: string,
  rows: readonly Record<string, unknown>[],
  columnTypes: ColumnTypeMap = {},
): boolean {
  if (isNonPrimitiveColumnType(columnTypes[column]))
    return false
  return rows.every(row => isPrimitiveValue(row[column]))
}

/** Columns from `columns` that can be sorted client-side. */
export function collectSortableColumns(
  columns: readonly string[],
  rows: readonly Record<string, unknown>[],
  columnTypes: ColumnTypeMap = {},
): Set<string> {
  return new Set(columns.filter(column => isSortableColumn(column, rows, columnTypes)))
}

type SortValueKind = 'boolean' | 'number' | 'date' | 'string'

/**
 * Fixed ordering across kinds, so a column holding mixed primitive types
 * (SQLite's dynamic typing makes this routine) still yields ONE comparator.
 * The kind is resolved per column, never per cell — per-cell resolution
 * breaks comparator symmetry and flips order between passes.
 */
const KIND_PRECEDENCE: Record<SortValueKind, number> = { number: 0, boolean: 1, date: 2, string: 3 }

/** Kind fixed by the declared column type, if it names one. */
function resolveDeclaredKind(columnType?: string): SortValueKind | null {
  const type = (columnType ?? '').trim()
  if (BOOLEAN_TYPE_PATTERN.test(type))
    return 'boolean'
  if (NUMERIC_TYPE_PATTERN.test(type))
    return 'number'
  if (DATE_TYPE_PATTERN.test(type))
    return 'date'
  return null
}

/** Kind of a value when no usable type is declared; strings are never date-sniffed. */
function resolveValueKind(value: unknown): SortValueKind {
  if (typeof value === 'boolean')
    return 'boolean'
  if (typeof value === 'number')
    return 'number'
  return 'string'
}

function kindFromToken(kindOrType?: string): SortValueKind {
  if (kindOrType && (kindOrType === 'boolean' || kindOrType === 'number' || kindOrType === 'date' || kindOrType === 'string'))
    return kindOrType
  return resolveDeclaredKind(kindOrType) ?? 'string'
}

/** One kind for the whole column: declared type wins, otherwise the highest-precedence kind present. */
function resolveColumnKind(
  rows: readonly Record<string, unknown>[],
  column: string,
  columnType?: string,
): SortValueKind {
  const declared = resolveDeclaredKind(columnType)
  if (declared)
    return declared
  let best = 'string' as SortValueKind
  for (const row of rows) {
    const value = row[column]
    if (value === null || value === undefined || typeof value === 'object')
      continue
    const kind = resolveValueKind(value)
    if (KIND_PRECEDENCE[kind] < KIND_PRECEDENCE[best]) {
      best = kind
      if (best === 'number')
        break
    }
  }
  return best
}

function toBoolean(value: unknown): boolean {
  return value === true
    || value === 1
    || ['true', 't', '1', 'y', 'yes'].includes(String(value).toLowerCase())
}

function compareStrings(a: unknown, b: unknown): number {
  return String(a).localeCompare(String(b), undefined, { numeric: true, sensitivity: 'base' })
}

function compareNumbers(a: unknown, b: unknown): number {
  const na = Number(a)
  const nb = Number(b)
  const aValid = !Number.isNaN(na)
  const bValid = !Number.isNaN(nb)
  if (aValid && bValid)
    return na === nb ? 0 : na < nb ? -1 : 1
  if (aValid)
    return -1
  if (bValid)
    return 1
  return compareStrings(a, b)
}

function compareDates(a: unknown, b: unknown): number {
  const ta = Date.parse(String(a))
  const tb = Date.parse(String(b))
  if (!Number.isNaN(ta) && !Number.isNaN(tb))
    return ta === tb ? 0 : ta < tb ? -1 : 1
  return compareStrings(a, b)
}

/**
 * Ascending comparison for two primitive cell values.
 * NULL/undefined always sort after real values in ascending order.
 */
export function compareSortValues(a: unknown, b: unknown, kindOrType?: string): number {
  const aNull = a === null || a === undefined
  const bNull = b === null || b === undefined
  if (aNull && bNull)
    return 0
  if (aNull)
    return 1
  if (bNull)
    return -1

  switch (kindFromToken(kindOrType)) {
    case 'boolean':
      return (toBoolean(a) ? 1 : 0) - (toBoolean(b) ? 1 : 0)
    case 'number':
      return compareNumbers(a, b)
    case 'date':
      return compareDates(a, b)
    default:
      return compareStrings(a, b)
  }
}

/**
 * Return a sorted copy of `rows`, applying every rule in `sortState` in order.
 * The input array is never mutated and equal rows keep their original order.
 */
export function sortRowsByState<T extends Record<string, unknown>>(
  rows: readonly T[],
  sortState: SortState,
  columnTypes: ColumnTypeMap = {},
): T[] {
  if (rows.length === 0 || sortState.length === 0)
    return [...rows]

  const kinds = new Map(sortState.map(rule => [
    rule.column,
    resolveColumnKind(rows, rule.column, columnTypes[rule.column]),
  ]))

  return rows
    .map((row, index) => ({ row, index }))
    .sort((a, b) => {
      for (const rule of sortState) {
        const result = compareSortValues(a.row[rule.column], b.row[rule.column], kinds.get(rule.column))
        if (result !== 0)
          return rule.direction === 'DESC' ? -result : result
      }
      return a.index - b.index
    })
    .map(entry => entry.row)
}

/** Human-readable summary of active sort rules, e.g. `id ↑, name ↓`. */
export function formatSortState(sortState: SortState): string {
  return sortState.map(rule => `${rule.column} ${rule.direction === 'ASC' ? '↑' : '↓'}`).join(', ')
}
