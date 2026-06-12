export type ExportFormat = 'csv' | 'jsonl' | 'sql' | 'excel'

export type TransferScope = 'server' | 'database' | 'tables'

export type ColumnInfo = {
  name: string
  data_type?: string
  nullable?: boolean
  default_value?: string
  is_primary_key?: boolean
}

export type TableColumns = {
  tableName: string
  columns: ColumnInfo[]
  selectedColumns: string[]
}

export type ExportSource = {
  table: string
  columns: string[]
}

export type CsvExportOptions = {
  delimiter?: string
  quoteChar?: string
  encoding?: string
  includeHeader?: boolean
  quoteAll?: boolean
  lineEnding?: 'LF' | 'CRLF'
}

export type JsonlExportOptions = {
  dateFormat?: string
}

export type SqlExportOptions = {
  targetTable: string
  batchSize?: number
  includeCreateTable?: boolean
  includeDropTable?: boolean
  targetEngine?: string
}

export type ExcelExportOptions = {
  sheetName?: string
  includeHeader?: boolean
  autoFitColumns?: boolean
  freezeHeader?: boolean
}

export type ExportRequest = {
  connectionId: string
  database?: string
  schema?: string
  scope: TransferScope
  sources: ExportSource[]
  format: ExportFormat
  csvOptions?: CsvExportOptions
  jsonlOptions?: JsonlExportOptions
  sqlOptions?: SqlExportOptions
  excelOptions?: ExcelExportOptions
  outputPath: string
}

export type ImportFormat = 'csv' | 'jsonl' | 'sql' | 'excel'

export type ColumnMapping = {
  sourceColumn: string
  targetColumn?: string
  targetType?: string
}

export type ConflictStrategy = 'skip' | 'replace' | 'upsert' | 'abort'

export type CsvImportOptions = {
  delimiter?: string
  encoding?: string
  hasHeader?: boolean
}

export type ImportTarget = {
  sourceTable?: string
  targetTable: string
  columnMappings?: ColumnMapping[]
}

export type ImportRequest = {
  connectionId: string
  database?: string
  schema?: string
  scope: TransferScope
  createDatabaseIfNotExists?: boolean
  tables: ImportTarget[]
  filePath: string
  format: ImportFormat
  columnMappings: ColumnMapping[]
  conflictStrategy?: ConflictStrategy
  batchSize?: number
  createTable?: boolean
  truncateBefore?: boolean
  dryRun?: boolean
  csvOptions?: CsvImportOptions
  excelOptions?: ExcelImportOptions
}

export type TransferProgress = {
  operation: string
  phase: string
  currentTable?: string
  totalRows?: number
  processedRows: number
  skippedRows: number
  errorCount: number
  percent: number
  elapsedMs: number
  estimatedRemainingMs?: number
  message?: string
}

export type TransferError = {
  rowNumber?: number
  statementNumber?: number
  message: string
  sql?: string
}

export type TransferResult = {
  success: boolean
  totalRows: number
  processedRows: number
  skippedRows: number
  errorCount: number
  durationMs: number
  outputPath?: string
  outputSizeBytes?: number
  errors: TransferError[]
}

export type FileDetectionResult = {
  format: ImportFormat
  encoding: string
  estimatedRows?: number
  fileSizeBytes: number
  columns: string[]
  csvDelimiter?: string
  hasHeader?: boolean
}

export type ExportPreview = {
  columns: string[]
  sampleRows: string[][]
  totalRowsEstimate?: number
  formattedPreview: string
}

export type TaskKind = 'export' | 'import'

export type TaskStatus = 'pending' | 'running' | 'completed' | 'failed'

export type TaskRuntime = {
  complete: number
  total: number
  skipped: number
  errorCount: number
}

export type ExportTaskConfig = {
  connectionId: string
  database?: string
  schema?: string
  scope: TransferScope
  sources: ExportSource[]
  format: ExportFormat
  outputPath: string
}

export type ImportTaskConfig = {
  connectionId: string
  database?: string
  schema?: string
  scope: TransferScope
  tables: ImportTarget[]
  filePath: string
  format: ImportFormat
  conflictStrategy?: ConflictStrategy
  createDatabaseIfNotExists?: boolean
}

export type TaskConfig = ExportTaskConfig | ImportTaskConfig

export type BackgroundTask = {
  id: string
  kind: TaskKind
  status: TaskStatus
  progress: { complete: number, total: number }
  config: TaskConfig
  runtime: TaskRuntime
  label: string
  startTime: Date
  endTime?: Date
  error?: string
}

export type ExcelImportOptions = {
  sheetName?: string
  hasHeader?: boolean
}
