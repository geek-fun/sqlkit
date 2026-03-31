export function formatTableValue(v: unknown): string {
  if (v === null || v === undefined)
    return 'NULL'
  if (typeof v === 'object')
    return JSON.stringify(v)
  return String(v)
}

export const isTableNullValue = (v: unknown): boolean => v === null || v === undefined

export function rowsToCsv(rows: Record<string, unknown>[], columns: string[]): string {
  const escape = (s: string) => `"${s.replace(/"/g, '""')}"`
  const header = columns.map(escape).join(',')
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

export function computeTotalPages(totalCount: number, rowsPerPage: number): number {
  return totalCount > 0 ? Math.ceil(totalCount / rowsPerPage) : 1
}

export function computeOffset(page: number, rowsPerPage: number): number {
  return (page - 1) * rowsPerPage
}
