/**
 * Pure utility functions for the DataTableView component.
 * Extracted for testability.
 */

/**
 * Format a cell value for display. NULL values are shown as 'NULL'.
 * Object values are JSON-stringified. Other values are coerced to strings.
 */
export function formatTableValue(v: unknown): string {
  if (v === null || v === undefined)
    return 'NULL'
  if (typeof v === 'object')
    return JSON.stringify(v)
  return String(v)
}

/**
 * Returns true if the value is null or undefined (renders as NULL in the table).
 */
export function isTableNullValue(v: unknown): boolean {
  return v === null || v === undefined
}

/**
 * Serialize visible table rows to a CSV string.
 * Columns and string values are double-quoted; embedded quotes are doubled.
 * NULL values are serialized as empty cells.
 */
export function rowsToCsv(rows: Record<string, unknown>[], columns: string[]): string {
  const escape = (s: string) => `"${s.replace(/"/g, '""')}"`
  const header = columns.map(c => escape(c)).join(',')
  const lines = rows.map(row =>
    columns.map((c) => {
      const v = row[c]
      if (v === null || v === undefined)
        return ''
      const s = typeof v === 'object' ? JSON.stringify(v) : String(v)
      return escape(s)
    }).join(','),
  )
  return [header, ...lines].join('\n')
}

/**
 * Compute the total number of pages given a total row count and page size.
 */
export function computeTotalPages(totalCount: number, rowsPerPage: number): number {
  return totalCount > 0 ? Math.ceil(totalCount / rowsPerPage) : 1
}

/**
 * Compute the row offset for a given page (0-indexed).
 */
export function computeOffset(page: number, rowsPerPage: number): number {
  return (page - 1) * rowsPerPage
}
