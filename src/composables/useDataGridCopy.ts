import type { CopyFormat } from '@/types/grid'
import { useI18n } from 'vue-i18n'
import { toast } from '@/composables/useNotifications'

function csvEscape(value: string): string {
  if (value.includes(',') || value.includes('"') || value.includes('\n')) {
    return `"${value.replace(/"/g, '""')}"`
  }
  return value
}

export function useDataGridCopy() {
  const { t } = useI18n()

  const formatValueForCopy = (value: unknown): string => {
    if (value === null || value === undefined)
      return 'NULL'
    if (typeof value === 'object')
      return JSON.stringify(value)
    return String(value)
  }

  const copyCellValue = async (value: unknown): Promise<void> => {
    const text = formatValueForCopy(value)
    await navigator.clipboard.writeText(text)
    toast.success(t('common.copied'))
  }

  const buildCsv = (rows: Record<string, unknown>[], columns: string[]): string => {
    const header = columns.map(c => csvEscape(c)).join(',')
    const data = rows.map(row =>
      columns.map(col => csvEscape(formatValueForCopy(row[col]))).join(','),
    )
    return [header, ...data].join('\n')
  }

  const buildJson = (rows: Record<string, unknown>[], columns: string[]): string => {
    const filtered = rows.map((row) => {
      const obj: Record<string, unknown> = {}
      for (const col of columns)
        obj[col] = row[col]
      return obj
    })
    return JSON.stringify(filtered, null, 2)
  }

  const buildInsertStatements = (
    rows: Record<string, unknown>[],
    columns: string[],
    tableName: string,
  ): string => {
    return rows
      .map((row) => {
        const values = columns.map((col) => {
          const v = row[col]
          if (v === null || v === undefined)
            return 'NULL'
          if (typeof v === 'number')
            return String(v)
          if (typeof v === 'boolean')
            return v ? 'TRUE' : 'FALSE'
          return `'${String(v).replace(/'/g, '\'\'')}'`
        })
        return `INSERT INTO ${tableName} (${columns.join(', ')}) VALUES (${values.join(', ')});`
      })
      .join('\n')
  }

  const copyRowsAs = async (
    rows: Record<string, unknown>[],
    columns: string[],
    format: CopyFormat,
    tableName?: string,
  ): Promise<void> => {
    let content: string
    switch (format) {
      case 'csv':
        content = buildCsv(rows, columns)
        break
      case 'json':
        content = buildJson(rows, columns)
        break
      case 'insert':
        content = buildInsertStatements(rows, columns, tableName ?? 'table')
        break
    }
    await navigator.clipboard.writeText(content)
    toast.success(t('components.dataGrid.export.copied'))
  }

  const exportToFile = async (
    rows: Record<string, unknown>[],
    columns: string[],
    format: CopyFormat,
    tableName?: string,
  ): Promise<void> => {
    let content: string
    let extension: string
    switch (format) {
      case 'csv':
        content = buildCsv(rows, columns)
        extension = 'csv'
        break
      case 'json':
        content = buildJson(rows, columns)
        extension = 'json'
        break
      case 'insert':
        content = buildInsertStatements(rows, columns, tableName ?? 'table')
        extension = 'sql'
        break
    }

    // Try Tauri save dialog, fallback to browser download
    try {
      const { save } = await import('@tauri-apps/plugin-dialog')
      const { invoke } = await import('@tauri-apps/api/core')
      const selectedPath = await save({
        filters: [{ name: `${format.toUpperCase()} Files`, extensions: [extension] }],
        defaultPath: `${tableName ?? 'export'}.${extension}`,
      })
      if (selectedPath) {
        await invoke('write_text_file', { path: selectedPath, content })
        toast.success(t('components.dataGrid.export.exported'))
        return
      }
    }
    catch {
      // Fall through to browser download
    }

    // Browser download fallback
    const blob = new Blob([content], { type: 'text/plain;charset=utf-8' })
    const url = URL.createObjectURL(blob)
    const a = document.createElement('a')
    a.href = url
    a.download = `${tableName ?? 'export'}.${extension}`
    document.body.appendChild(a)
    a.click()
    document.body.removeChild(a)
    URL.revokeObjectURL(url)
    toast.success(t('components.dataGrid.export.exported'))
  }

  return {
    copyCellValue,
    copyRowsAs,
    exportToFile,
    csvEscape,
    formatValueForCopy,
    buildCsv,
    buildJson,
    buildInsertStatements,
  }
}
