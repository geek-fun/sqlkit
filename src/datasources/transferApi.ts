import type {
  DdlRequest,
  ExportPreview,
  ExportRequest,
  FileDetectionResult,
  ImportFormat,
  ImportRequest,
  MigrationMapping,
  MigrationPreview,
  MigrationRequest,
  TransferResult,
} from '@/types/transfer'

import { invoke } from '@tauri-apps/api/core'

export function previewExport(request: ExportRequest, previewRows = 10) {
  return invoke<ExportPreview>('preview_export_data', { request, previewRows })
}

export function executeExport(request: ExportRequest) {
  return invoke<TransferResult>('execute_export_data', { request })
}

export function detectFile(filePath: string) {
  return invoke<FileDetectionResult>('detect_file_format', { filePath })
}

export function previewImport(filePath: string, format: ImportFormat, previewRows = 10) {
  return invoke<ExportPreview>('preview_import_data', { filePath, format, previewRows })
}

export function executeImport(request: ImportRequest) {
  return invoke<TransferResult>('execute_import_data', { request })
}

export function previewMigration(request: MigrationRequest) {
  return invoke<MigrationPreview>('preview_migration_data', { request })
}

export function executeMigration(request: MigrationRequest) {
  return invoke<TransferResult>('execute_migration_data', { request })
}

type AutoMapParams = {
  connectionId: string
  database?: string
  schema?: string
  table: string
  targetEngine: string
}

export function autoMapMigrationColumns(params: AutoMapParams) {
  return invoke<MigrationMapping[]>('auto_map_migration_columns', params)
}

export function generateDdl(request: DdlRequest) {
  return invoke<string>('generate_ddl_for_objects', { request })
}
