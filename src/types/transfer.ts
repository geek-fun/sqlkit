export type ExportFormat = 'csv' | 'jsonl' | 'sql' | 'excel'

export type ExportSource = {
  table: string
  columns: string[]
  whereClause?: string
  orderBy?: string
  limit?: number
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
  source: ExportSource
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

export type ImportRequest = {
  connectionId: string
  database?: string
  schema?: string
  table: string
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

export type TaskKind = 'export' | 'import' | 'sqlFile' | 'migration'

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
  table: string
  columns: string[]
  whereClause?: string
  orderBy?: string
  limit?: number
  format: ExportFormat
  outputPath: string
}

export type ImportTaskConfig = {
  connectionId: string
  database?: string
  schema?: string
  table: string
  filePath: string
  format: ImportFormat
  conflictStrategy?: ConflictStrategy
}

export type SqlFileTaskConfig = {
  connectionId: string
  database?: string
  filePath: string
  onError: 'rollback' | 'skipAndContinue' | 'stop'
}

export type MigrationTaskConfig = {
  sourceConnectionId: string
  sourceDatabase?: string
  targetConnectionId: string
  targetDatabase?: string
  tables: string[]
}

export type TaskConfig = ExportTaskConfig | ImportTaskConfig | SqlFileTaskConfig | MigrationTaskConfig

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

export type DdlObjectType = 'table' | 'view' | 'index'

export type DdlObject = {
  name: string
  objectType: DdlObjectType
  schema?: string
}

export type DdlOptions = {
  includeCreateTable?: boolean
  includePrimaryKeys?: boolean
  includeForeignKeys?: boolean
  includeIndexes?: boolean
  includeConstraints?: boolean
  includeComments?: boolean
  includeStorageOptions?: boolean
  includeDropIfExists?: boolean
  includeIfNotExists?: boolean
  includeData?: boolean
  targetEngine?: string
}

export type DdlRequest = {
  connectionId: string
  database?: string
  schema?: string
  objects: DdlObject[]
  options: DdlOptions
}

export type ExcelImportOptions = {
  sheetName?: string
  hasHeader?: boolean
}

export type MigrationConversion = 'direct' | 'mapped' | 'custom'

export type MigrationMapping = {
  sourceColumn: string
  sourceType: string
  targetColumn: string
  targetType: string
  conversion: MigrationConversion
}

export type MigrationTablePlan = {
  sourceTable: string
  targetTable: string
  columnMappings: MigrationMapping[]
}

export type MigrationErrorStrategy = 'skipRow' | 'skipTable' | 'abort'

export type MigrationRequest = {
  sourceConnectionId: string
  sourceDatabase?: string
  sourceSchema?: string
  targetConnectionId: string
  targetDatabase?: string
  targetSchema?: string
  tablePlans: MigrationTablePlan[]
  batchSize?: number
  onError?: MigrationErrorStrategy
  createTables?: boolean
  dropTables?: boolean
  migrateIndexes?: boolean
  migrateForeignKeys?: boolean
  migrateConstraints?: boolean
  disableFkChecks?: boolean
}

export type MigrationTablePreview = {
  sourceTable: string
  targetTable: string
  rowCount: number
  columnCount: number
  mappings: MigrationMapping[]
}

export type MigrationPreview = {
  tables: MigrationTablePreview[]
  totalRows: number
  typeConversions: number
}
