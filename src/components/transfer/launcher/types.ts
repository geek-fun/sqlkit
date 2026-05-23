export type LauncherAction = 'backup' | 'restore' | 'migrate' | 'export'

export type LauncherScope = 'server' | 'database' | 'table'

export type LauncherFormat = 'sql' | 'csv' | 'excel'

export type LauncherOptions = {
  format?: LauncherFormat
  destination?: string
  parallelism?: number
  filePath?: string
  fileFormat?: LauncherFormat
  targetTable?: string
  dropTargetFirst?: boolean
  useSourceNames?: boolean
  customPrefix?: string
}

export type LauncherSource = {
  connectionId?: string
  scope?: LauncherScope
  database?: string
  schema?: string
  tables?: string[]
}

export type LauncherTarget = {
  connectionId?: string
  database?: string
}

export type LauncherState = {
  action: LauncherAction
  source: LauncherSource
  target: LauncherTarget
  options: LauncherOptions
}
